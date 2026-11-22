//! Offline executable-level MCP framing and JSON-RPC regression contracts.

use serde_json::{json, Value};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

fn initialize(version: &str) -> Value {
    json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": {"protocolVersion": version, "capabilities": {},
                   "clientInfo": {"name": "offline-contract", "version": "1"}}
    })
}

fn isolated_command() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_fyi-mcp"));
    for (name, _) in std::env::vars_os() {
        if name.to_string_lossy().starts_with("FYI_MCP_") || name == "DATABASE_URL" {
            command.env_remove(name);
        }
    }
    command
        .env("RUST_LOG", "info")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    command
}

async fn exchange(lines: &[String]) -> (Vec<Value>, String) {
    let mut child = isolated_command()
        .env("FYI_MCP_EPHEMERAL", "1")
        .stdin(Stdio::piped())
        .spawn()
        .expect("start the real MCP executable");
    let mut input = child.stdin.take().unwrap();
    let bytes = lines.join("\n") + "\n";
    let writer = tokio::spawn(async move {
        input.write_all(bytes.as_bytes()).await.unwrap();
        input.shutdown().await.unwrap();
    });
    let output = tokio::time::timeout(Duration::from_secs(15), child.wait_with_output())
        .await
        .expect("MCP must finish after stdin EOF")
        .unwrap();
    writer.await.unwrap();
    assert!(
        output.status.success(),
        "server failed: {:?}",
        output.status
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let responses = stdout
        .lines()
        .map(|line| serde_json::from_str(line).expect("stdout must contain only JSON-RPC"))
        .collect();
    (responses, String::from_utf8(output.stderr).unwrap())
}

#[tokio::test]
async fn ambiguous_remote_credentials_fail_startup_before_database_or_protocol_output() {
    struct ScratchDirectory(std::path::PathBuf);
    impl Drop for ScratchDirectory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = std::env::temp_dir().join(format!(
        "fyi-mcp-credential-contract-{}-{nonce}",
        std::process::id()
    ));
    std::fs::create_dir(&directory).unwrap();
    let directory = ScratchDirectory(directory);
    for (index, (first, second)) in [
        ("synthetic-private", "synthetic_private"),
        ("SYNTHETIC_PRIVATE", "synthetic_private"),
    ]
    .into_iter()
    .enumerate()
    {
        let database = directory.0.join(format!("private-{index}.db"));
        let output = tokio::time::timeout(
            Duration::from_secs(5),
            isolated_command()
                .env("DATABASE_URL", format!("sqlite://{}", database.display()))
                .env(
                    "FYI_MCP_REMOTE_INSTANCES",
                    format!("{first}=https://first.example,{second}=https://second.example"),
                )
                .env("FYI_MCP_REMOTE_READ", format!("{first},{second}"))
                .env("FYI_MCP_REMOTE_WRITE", format!("{first},{second}"))
                .env(
                    "FYI_MCP_REMOTE_BOT_TOKEN_SYNTHETIC_PRIVATE",
                    "synthetic-credential-never-disclose",
                )
                .stdin(Stdio::null())
                .output(),
        )
        .await
        .expect("invalid configuration must exit within five seconds")
        .unwrap();
        assert!(
            !output.status.success(),
            "ambiguous configuration was accepted"
        );
        assert!(output.stdout.is_empty());
        assert!(
            !database.exists(),
            "invalid configuration created a database"
        );
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.contains("distinct credential environment variables"));
        for private in [
            first,
            second,
            "first.example",
            "second.example",
            "synthetic-credential-never-disclose",
            database.to_str().unwrap(),
        ] {
            assert!(
                !stderr.contains(private),
                "startup error exposed private input"
            );
        }
    }
}

#[tokio::test]
async fn stdio_negotiates_supported_revisions_and_answers_ping() {
    for version in ["2025-06-18", "2025-03-26", "2099-01-01"] {
        let (responses, _) = exchange(&[
            initialize(version).to_string(),
            json!({"jsonrpc":"2.0", "method":"notifications/initialized"}).to_string(),
            json!({"jsonrpc":"2.0", "id":"ping-id", "method":"ping"}).to_string(),
        ])
        .await;
        assert_eq!(responses.len(), 2);
        let negotiated = if version == "2099-01-01" {
            "2025-06-18"
        } else {
            version
        };
        assert_eq!(responses[0]["result"]["protocolVersion"], negotiated);
        assert_eq!(
            responses[1],
            json!({"jsonrpc":"2.0", "id":"ping-id", "result":{}})
        );
    }
}

