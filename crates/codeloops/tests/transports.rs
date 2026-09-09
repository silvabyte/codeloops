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
    let service = start(root, &address).await;
    cli(root, &address, &["flush"], None);
    let listed = cli(root, &address, &["history", "list"], None);
    let session = listed["items"][0]["id"].as_str().unwrap();
    assert_eq!(listed["items"][0]["state"], "idle");
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
