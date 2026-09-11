use base64::{Engine, engine::general_purpose::STANDARD as B64};
use rmcp::{ServiceExt, model::CallToolRequestParams, transport::TokioChildProcess};
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    net::TcpListener,
    os::unix::fs::PermissionsExt,
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
        let fixture = Self::uninstalled();
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

    fn uninstalled() -> Self {
        let temporary = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(temporary.path()).unwrap();
        let prefix = root.join("preview install 'quoted'");
        let home = root.join("home");
        let data = root.join("data 'isolated'");
        fs::create_dir_all(home.join(".config/opencode")).unwrap();
        fs::create_dir_all(home.join(".cursor")).unwrap();
        let manager = home.join("manager");
        fs::create_dir_all(&manager).unwrap();
        for program in ["systemctl", "journalctl", "launchctl"] {
            let path = manager.join(program);
            fs::write(&path, include_bytes!("fixtures/service-manager.py")).unwrap();
            fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
        }
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap().to_string();
        Self {
            temporary,
            prefix,
            home,
            data,
            address,
        }
    }

    fn binary(&self) -> PathBuf {
        self.prefix.join("bin/codeloops")
    }

    fn command(&self) -> Command {
        let mut command = Command::new(self.binary());
        command
            .env("PATH", self.manager_path())
            .env("CODELOOPS_TEST_SERVICE_DIR", self.home.join("manager"))
            .env("HOME", &self.home)
            .env("XDG_CONFIG_HOME", self.home.join(".config"))
            .env("XDG_DATA_HOME", self.home.join(".local/share"))
            .env_remove("CODELOOPS_DATA_DIR")
            .env_remove("CODELOOPS_ADDRESS")
            .env_remove("CODELOOPS_OPENCODE_VERSION")
            .arg("--json");
        command
    }

    fn manager_path(&self) -> String {
        format!(
            "{}:{}",
            self.home.join("manager").display(),
            std::env::var("PATH").unwrap()
        )
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

impl Drop for Fixture {
    fn drop(&mut self) {
        if self.prefix.join("share/codeloops/service.json").exists() {
            let _ = self.command().args(["service", "stop"]).output();
        }
        // Recovery assertions may have removed the installed binary. Reap only
        // this fixture's detached process through its transport as a fallback.
        if self.home.join("manager/state.json").exists() {
            let _ = Command::new(self.home.join("manager/systemctl"))
                .env("CODELOOPS_TEST_SERVICE_DIR", self.home.join("manager"))
                .args(["--user", "stop", "fixture"])
                .output();
        }
    }
}

// Exercise the user's Make entry point, including the real release build. Keep
// the toolchain available while all runtime and client state uses the fixture.
fn make(fixture: &Fixture, target: &str) -> Command {
    let home = PathBuf::from(std::env::var_os("HOME").unwrap());
    let mut command = Command::new("make");
    command
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .args(["--no-print-directory", "-j4", target])
        .arg(format!("PREFIX={}", fixture.prefix.display()))
        .arg(format!("DATA_DIR={}", fixture.data.display()))
        .arg(format!("ADDRESS={}", fixture.address))
        .arg("PROFILE=quickstart")
        .arg("SETUP_ARGS=--opencode-version fixture")
        .env("PATH", fixture.manager_path())
        .env("CODELOOPS_TEST_SERVICE_DIR", fixture.home.join("manager"))
        .env("HOME", &fixture.home)
        .env("XDG_CONFIG_HOME", fixture.home.join(".config"))
        .env("XDG_DATA_HOME", fixture.home.join(".local/share"))
        .env(
            "CARGO_HOME",
            std::env::var_os("CARGO_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| home.join(".cargo")),
        )
        .env(
            "RUSTUP_HOME",
            std::env::var_os("RUSTUP_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| home.join(".rustup")),
        )
        .env_remove("CODELOOPS_DATA_DIR")
        .env_remove("CODELOOPS_ADDRESS")
        .env_remove("MAKEFLAGS")
        .env_remove("MFLAGS")
        .env_remove("CARGO_MAKEFLAGS");
    command
}

fn make_start(fixture: &Fixture) -> String {
    let output = make(fixture, "start").output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    // Make has exited, but the managed service must remain available.
    fixture.run(&["health"], None);
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn make_setup_bootstraps_and_repeats_without_duplicate_registrations() {
    let fixture = Fixture::uninstalled();
    let config = fixture.home.join(".config/opencode/opencode.jsonc");
    fs::write(
        &config,
        "{\n// keep my settings\n\"plugin\": [\"my-plugin\"]\n}\n",
    )
    .unwrap();
    let other_config = fixture.home.join(".config/opencode/opencode.json");
    fs::write(&other_config, "{\"plugin\": [\"other-plugin\"]}").unwrap();
    assert!(!fixture.binary().exists());
    let setup = || {
        let output = make(&fixture, "setup").output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    };
    setup();
    let first = fs::read_to_string(&config).unwrap();
    let hooks = fs::read(fixture.home.join(".cursor/hooks.json")).unwrap();
    assert!(first.contains("// keep my settings"));
    assert!(first.contains("my-plugin"));
    assert!(first.contains("codeloops-history-quickstart"));
    setup();
    assert_eq!(fs::read_to_string(config).unwrap(), first);
    assert_eq!(
        fs::read(fixture.home.join(".cursor/hooks.json")).unwrap(),
        hooks
    );
    assert!(fixture.binary().is_file());
    assert_eq!(
        fs::read_to_string(other_config).unwrap(),
        "{\"plugin\": [\"other-plugin\"]}"
    );
    assert!(
        TcpListener::bind(&fixture.address).is_ok(),
        "setup must not start a service"
    );
}

#[test]
fn make_start_from_fresh_prefix_captures_and_recalls_after_restart() {
    let fixture = Fixture::uninstalled();
    let json = fixture.home.join(".config/opencode/opencode.json");
    let jsonc = fixture.home.join(".config/opencode/opencode.jsonc");
    let lower = "{\"plugin\": [\"lower-priority-plugin\"]}";
    fs::write(&json, lower).unwrap();
    fs::write(&jsonc, "{\n// keep me\n\"plugin\": [\"active-plugin\"]\n}").unwrap();
    let output = make_start(&fixture);
    let configured = fs::read_to_string(&jsonc).unwrap();
    assert!(configured.contains("// keep me"));
    assert!(configured.contains("active-plugin"));
    assert!(configured.contains("configured-history.ts"));
    assert_eq!(fs::read_to_string(&json).unwrap(), lower);
    fixture.hook(&json!({
        "hook_event_name": "afterAgentResponse",
        "conversation_id": "quickstart-conversation",
        "generation_id": "quickstart-turn",
        "text": "codeloops first recall check",
    }));
    fixture.run(&["flush"], None);
    let args = [
        "history",
        "search",
        "codeloops first recall check",
        "--role",
        "assistant",
    ];
    let result = fixture.run(&args, None);
    assert_eq!(result["items"].as_array().unwrap().len(), 1);
    assert!(output.contains("running in the background"));
    assert!(output.contains("You can close this terminal"));
    assert!(output.contains("Restart OpenCode"));
    assert!(!output.contains("service_unavailable"));
    assert_eq!(fixture.run(&["service", "status"], None)["running"], true);
    let logs = fixture.run(&["service", "logs"], None);
    assert!(
        logs["logs"]
            .as_str()
            .unwrap()
            .contains("CodeLoops listening at")
    );
    fixture.run(&["service", "stop"], None);
    assert!(TcpListener::bind(&fixture.address).is_ok());
    make_start(&fixture);
    assert_eq!(fixture.run(&args, None), result);
    assert_eq!(fs::read_to_string(jsonc).unwrap(), configured);
    assert_eq!(fs::read_to_string(json).unwrap(), lower);
    fixture.run(&["uninstall"], None);
    assert!(TcpListener::bind(&fixture.address).is_ok());
    assert!(fixture.data.join("archive/history.sqlite3").exists());
}

#[test]
fn managed_service_start_failure_does_not_report_success_or_leave_a_process() {
    let fixture = Fixture::new();
    checked(fixture.setup());
    let output = fixture
        .command()
        .env("CODELOOPS_TEST_SERVICE_FAIL", "1")
        .args(["service", "start"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("fixture manager refused startup"));
    assert!(!String::from_utf8_lossy(&output.stdout).contains("running in the background"));
    assert_eq!(fixture.run(&["service", "status"], None)["running"], false);
    assert!(TcpListener::bind(&fixture.address).is_ok());
    fixture.run(&["service", "start"], None);
    assert_eq!(fixture.run(&["service", "status"], None)["healthy"], true);
    fixture.run(&["service", "start"], None);
    fixture.run(&["service", "stop"], None);
    fixture.run(&["service", "stop"], None);
}

#[tokio::test]
async fn managed_start_does_not_claim_or_kill_an_existing_foreground_service() {
    let fixture = Fixture::new();
    checked(fixture.setup());
    let _foreground = fixture.start().await;
    let output = fixture
        .command()
        .args(["service", "start"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("press Ctrl+C"));
    assert!(!fixture.prefix.join("share/codeloops/service.json").exists());
    fixture.run(&["health"], None);
}

#[test]
fn managed_service_preserves_modified_definitions_and_uninstall_recovers_missing_binary() {
    let fixture = Fixture::new();
    checked(fixture.setup());
    fixture.run(&["service", "start"], None);
    let record: Value = serde_json::from_slice(
        &fs::read(fixture.prefix.join("share/codeloops/service.json")).unwrap(),
    )
    .unwrap();
    let definition = PathBuf::from(record["path"].as_str().unwrap());
    let original = fs::read(&definition).unwrap();
    fs::write(&definition, "user-modified service").unwrap();
    let output = fixture.command().arg("uninstall").output().unwrap();
    let preserved = fs::read_to_string(&definition).unwrap();
    fs::write(&definition, original).unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("service definition was modified"));
    assert_eq!(preserved, "user-modified service");
    assert!(fixture.binary().exists());
    fs::remove_file(fixture.binary()).unwrap();
    let output = make(&fixture, "uninstall").output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!definition.exists());
    assert!(!fixture.prefix.join("share/codeloops/service.json").exists());
    assert!(TcpListener::bind(&fixture.address).is_ok());
    assert!(fixture.data.join("archive/history.sqlite3").exists());
}

#[test]
fn setup_extends_inherited_opencode_plugins_without_shadowing_them() {
    for inherited_file in ["opencode.json", "config.json"] {
        let fixture = Fixture::new();
        let directory = fixture.home.join(".config/opencode");
        let inherited = directory.join(inherited_file);
        let jsonc = directory.join("opencode.jsonc");
        fs::write(&inherited, "{\"plugin\": [\"existing-plugin\"]}").unwrap();
        fs::write(
            &jsonc,
            "{\n// inherit plugins\n\"model\": \"fixture/model\"\n}",
        )
        .unwrap();
        checked(fixture.setup());
        let lower: Value = serde_json::from_slice(&fs::read(&inherited).unwrap()).unwrap();
        assert_eq!(lower["plugin"][0], "existing-plugin");
        assert_eq!(lower["plugin"].as_array().unwrap().len(), 2);
        assert!(
            lower["plugin"][1]
                .as_str()
                .unwrap()
                .contains("configured-history.ts")
        );
        let upper = fs::read_to_string(&jsonc).unwrap();
        assert!(upper.contains("// inherit plugins"));
        assert!(upper.contains("fixture/model"));
        assert!(upper.contains("codeloops-history-preview"));
        assert!(!upper.contains("\"plugin\""));
        checked(fixture.setup());
        assert_eq!(fs::read_to_string(&jsonc).unwrap(), upper);
        let repeated: Value = serde_json::from_slice(&fs::read(&inherited).unwrap()).unwrap();
        assert_eq!(repeated, lower);
        fixture.run(&["uninstall"], None);
        let removed: Value = serde_json::from_slice(&fs::read(inherited).unwrap()).unwrap();
        assert_eq!(removed["plugin"], json!(["existing-plugin"]));
        assert!(!fs::read_to_string(jsonc).unwrap().contains("\"plugin\""));
    }
}

#[test]
fn setup_does_not_shadow_an_inherited_mcp_conflict() {
    let fixture = Fixture::new();
    let directory = fixture.home.join(".config/opencode");
    let json = directory.join("opencode.json");
    let jsonc = directory.join("opencode.jsonc");
    let lower = "{\"mcp\": {\"codeloops-history-preview\": {\"type\": \"local\", \"command\": [\"other\"]}}}";
    fs::write(&json, lower).unwrap();
    fs::write(&jsonc, "{}").unwrap();
    let output = fixture.setup();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("configuration conflict"));
    assert_eq!(fs::read_to_string(json).unwrap(), lower);
    assert_eq!(fs::read_to_string(jsonc).unwrap(), "{}");
    assert!(!fixture.home.join(".cursor/hooks.json").exists());
}

#[test]
fn setup_explicit_opencode_path_overrides_default_discovery() {
    let fixture = Fixture::new();
    let directory = fixture.home.join(".config/opencode");
    let json = directory.join("opencode.json");
    let jsonc = directory.join("opencode.jsonc");
    let untouched = "{\n// explicitly choose the other file\n\"plugin\": []\n}";
    fs::write(&json, "{\"plugin\": [\"existing-plugin\"]}").unwrap();
    fs::write(&jsonc, untouched).unwrap();
    for _ in 0..2 {
        checked(
            fixture
                .command()
                .arg("--data-dir")
                .arg(&fixture.data)
                .args([
                    "--address",
                    &fixture.address,
                    "setup",
                    "--opencode-version",
                    "fixture",
                    "--opencode-config",
                ])
                .arg(&json)
                .output()
                .unwrap(),
        );
    }
    let selected: Value = serde_json::from_slice(&fs::read(json).unwrap()).unwrap();
    assert_eq!(selected["plugin"][0], "existing-plugin");
    assert_eq!(selected["plugin"].as_array().unwrap().len(), 2);
    assert!(selected["mcp"]["codeloops-history-preview"].is_object());
    assert_eq!(fs::read_to_string(jsonc).unwrap(), untouched);
}

#[test]
fn make_start_stops_when_build_or_setup_fails() {
    let fixture = Fixture::uninstalled();
    let failed_build = make(&fixture, "start").arg("CARGO=false").output().unwrap();
    assert!(!failed_build.status.success());
    assert!(String::from_utf8_lossy(&failed_build.stderr).contains("Building CodeLoops"));
    assert!(!fixture.binary().exists());
    assert!(!fixture.home.join(".cursor/hooks.json").exists());

    let config = fixture.home.join(".config/opencode/opencode.json");
    fs::write(&config, "invalid json").unwrap();
    let output = make(&fixture, "start").output().unwrap();
    assert!(!output.status.success());
    assert!(
        fixture.binary().exists(),
        "build/install must succeed before testing setup failure"
    );
    assert!(!String::from_utf8_lossy(&output.stderr).contains("CodeLoops listening at"));
    assert_eq!(fs::read_to_string(config).unwrap(), "invalid json");
    assert!(TcpListener::bind(&fixture.address).is_ok());
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
