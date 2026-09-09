use rmcp::{ServiceExt, model::CallToolRequestParams, transport::TokioChildProcess};
use serde_json::{Value, json};
use std::{
    io::Write,
    net::TcpListener,
    path::Path,
    process::{Child, Command, Stdio},
    time::Duration,
};

struct Service(Child);
impl Drop for Service {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn command(root: &Path, address: &str) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_codeloops"));
    c.arg("--data-dir")
        .arg(root)
        .args(["--address", address, "--json"]);
    c
}

fn cli(root: &Path, address: &str, args: &[&str], input: Option<&Value>) -> Value {
    let mut child = command(root, address)
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
            .write_all(serde_json::to_string(input).unwrap().as_bytes())
            .unwrap();
    }
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

async fn start(root: &Path, address: &str) -> Service {
    let service = Service(
        command(root, address)
            .arg("serve")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    for _ in 0..200 {
        if let Ok(token) = std::fs::read_to_string(root.join("credential"))
            && reqwest::Client::new()
                .post(format!("http://{address}/v1/health"))
                .bearer_auth(token)
                .send()
                .await
                .is_ok()
        {
            return service;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("service did not start");
}

fn native(event: Value) -> Value {
    json!({"source_version":"1.18.30-fixture","directory":"/workspace with spaces","project":"native-project","event":event})
}

#[tokio::test]
async fn offline_capture_restart_and_real_mcp_cli_rest_have_equal_results() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap().to_string();
    drop(listener);
    let events = [
        json!({"type":"session.created","properties":{"info":{"id":"ses_one","title":"Round trip"}}}),
        json!({"type":"message.updated","properties":{"info":{"id":"msg_one","sessionID":"ses_one","role":"user","time":{"created":10}}}}),
        json!({"type":"message.part.updated","properties":{"part":{"id":"part_one","messageID":"msg_one","sessionID":"ses_one","type":"text","text":"find "},"unknown":"kept"}}),
        json!({"type":"message.part.delta","properties":{"partID":"part_one","messageID":"msg_one","sessionID":"ses_one","field":"text","delta":"my archived conversation"}}),
        json!({"type":"message.part.updated","properties":{"part":{"id":"part_one","messageID":"msg_one","sessionID":"ses_one","type":"text","text":"find my archived conversation"}}}),
        json!({"type":"session.idle","properties":{"sessionID":"ses_one"}}),
    ];
    for event in events {
        assert_eq!(
            cli(root, &address, &["capture-opencode"], Some(&native(event)))["queued"],
            true
        );
    }
    for (hook, field, text) in [
        (
            "beforeSubmitPrompt",
            "prompt",
            "sapphire cross-client question",
        ),
        ("afterAgentResponse", "text", "sapphire cross-client answer"),
    ] {
        let mut input = cursor_event(hook, "ses_one", "generation-one");
        input[field] = text.into();
        assert_eq!(
            cli(root, &address, &["capture-cursor"], Some(&input)),
            json!({})
        );
    }
    let service = start(root, &address).await;
    cli(root, &address, &["flush"], None);
    let listed = cli(root, &address, &["history", "list"], None);
    assert_eq!(listed["items"].as_array().unwrap().len(), 2);
    let opencode_session = listed["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["source"] == "opencode")
        .unwrap();
    let session = opencode_session["id"].as_str().unwrap();
    assert_eq!(opencode_session["state"], "idle");
    let before = cli(
        root,
        &address,
        &["history", "search", "archived conversation"],
        None,
    );
    assert_eq!(before["items"].as_array().unwrap().len(), 1);
    assert_eq!(before["items"][0]["role"], "user");
    assert_eq!(before["items"][0]["text"], "find my archived conversation");
    drop(service);
    let _service = start(root, &address).await;
    assert_eq!(
        before,
        cli(
            root,
            &address,
            &["history", "search", "archived conversation"],
            None
        )
    );
    let token = std::fs::read_to_string(root.join("credential")).unwrap();
    let client = reqwest::Client::new();
    let request = json!({"operation":"search","text":"archived conversation"});
    let url = format!("http://{address}/v1/history/query");
    assert_eq!(
        client
            .post(&url)
            .json(&request)
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    let rest: Value = client
        .post(&url)
        .bearer_auth(&token)
        .json(&request)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(before, rest);

    let mut mcp_command = tokio::process::Command::new(env!("CARGO_BIN_EXE_codeloops"));
    mcp_command
        .arg("--data-dir")
        .arg(root)
        .args(["--address", &address, "mcp"]);
    let mcp = ().serve(TokioChildProcess::new(mcp_command).unwrap()).await.unwrap();
    let tools = mcp.list_all_tools().await.unwrap();
    assert!(tools.iter().any(|t| t.name == "history_query"));
    let call = CallToolRequestParams::new("history_query")
        .with_arguments(json!({"request":request}).as_object().unwrap().clone());
    let result = mcp.call_tool(call).await.unwrap();
    assert_eq!(result.structured_content.unwrap(), rest);
    let cursor_search =
        json!({"operation":"search","text":"sapphire","filter":{"source":"cursor"}});
    let cursor_rest: Value = client
        .post(&url)
        .bearer_auth(&token)
        .json(&cursor_search)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(cursor_rest["items"].as_array().unwrap().len(), 2);
    assert_ne!(cursor_rest["items"][0]["session_id"], session);
    assert_eq!(
        cursor_rest,
        cli(
            root,
            &address,
            &["history", "search", "sapphire", "--source", "cursor"],
            None
        )
    );
    let cursor_mcp = mcp
        .call_tool(
            CallToolRequestParams::new("history_query").with_arguments(
                json!({"request":cursor_search})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
        )
        .await
        .unwrap();
    assert_eq!(cursor_mcp.structured_content.unwrap(), cursor_rest);
    let page_request = json!({"operation":"show","session_id":session,"page":{"limit":1}});
    let mcp_page = mcp
        .call_tool(
            CallToolRequestParams::new("history_query")
                .with_arguments(json!({"request":page_request}).as_object().unwrap().clone()),
        )
        .await
        .unwrap();
    assert_eq!(
        mcp_page.structured_content.unwrap(),
        cli(
            root,
            &address,
            &["history", "show", session, "--limit", "1"],
            None
        )
    );

    let invalid = json!({"operation":"entry","entry_id":"missing"});
    let response = client
        .post(&url)
        .bearer_auth(&token)
        .json(&invalid)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 404);
    let rest_error: Value = response.json().await.unwrap();
    let mcp_error = mcp
        .call_tool(
            CallToolRequestParams::new("history_query")
                .with_arguments(json!({"request":invalid}).as_object().unwrap().clone()),
        )
        .await
        .unwrap();
    assert_eq!(mcp_error.is_error, Some(true));
    assert_eq!(mcp_error.structured_content.unwrap(), rest_error);
    let output = command(root, &address)
        .args(["history", "entry", "missing"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stderr).unwrap(),
        rest_error
    );
    let mut failed = command(root, &address)
        .arg("capture-opencode")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    failed.stdin.take().unwrap().write_all(b"{}").unwrap();
    assert!(!failed.wait_with_output().unwrap().status.success());
    let health = cli(root, &address, &["health"], None);
    assert_eq!(health["enqueue_failures"], 1);
    assert!(
        health["last_error"]
            .as_str()
            .unwrap()
            .contains("missing field")
    );
    mcp.cancel().await.unwrap();
}

fn cursor_event(hook: &str, session: &str, generation: &str) -> Value {
    json!({"hook_event_name":hook,"conversation_id":session,"generation_id":generation,
        "cursor_version":"documented-fixture","workspace_roots":["/workspace with spaces"],"unknown":{"retained":true}})
}

#[test]
fn cursor_repeated_messages_relationships_lifecycle_and_delivery_replay() {
    use session_history::{
        History,
        model::{Capture, Filter, Page, Query},
    };
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let address = "127.0.0.1:47823";
    let submit = |hook: &str, generation: &str, field: &str, value: &str| {
        let mut input = cursor_event(hook, "conversation", generation);
        input[field] = value.into();
        assert_eq!(
            cli(root, address, &["capture-cursor"], Some(&input)),
            json!({})
        );
    };
    submit("beforeSubmitPrompt", "one", "prompt", "identical text");
    submit("beforeSubmitPrompt", "two", "prompt", "identical text");
    submit("afterAgentResponse", "one", "text", "first response");
    submit("afterAgentResponse", "one", "text", "second response");
    submit("stop", "two", "status", "completed");
    submit("sessionStart", "two", "session_id", "conversation");
    submit("subagentStop", "two", "status", "error");
    let db = rusqlite::Connection::open(root.join("opencode-spool.sqlite3")).unwrap();
    let envelopes: Vec<Capture> = db
        .prepare("SELECT envelope FROM queue ORDER BY sequence")
        .unwrap()
        .query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .map(|r| serde_json::from_str(&r.unwrap()).unwrap())
        .collect();
    assert!(
        envelopes
            .iter()
            .all(|c| c.source_payload["unknown"]["retained"] == true)
    );
    let mut history = History::open(root.join("archive")).unwrap();
    // Simulate archive commit followed by process death before outbox deletion.
    let receipt = history.ingest(envelopes[0].clone()).unwrap();
    assert_eq!(
        receipt.capture_id,
        history.ingest(envelopes[0].clone()).unwrap().capture_id
    );
    let health = cli(root, address, &["flush"], None);
    assert_eq!(health["sources"]["cursor"]["pending"], 0);
    assert_eq!(health["sources"]["cursor"]["rejected"], 0);
    let sessions = history
        .query(Query::List {
            filter: Filter::default(),
            page: Page::default(),
        })
        .unwrap();
    assert_eq!(sessions["items"][0]["state"], "idle");
    let search = |text: &str, history: &mut History| {
        history
            .query(Query::Search {
                text: text.into(),
                filter: Filter::default(),
                page: Page::default(),
            })
            .unwrap()
    };
    let prompts = search("identical", &mut history);
    assert_eq!(prompts["items"].as_array().unwrap().len(), 2);
    assert_ne!(prompts["items"][0]["id"], prompts["items"][1]["id"]);
    let responses = search("response", &mut history);
    assert_eq!(responses["items"].as_array().unwrap().len(), 2);
    let prompt_native = envelopes
        .iter()
        .find_map(|c| match &c.change {
            session_history::model::Change::Message {
                native_id, role, ..
            } if role == "user" => Some(native_id),
            _ => None,
        })
        .unwrap();
    for capture in &envelopes {
        if let session_history::model::Change::Message {
            role,
            parent_native_id,
            ..
        } = &capture.change
            && role == "assistant"
        {
            assert_eq!(parent_native_id.as_ref(), Some(prompt_native));
        }
    }
    let session = sessions["items"][0]["id"].as_str().unwrap();
    let captures = history
        .query(Query::Captures {
            session_id: session.into(),
            page: Page {
                limit: 100,
                cursor: None,
            },
        })
        .unwrap();
    assert_eq!(captures["items"].as_array().unwrap().len(), envelopes.len());
    submit("stop", "two", "status", "aborted");
    cli(root, address, &["flush"], None);
    let sessions = history
        .query(Query::List {
            filter: Filter::default(),
            page: Page::default(),
        })
        .unwrap();
    assert_eq!(sessions["items"][0]["state"], "interrupted");
    submit("sessionEnd", "two", "reason", "user_close");
    cli(root, address, &["flush"], None);
    let sessions = history
        .query(Query::List {
            filter: Filter::default(),
            page: Page::default(),
        })
        .unwrap();
    assert_eq!(sessions["items"][0]["state"], "ended");
}

#[test]
fn cursor_hook_failures_are_neutral_and_reported_without_partial_enqueue() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let invalid = [
        json!({}),
        cursor_event("unsupported", "conversation", "one"),
        cursor_event("beforeSubmitPrompt", "conversation", "one"),
    ];
    for input in invalid {
        assert_eq!(
            cli(root, "127.0.0.1:47823", &["capture-cursor"], Some(&input)),
            json!({})
        );
    }
    let health = cli(root, "127.0.0.1:47823", &["flush"], None);
    assert_eq!(health["pending"], 0);
    assert_eq!(health["sources"]["cursor"]["enqueue_failures"], 3);
    assert_eq!(health["sources"]["opencode"]["enqueue_failures"], 0);
    // Initialization failure also remains fail-open, including with failClosed hooks.
    let file = root.join("not-a-directory");
    std::fs::write(&file, b"fixture").unwrap();
    assert_eq!(
        cli(
            &file,
            "127.0.0.1:47823",
            &["capture-cursor"],
            Some(&json!({}))
        ),
        json!({})
    );
}

#[test]
fn concurrent_cursor_hooks_keep_identity_roots_and_optional_transcripts() {
    use session_history::{
        History,
        model::{Capture, Filter, Page, Query},
    };
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::thread::scope(|scope| {
        for generation in 0..8 {
            scope.spawn(move || {
                let mut input = cursor_event(
                    "beforeSubmitPrompt",
                    "same-native-id",
                    &generation.to_string(),
                );
                input["prompt"] = "parallel identical prompt".into();
                input["workspace_roots"] = json!(["/a/project", "/b/project", "/a/project"]);
                if generation % 2 == 0 {
                    input["transcript_path"] = json!("/does/not/exist");
                }
                assert_eq!(
                    cli(root, "127.0.0.1:47823", &["capture-cursor"], Some(&input)),
                    json!({})
                );
            });
        }
    });
    let mut child = cursor_event("sessionStart", "distinct-child-conversation", "child");
    child["parent_conversation_id"] = "same-native-id".into();
    cli(root, "127.0.0.1:47823", &["capture-cursor"], Some(&child));
    cli(root, "127.0.0.1:47823", &["flush"], None);
    let history = History::open(root.join("archive")).unwrap();
    let found = history
        .query(Query::Search {
            text: "parallel identical prompt".into(),
            filter: Filter::default(),
            page: Page::default(),
        })
        .unwrap();
    assert_eq!(found["items"].as_array().unwrap().len(), 8);
    let sessions = history
        .query(Query::List {
            filter: Filter::default(),
            page: Page::default(),
        })
        .unwrap();
    assert_eq!(sessions["items"].as_array().unwrap().len(), 2);
    let session = found["items"][0]["session_id"].as_str().unwrap();
    let captures = history
        .query(Query::Captures {
            session_id: session.into(),
            page: Page {
                limit: 100,
                cursor: None,
            },
        })
        .unwrap();
    assert_eq!(captures["items"].as_array().unwrap().len(), 32);
    let child_session = sessions["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["native_id"] == "distinct-child-conversation")
        .unwrap();
    // The list projection exposes the parent native identity, not an invented child transcript.
    assert_eq!(child_session["parent_native_id"], "same-native-id");
    let parent_session = sessions["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["native_id"] == "same-native-id")
        .unwrap();
    assert_eq!(parent_session["workspace_ids"].as_array().unwrap().len(), 2);
    let mut input = cursor_event("beforeSubmitPrompt", "rootless", "one");
    input.as_object_mut().unwrap().remove("workspace_roots");
    input.as_object_mut().unwrap().remove("cursor_version");
    input["prompt"] = "rootless text".into();
    cli(root, "127.0.0.1:47823", &["capture-cursor"], Some(&input));
    let spool = rusqlite::Connection::open(root.join("opencode-spool.sqlite3")).unwrap();
    let envelope: String = spool
        .query_row(
            "SELECT envelope FROM queue ORDER BY sequence LIMIT 1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let capture: Capture = serde_json::from_str(&envelope).unwrap();
    assert_eq!(capture.origin.source_version, "unknown");
}

#[test]
fn shared_outbox_upgrades_legacy_identity_health_and_delta_baseline() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let db = rusqlite::Connection::open(root.join("opencode-spool.sqlite3")).unwrap();
    db.execute_batch("CREATE TABLE identities(key TEXT PRIMARY KEY,id TEXT NOT NULL);
      INSERT INTO identities VALUES('device','00000000-0000-4000-8000-000000000001'),('opencode','00000000-0000-4000-8000-000000000002');
      CREATE TABLE counter(value INTEGER NOT NULL); INSERT INTO counter VALUES(50);
      CREATE TABLE parts(session TEXT,message TEXT,part TEXT,kind TEXT,text TEXT,PRIMARY KEY(session,message,part));
      INSERT INTO parts VALUES('session','message','part','text','old ');
      CREATE TABLE health(id INTEGER PRIMARY KEY CHECK(id=1),delivered INTEGER NOT NULL DEFAULT 0,last_error TEXT);
      INSERT INTO health VALUES(1,12,'legacy failure');
      CREATE TABLE failures(id INTEGER PRIMARY KEY AUTOINCREMENT,observed INTEGER NOT NULL,message TEXT NOT NULL);
      INSERT INTO failures(observed,message) VALUES(1,'legacy failure');").unwrap();
    cli(
        root,
        "127.0.0.1:47823",
        &["capture-opencode"],
        Some(&native(
            json!({"type":"message.part.delta","properties":{"sessionID":"session","messageID":"message","partID":"part","field":"text","delta":"text"}}),
        )),
    );
    let envelope: String = db
        .query_row("SELECT envelope FROM queue", [], |r| r.get(0))
        .unwrap();
    let value: Value = serde_json::from_str(&envelope).unwrap();
    assert_eq!(
        value["origin"]["device_id"],
        "00000000-0000-4000-8000-000000000001"
    );
    assert_eq!(
        value["origin"]["installation_id"],
        "00000000-0000-4000-8000-000000000002"
    );
    assert_eq!(value["sequence"], 51);
    assert_eq!(value["change"]["text"], "old text");
    let health = cli(root, "127.0.0.1:47823", &["flush"], None);
    assert_eq!(health["sources"]["opencode"]["delivered"], 13);
    assert_eq!(health["sources"]["opencode"]["enqueue_failures"], 1);
    assert_eq!(health["sources"]["cursor"]["delivered"], 0);
}
