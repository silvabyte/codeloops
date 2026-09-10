use base64::{Engine, engine::general_purpose::STANDARD as B64};
use rmcp::{ServiceExt, model::CallToolRequestParams, transport::TokioChildProcess};
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    net::TcpListener,
    path::{Path, PathBuf},
    process::{Child, Command, Output, Stdio},
    time::Duration,
};

struct Service(Child);
impl Drop for Service {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

struct Fixture {
    temporary: tempfile::TempDir,
    prefix: PathBuf,
    home: PathBuf,
    data: PathBuf,
    address: String,
}

fn checked(output: Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

impl Fixture {
    fn new() -> Self {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path();
        let prefix = root.join("preview install 'quoted'");
        let home = root.join("home");
        let data = root.join("data 'isolated'");
        fs::create_dir_all(home.join(".config/opencode")).unwrap();
        fs::create_dir_all(home.join(".cursor")).unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let fixture = Self {
            temporary,
            prefix,
            home,
            data,
            address,
        };
        let source = std::env::var_os("CODELOOPS_E2E_BINARY")
            .map(PathBuf::from)
            .unwrap_or_else(|| env!("CARGO_BIN_EXE_codeloops").into());
        checked(
            Command::new(source)
                .args(["install", "--prefix"])
                .arg(&fixture.prefix)
                .output()
                .unwrap(),
        );
        fixture
    }

    fn binary(&self) -> PathBuf {
        self.prefix.join("bin/codeloops")
    }

    fn command(&self) -> Command {
        let mut command = Command::new(self.binary());
        command
            .env("HOME", &self.home)
            .env("XDG_CONFIG_HOME", self.home.join(".config"))
            .env("XDG_DATA_HOME", self.home.join(".local/share"))
            .env_remove("CODELOOPS_DATA_DIR")
            .env_remove("CODELOOPS_ADDRESS")
            .env_remove("CODELOOPS_OPENCODE_VERSION")
            .arg("--json");
        command
    }

    fn setup(&self) -> Output {
        self.command()
            .arg("--data-dir")
            .arg(&self.data)
            .args([
                "--address",
                &self.address,
                "setup",
                "--opencode-version",
                "fixture",
            ])
            .output()
            .unwrap()
    }

    fn run(&self, args: &[&str], input: Option<&Value>) -> Value {
        let mut child = self
            .command()
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        if let Some(input) = input {
            child
                .stdin
                .take()
                .unwrap()
                .write_all(&serde_json::to_vec(input).unwrap())
                .unwrap();
        }
        checked(child.wait_with_output().unwrap())
    }

    fn hook(&self, event: &Value) {
        let hooks: Value =
            serde_json::from_slice(&fs::read(self.home.join(".cursor/hooks.json")).unwrap())
                .unwrap();
        let command = hooks["hooks"][event["hook_event_name"].as_str().unwrap()]
            .as_array()
            .unwrap()
            .iter()
            .find(|hook| hook["command"].as_str().unwrap().contains("capture-cursor"))
            .unwrap();
        let mut child = Command::new("sh")
            .args(["-c", command["command"].as_str().unwrap()])
            .env_remove("CODELOOPS_DATA_DIR")
            .env_remove("CODELOOPS_ADDRESS")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(&serde_json::to_vec(event).unwrap())
            .unwrap();
        assert_eq!(checked(child.wait_with_output().unwrap()), json!({}));
    }

    async fn start(&self) -> Service {
        let service = Service(
            self.command()
                .arg("serve")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        );
        for _ in 0..200 {
            let output = self.command().arg("health").output().unwrap();
            if output.status.success() {
                return service;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        panic!("installed service failed to start");
    }
}

fn git(repo: &Path, args: &[&str]) {
    let mut command = Command::new("git");
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("GIT_") {
            command.env_remove(key);
        }
    }
    let output = command
        .current_dir(repo)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_AUTHOR_NAME", "Fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.com")
        .env("GIT_COMMITTER_NAME", "Fixture")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.com")
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn records(bundle: &Path, kind: &str) -> Vec<Value> {
    let manifest: Value =
        serde_json::from_slice(&fs::read(bundle.join("manifest.json")).unwrap()).unwrap();
    manifest["record_pages"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|page| page["kind"] == kind)
        .flat_map(|page| {
            let bytes = fs::read(
                bundle
                    .join("artifacts")
                    .join(page["hash"].as_str().unwrap()),
            )
            .unwrap();
            let page: Value = serde_json::from_slice(&bytes).unwrap();
            page["items"].as_array().unwrap().clone()
        })
        .collect()
}

fn captured_file(bundle: &Path, checkpoint: &Value, layer: &str) -> Vec<u8> {
    let root = checkpoint["manifests"][layer].as_str().unwrap();
    let manifest: Value =
        serde_json::from_slice(&fs::read(bundle.join("artifacts").join(root)).unwrap()).unwrap();
    let directory = manifest[B64.encode("nested")]["hash"].as_str().unwrap();
    let manifest: Value =
        serde_json::from_slice(&fs::read(bundle.join("artifacts").join(directory)).unwrap())
            .unwrap();
    let hash = manifest[B64.encode("file")]["hash"].as_str().unwrap();
    fs::read(bundle.join("artifacts").join(hash)).unwrap()
}

#[tokio::test]
async fn installed_global_setup_recovery_export_and_uninstall() {
    let fixture = Fixture::new();
    let opencode_path = fixture.home.join(".config/opencode/opencode.jsonc");
    let original = r#"{
  // Keep my client settings.
  "model": "fixture/model",
  "plugin": ["unrelated-plugin"],
  "mcp": {
    "other": {
      "type": "local",
      "command": ["other"]
    }
  },
}
"#;
    fs::write(&opencode_path, original).unwrap();
    fs::write(
        fixture.home.join(".cursor/hooks.json"),
        serde_json::to_vec_pretty(&json!({
            "version": 1,
            "hooks": {"beforeSubmitPrompt": [{"command": "unrelated-hook"}]},
        }))
        .unwrap(),
    )
    .unwrap();
    let setup = checked(fixture.setup());
    assert_eq!(setup["scope"], "user_global");
    assert_eq!(setup["data_dir"], fixture.data.to_str().unwrap());
    let configured = fs::read(&opencode_path).unwrap();
    checked(fixture.setup());
    assert_eq!(fs::read(&opencode_path).unwrap(), configured);
    assert!(
        String::from_utf8(configured)
            .unwrap()
            .contains("// Keep my client settings.")
    );
    let credential = fs::read(fixture.data.join("credential")).unwrap();
    // Resume a partially applied setup using its durable ownership record.
    fs::remove_file(fixture.home.join(".cursor/mcp.json")).unwrap();
    checked(fixture.setup());
    assert!(fixture.home.join(".cursor/mcp.json").exists());
    assert_eq!(
        fs::read(fixture.data.join("credential")).unwrap(),
        credential
    );

    let repo = fixture.temporary.path().join("source");
    fs::create_dir_all(repo.join("nested")).unwrap();
    git(&repo, &["init", "-q"]);
    fs::write(repo.join("nested/file"), b"HEAD\n").unwrap();
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-qm", "baseline"]);
    fs::write(repo.join("nested/file"), b"INDEX\n").unwrap();
    git(&repo, &["add", "."]);
    fs::write(repo.join("nested/file"), b"DIRTY\n").unwrap();
    let mut event = json!({
        "conversation_id": "cursor-e2e",
        "generation_id": "generation-one",
        "cursor_version": "fixture",
        "workspace_roots": [repo],
        "hook_event_name": "beforeSubmitPrompt",
        "prompt": "portable needle",
    });
    fixture.hook(&event);
    event["hook_event_name"] = "preToolUse".into();
    event["tool_use_id"] = "failed-tool".into();
    event["tool_name"] = "Shell".into();
    event["tool_input"] = json!({"command": "edit then fail"});
    fixture.hook(&event);
    fs::write(repo.join("nested/file"), [0, 255, 7, 42]).unwrap();
    event["hook_event_name"] = "postToolUseFailure".into();
    event["error_message"] = "source-reported failure".into();
    fixture.hook(&event);
    // Drain must never sample this later filesystem state.
    fs::write(repo.join("nested/file"), b"AFTER CAPTURE\n").unwrap();
    event["hook_event_name"] = "afterAgentResponse".into();
    event["text"] = "portable answer".into();
    fixture.hook(&event);
    for generation in ["generation-two", "generation-three"] {
        event["hook_event_name"] = "beforeSubmitPrompt".into();
        event["generation_id"] = generation.into();
        fixture.hook(&event);
    }
    let native = |event: Value| {
        json!({
            "directory": repo,
            "project": "fixture",
            "source_version": "fixture",
            "event": event,
        })
    };
    fixture.run(
        &["capture-opencode"],
        Some(&native(json!({
            "type": "message.updated",
            "properties": {
                "info": {
                    "id": "message",
                    "sessionID": "opencode-e2e",
                    "role": "assistant",
                },
            },
        }))),
    );
    fixture.run(
        &["capture-opencode"],
        Some(&native(json!({
            "type": "message.part.updated",
            "properties": {
                "part": {
                    "id": "part",
                    "messageID": "message",
                    "sessionID": "opencode-e2e",
                    "type": "text",
                    "text": "OpenCode cross-client answer",
                },
            },
        }))),
    );
    fs::remove_dir_all(&repo).unwrap();
    let service = fixture.start().await;
    assert_eq!(fixture.run(&["flush"], None)["pending"], 0);
    let prompts = fixture.run(
        &["history", "search", "portable needle", "--kind", "message"],
        None,
    );
    assert_eq!(prompts["items"].as_array().unwrap().len(), 3);
    let sessions = fixture.run(&["history", "list"], None);
    assert_eq!(sessions["items"].as_array().unwrap().len(), 2);
    let cursor = sessions["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|session| session["source"] == "cursor")
        .unwrap();
    let session = cursor["id"].as_str().unwrap();
    let mut delivery = json!({
        "schema_version": 1,
        "delivery_id": uuid::Uuid::new_v4().to_string(),
        "origin": {
            "device_id": cursor["device_id"],
            "installation_id": cursor["installation_id"],
            "source": "cursor",
            "source_version": "fixture",
        },
        "sequence": 10000,
        "native_session_id": cursor["native_id"],
        "project_id": cursor["project_id"],
        "workspace_id": cursor["workspace_id"],
        "observed_at": 100,
        "change": {
            "type": "part",
            "message_id": "direct",
            "native_id": "text",
            "kind": "text",
            "text": "explicit replay",
            "removed": false,
        },
        "source_payload": {"fixture": true},
    });
    let receipt = fixture.run(&["capture"], Some(&delivery));
    assert_eq!(receipt, fixture.run(&["capture"], Some(&delivery)));
    delivery["source_payload"] = json!({"conflict": true});
    let mut conflicting = fixture
        .command()
        .arg("capture")
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    conflicting
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&delivery).unwrap())
        .unwrap();
    let conflict = conflicting.wait_with_output().unwrap();
    assert!(!conflict.status.success());
    let error: Value = serde_json::from_slice(&conflict.stderr).unwrap();
    assert_eq!(error["error"]["code"], "delivery_conflict");
    let descriptor = fixture.run(&["history", "export", session], None);
    let request = json!({
        "operation": "export",
        "session_id": session,
    });
    let rest: Value = reqwest::Client::new()
        .post(format!("http://{}/v1/history/query", fixture.address))
        .bearer_auth(String::from_utf8(credential.clone()).unwrap())
        .json(&request)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(descriptor, rest);
    let mut command = tokio::process::Command::new(fixture.binary());
    command
        .env_remove("CODELOOPS_DATA_DIR")
        .env_remove("CODELOOPS_ADDRESS")
        .arg("mcp");
    let mcp = ().serve(TokioChildProcess::new(command).unwrap()).await.unwrap();
    let response = mcp
        .call_tool(
            CallToolRequestParams::new("history_query")
                .with_arguments(json!({"request": request}).as_object().unwrap().clone()),
        )
        .await
        .unwrap();
    assert_eq!(response.structured_content.unwrap(), descriptor);
    mcp.cancel().await.unwrap();
    let bundle = fixture.temporary.path().join("export");
    fixture.run(
        &[
            "history",
            "export",
            session,
            "--output",
            bundle.to_str().unwrap(),
        ],
        None,
    );
    let before = fixture.run(&["history", "search", "portable answer"], None);
    drop(service);
    let service = fixture.start().await;
    assert_eq!(
        before,
        fixture.run(&["history", "search", "portable answer"], None)
    );
    drop(service);
    assert_eq!(
        fixture.run(&["verify-export", bundle.to_str().unwrap()], None)["verified"],
        true
    );
    let entries = records(&bundle, "entries");
    let tool = entries
        .iter()
        .find(|entry| entry["kind"] == "tool")
        .unwrap();
    assert_eq!(tool["tool"]["status"], "failed");
    let checkpoints = records(&bundle, "checkpoints");
    let link = &tool["checkpoints"][0];
    let after = checkpoints
        .iter()
        .find(|record| record["id"] == link["checkpoint_id"])
        .unwrap();
    assert_eq!(captured_file(&bundle, after, "worktree"), [0, 255, 7, 42]);
    let baseline = checkpoints
        .iter()
        .find(|record| record["id"] == link["baseline_id"])
        .unwrap();
    assert_eq!(captured_file(&bundle, baseline, "head"), b"HEAD\n");
    assert_eq!(captured_file(&bundle, baseline, "index"), b"INDEX\n");
    assert_eq!(captured_file(&bundle, baseline, "worktree"), b"DIRTY\n");
    let manifest_text = fs::read_to_string(bundle.join("manifest.json")).unwrap();
    assert!(!manifest_text.contains(std::str::from_utf8(&credential).unwrap()));
    fs::write(fixture.prefix.join("keep-user-file"), "unrelated asset").unwrap();
    let uninstall = fixture.run(&["uninstall"], None);
    assert_eq!(uninstall["uninstalled"], true);
    assert!(!fixture.binary().exists());
    let unowned_wrapper = fixture.prefix.join("share/codeloops/configured-history.ts");
    fs::write(&unowned_wrapper, "user file after uninstall").unwrap();
    let repeated = Command::new(env!("CARGO_BIN_EXE_codeloops"))
        .args(["uninstall", "--prefix"])
        .arg(&fixture.prefix)
        .output()
        .unwrap();
    assert_eq!(checked(repeated)["uninstalled"], true);
    assert_eq!(
        fs::read_to_string(unowned_wrapper).unwrap(),
        "user file after uninstall"
    );
    assert!(fixture.data.join("archive/history.sqlite3").exists());
    assert!(fixture.data.join("opencode-spool.sqlite3").exists());
    assert_eq!(
        fs::read(fixture.data.join("credential")).unwrap(),
        credential
    );
    assert!(fixture.prefix.join("keep-user-file").exists());
    let restored = fs::read_to_string(&opencode_path).unwrap();
    assert!(restored.contains("unrelated-plugin"));
    assert!(restored.contains("// Keep my client settings."));
    assert!(!restored.contains("codeloops-history-preview"));
    let hooks: Value =
        serde_json::from_slice(&fs::read(fixture.home.join(".cursor/hooks.json")).unwrap())
            .unwrap();
    assert_eq!(
        hooks["hooks"]["beforeSubmitPrompt"],
        json!([{"command": "unrelated-hook"}])
    );
    // The portable copy still verifies after the original archive is removed.
    fs::remove_dir_all(&fixture.data).unwrap();
    let verify = Command::new(env!("CARGO_BIN_EXE_codeloops"))
        .arg("verify-export")
        .arg(&bundle)
        .output()
        .unwrap();
    assert_eq!(checked(verify)["verified"], true);
    let content = tool["tool"]["error_hash"].as_str().unwrap();
    fs::write(bundle.join("artifacts").join(content), b"corrupted").unwrap();
    assert!(
        !Command::new(env!("CARGO_BIN_EXE_codeloops"))
            .arg("verify-export")
            .arg(&bundle)
            .output()
            .unwrap()
            .status
            .success()
    );
}

#[test]
fn setup_preflight_preserves_malformed_and_conflicting_settings() {
    let fixture = Fixture::new();
    let path = fixture.home.join(".cursor/mcp.json");
    fs::write(&path, "{malformed").unwrap();
    assert!(!fixture.setup().status.success());
    assert_eq!(fs::read_to_string(&path).unwrap(), "{malformed");
    assert!(!fixture.prefix.join("share/codeloops/setup.json").exists());
    assert!(!fixture.home.join(".config/opencode/opencode.json").exists());
    fs::write(
        &path,
        serde_json::to_vec_pretty(&json!({
            "mcpServers": {"codeloops-history-preview": {"command": "user-owned"}},
        }))
        .unwrap(),
    )
    .unwrap();
    let original = fs::read(&path).unwrap();
    assert!(!fixture.setup().status.success());
    assert_eq!(fs::read(&path).unwrap(), original);
    fs::write(&path, "{}\n").unwrap();
    checked(fixture.setup());
    let second = Fixture::new();
    let opencode = fixture.home.join(".config/opencode/opencode.json");
    let second_setup = second
        .command()
        .arg("--data-dir")
        .arg(&second.data)
        .args(["--address", &second.address, "setup", "--profile", "review"])
        .args(["--opencode-version", "fixture", "--opencode-config"])
        .arg(&opencode)
        .arg("--cursor-config-dir")
        .arg(fixture.home.join(".cursor"))
        .output()
        .unwrap();
    checked(second_setup);
    let config: Value = serde_json::from_slice(&fs::read(&opencode).unwrap()).unwrap();
    assert_eq!(config["plugin"].as_array().unwrap().len(), 2);
    assert!(config["mcp"]["codeloops-history-preview"].is_object());
    assert!(config["mcp"]["codeloops-history-review"].is_object());
    assert_ne!(
        fs::read(fixture.data.join("credential")).unwrap(),
        fs::read(second.data.join("credential")).unwrap()
    );
    second.run(&["uninstall"], None);
    let config: Value = serde_json::from_slice(&fs::read(&opencode).unwrap()).unwrap();
    assert_eq!(config["plugin"].as_array().unwrap().len(), 1);
    assert!(config["mcp"]["codeloops-history-preview"].is_object());
    let hooks_path = fixture.home.join(".cursor/hooks.json");
    let mut hooks: Value = serde_json::from_slice(&fs::read(&hooks_path).unwrap()).unwrap();
    hooks["hooks"]["preToolUse"][0]["timeout"] = 99.into();
    fs::write(&hooks_path, serde_json::to_vec_pretty(&hooks).unwrap()).unwrap();
    assert!(
        !fixture
            .command()
            .arg("uninstall")
            .output()
            .unwrap()
            .status
            .success()
    );
    assert!(fixture.binary().exists());
    assert_eq!(
        serde_json::from_slice::<Value>(&fs::read(hooks_path).unwrap()).unwrap(),
        hooks
    );
}

#[test]
fn installed_explicit_flush_drains_multiple_batches_offline() {
    let fixture = Fixture::new();
    checked(fixture.setup());
    for index in 0..105 {
        fixture.run(
            &["capture-opencode"],
            Some(&json!({
                "source_version": "fixture",
                "directory": fixture.home,
                "project": "fixture",
                "event": {
                    "type": "message.part.updated",
                    "properties": {
                        "part": {
                            "id": "part",
                            "messageID": "message",
                            "sessionID": "bulk",
                            "type": "text",
                            "text": format!("revision {index}"),
                        },
                    },
                },
            })),
        );
    }
    let health = fixture.run(&["flush"], None);
    assert_eq!(health["pending"], 0);
    assert_eq!(health["rejected"], 0);
    assert!(health["delivered"].as_u64().unwrap() >= 105);
    assert_eq!(health, fixture.run(&["flush"], None));
}