#[tokio::test]
async fn malformed_frames_have_stable_private_errors_and_recover() {
    let (responses, stderr) = exchange(&[
        "{private-synthetic-marker".into(),
        json!({"jsonrpc":"1.0", "id":2, "method":"ping"}).to_string(),
        json!({"jsonrpc":"2.0", "id":null, "method":"ping"}).to_string(),
        json!({"jsonrpc":"2.0", "id":true, "method":"ping"}).to_string(),
        json!({"jsonrpc":"2.0", "id":[], "method":"ping"}).to_string(),
        json!({"jsonrpc":"2.0", "id":1.5, "method":"ping"}).to_string(),
        json!({"jsonrpc":"2.0", "id":7, "method":17}).to_string(),
        json!({"jsonrpc":"2.0", "id":8, "method":"ping", "params":[]}).to_string(),
        json!({"jsonrpc":"2.0", "id":9, "method":"private-synthetic-marker"}).to_string(),
        json!({"jsonrpc":"2.0", "id":10, "method":"ping"}).to_string(),
    ])
    .await;
    assert_eq!(responses.len(), 10);
    assert_eq!(
        responses[0],
        json!({"jsonrpc":"2.0", "id":null,
        "error":{"code":-32700, "message":"Parse error"}})
    );
    for response in &responses[1..7] {
        assert_eq!(response["error"]["code"], -32600);
        assert!(response["id"].is_null());
    }
    assert_eq!(responses[7]["error"]["code"], -32602);
    assert_eq!(responses[7]["id"], 8);
    assert_eq!(responses[8]["error"]["code"], -32601);
    assert_eq!(responses[9]["result"], json!({}));
    assert!(!serde_json::to_string(&responses)
        .unwrap()
        .contains("private-synthetic-marker"));
    assert!(!stderr.contains("private-synthetic-marker"));
}

#[tokio::test]
async fn notifications_never_reply_or_execute_writes() {
    let (responses, _) = exchange(&[
        initialize("2025-06-18").to_string(),
        json!({"jsonrpc":"2.0", "method":"notifications/unknown", "params":{}}).to_string(),
        json!({"jsonrpc":"2.0", "method":"tools/call", "params":{
            "name":"create_request", "arguments":{"title":"synthetic", "body":"must not persist"}
        }})
        .to_string(),
        json!({"jsonrpc":"2.0", "id":2, "method":"tools/call", "params":{
            "name":"list_requests", "arguments":{}
        }})
        .to_string(),
        json!({"jsonrpc":"2.0", "id":3, "result":{}}).to_string(),
        json!({"jsonrpc":"2.0", "id":4, "method":"ping"}).to_string(),
    ])
    .await;
    assert_eq!(responses.len(), 3);
    assert_eq!(
        responses[1]["result"]["structuredContent"]["requests"],
        json!([])
    );
    assert_eq!(responses[2]["id"], 4);
}

#[tokio::test]
async fn legacy_batches_are_bounded_and_exclude_notification_responses() {
    let batch = json!([
        {"jsonrpc":"2.0", "id":"a", "method":"ping"},
        {"jsonrpc":"2.0", "method":"notifications/initialized"},
        {"jsonrpc":"2.0", "id":"b", "method":"ping"}
    ]);
    let (responses, _) = exchange(&[initialize("2025-03-26").to_string(), batch.to_string()]).await;
    assert_eq!(responses.len(), 2);
    assert_eq!(
        responses[1],
        json!([
            {"jsonrpc":"2.0", "id":"a", "result":{}},
            {"jsonrpc":"2.0", "id":"b", "result":{}}
        ])
    );
    let (responses, _) = exchange(&[initialize("2025-06-18").to_string(), batch.to_string()]).await;
    assert_eq!(responses[1]["error"]["code"], -32600);
}

#[tokio::test]
async fn oversized_stdio_frame_is_rejected_without_unbounded_drain() {
    let (responses, _) = exchange(&["x".repeat(1024 * 1024)]).await;
    assert_eq!(
        responses,
        vec![json!({
            "jsonrpc":"2.0", "id":null,
            "error":{"code":-32600,"message":"Request exceeds frame limit"}
        })]
    );
}

#[tokio::test]
async fn stock_profile_configuration_is_explicit_and_fails_closed_without_key() {
    let invalid = tokio::time::timeout(
        Duration::from_secs(5),
        isolated_command()
            .env("FYI_MCP_EPHEMERAL", "1")
            .env(
                "FYI_MCP_REMOTE_INSTANCES",
                "synthetic=https://source.invalid",
            )
            .env("FYI_MCP_REMOTE_STOCK_ALAVETELI", "unknown-private")
            .stdin(Stdio::null())
            .output(),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(!invalid.status.success());
    assert!(invalid.stdout.is_empty());
    let diagnostics = String::from_utf8(invalid.stderr).unwrap();
    assert!(diagnostics.contains("outside the allowlist"));
    assert!(!diagnostics.contains("unknown-private"));

    let mut child = isolated_command()
        .env("FYI_MCP_EPHEMERAL", "1")
        .env(
            "FYI_MCP_REMOTE_INSTANCES",
            "synthetic=https://source.invalid",
        )
        .env("FYI_MCP_REMOTE_READ", "synthetic")
        .env("FYI_MCP_REMOTE_STOCK_ALAVETELI", "synthetic")
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let message = json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"remote_get_request","arguments":{"instance_id":"synthetic","request_id":7001}}});
    stdin
        .write_all((message.to_string() + "\n").as_bytes())
        .await
        .unwrap();
    stdin.shutdown().await.unwrap();
    drop(stdin);
    let output = tokio::time::timeout(Duration::from_secs(10), child.wait_with_output())
        .await
        .unwrap()
        .unwrap();
    assert!(output.status.success());
    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(response["result"]["isError"], true);
    assert_eq!(
        response["result"]["structuredContent"]["error"]["kind"],
        "validation"
    );
    assert!(response["result"]["structuredContent"]["error"]["correlation_id"].is_null());
}
