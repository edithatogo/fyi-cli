//! Actual-process write durability contracts. All identities are synthetic.

use serde_json::{json, Value};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

struct Directory(std::path::PathBuf);

impl Directory {
    fn new() -> Self {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "fyi-mcp-process-journal-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir(&directory).unwrap();
        Self(directory)
    }
}

impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct Server {
    child: Child,
    input: ChildStdin,
    output: Lines<BufReader<ChildStdout>>,
    diagnostics: tokio::task::JoinHandle<Vec<u8>>,
}

impl Server {
    fn start(state_directory: &std::path::Path) -> Self {
        let mut child = write_server()
            .env("FYI_MCP_REMOTE_WRITE_STATE_DIR", state_directory)
            .stdin(Stdio::piped())
            .spawn()
            .unwrap();
        let input = child.stdin.take().unwrap();
        let output = BufReader::new(child.stdout.take().unwrap()).lines();
        let mut stderr = child.stderr.take().unwrap();
        let diagnostics = tokio::spawn(async move {
            let mut bytes = Vec::new();
            stderr.read_to_end(&mut bytes).await.unwrap();
            bytes
        });
        Self {
            child,
            input,
            output,
            diagnostics,
        }
    }

    async fn call(&mut self, id: u64, method: &str, params: Value) -> Value {
        let frame = serde_json::to_string(
            &json!({"jsonrpc":"2.0", "id":id, "method":method, "params":params}),
        )
        .unwrap()
            + "\n";
        self.input.write_all(frame.as_bytes()).await.unwrap();
        self.input.flush().await.unwrap();
        let response = tokio::time::timeout(Duration::from_secs(5), self.output.next_line())
            .await
            .unwrap()
            .unwrap()
            .expect("server response");
        let response: Value = serde_json::from_str(&response).unwrap();
        assert_eq!(response["id"], id);
        response
    }

    async fn stop(mut self) -> String {
        self.input.shutdown().await.unwrap();
        drop(self.input);
        assert!(
            tokio::time::timeout(Duration::from_secs(5), self.child.wait())
                .await
                .unwrap()
                .unwrap()
                .success()
        );
        String::from_utf8(self.diagnostics.await.unwrap()).unwrap()
    }
}

fn prepared_arguments() -> Value {
    json!({"instance_id":"synthetic", "phase":"prepare", "idempotency_key":"synthetic-private-replay-key", "title":"synthetic-private-title", "body":"synthetic-private-body Café 東京", "external_user_name":"synthetic-private-name", "external_url":"https://example.invalid/synthetic-private-url"})
}

fn write_server() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_fyi-mcp"));
    for (name, _) in std::env::vars_os() {
        if name.to_string_lossy().starts_with("FYI_MCP_") || name == "DATABASE_URL" {
            command.env_remove(name);
        }
    }
    command
        .env("FYI_MCP_EPHEMERAL", "1")
        .env("FYI_MCP_REMOTE_INSTANCES", "synthetic=https://127.0.0.1")
        .env("FYI_MCP_REMOTE_WRITE", "synthetic")
        .env("RUST_LOG", "warn")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    command
}

#[tokio::test]
async fn remote_writes_require_persistent_state_before_startup() {
    let output = tokio::time::timeout(Duration::from_secs(5), write_server().output())
        .await
        .expect("invalid startup must exit within five seconds")
        .unwrap();
    assert!(
        !output.status.success(),
        "remote writes started with only process-local replay protection"
    );
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("FYI_MCP_REMOTE_WRITE_STATE_DIR is required"));
    assert!(!stderr.contains("127.0.0.1"));
}

#[tokio::test]
async fn actual_process_restart_preserves_attempts_and_redacts_persistent_audit() {
    let directory = Directory::new();
    let state = directory.0.join("state");
    let mut first = Server::start(&state);
    let initialized = first.call(1, "initialize", json!({"protocolVersion":"2025-06-18", "capabilities":{}, "clientInfo":{"name":"synthetic", "version":"1"}})).await;
    assert_eq!(initialized["result"]["protocolVersion"], "2025-06-18");
    let prepared = first
        .call(
            2,
            "tools/call",
            json!({"name":"remote_create_request", "arguments":prepared_arguments()}),
        )
        .await;
    assert_eq!(
        prepared["result"]["structuredContent"]["status"],
        "prepared"
    );
    let token = prepared["result"]["structuredContent"]["confirmation_token"]
        .as_str()
        .unwrap()
        .to_string();
    let commit = first.call(3, "tools/call", json!({"name":"remote_create_request", "arguments":{"instance_id":"synthetic", "phase":"commit", "idempotency_key":"synthetic-private-replay-key", "confirmation_token":token}})).await;
    // The unchanged SyncClient rejects the literal loopback source at construction.
    // No HTTP call or live remote write is made. The reserved failure must persist.
    assert_eq!(commit["result"]["isError"], true);
    assert!(commit.to_string().contains("remote operation failed"));
    let first_diagnostics = first.stop().await;

    let mut restarted = Server::start(&state);
    let replay = restarted
        .call(
            4,
            "tools/call",
            json!({"name":"remote_create_request", "arguments":prepared_arguments()}),
        )
        .await;
    assert_eq!(replay["result"]["isError"], true);
    assert!(replay
        .to_string()
        .contains("durable write attempt already exists"));
    let rejected = restarted.call(5, "tools/call", json!({"name":"remote_create_request", "arguments":{"instance_id":"synthetic", "phase":"commit", "idempotency_key":"synthetic-private-replay-key", "confirmation_token":token}})).await;
    assert_eq!(rejected["result"]["isError"], true);
    let invalid = restarted.call(6, "tools/call", json!({"name":"remote_create_request", "arguments":{"instance_id":"synthetic", "unexpected":"synthetic-private-rejected-input"}})).await;
    assert_eq!(invalid["error"]["code"], -32602);
    let second_diagnostics = restarted.stop().await;

    let options = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(state.join("remote-write-v1.sqlite3"))
        .read_only(true);
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .unwrap();
    let events: Vec<String> =
        sqlx::query_scalar("SELECT event FROM mcp_write_audit ORDER BY sequence")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(
        events,
        ["prepare", "commit", "failure", "replay", "reject", "reject"]
    );
    let outcomes: Vec<String> = sqlx::query_scalar("SELECT outcome FROM mcp_write_attempts")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(outcomes, ["uncertain"]);
    pool.close().await;
    let persisted = std::fs::read(state.join("remote-write-v1.sqlite3")).unwrap();
    for marker in [
        "synthetic-private",
        token.as_str(),
        directory.0.to_str().unwrap(),
    ] {
        assert!(!first_diagnostics.contains(marker));
        assert!(!second_diagnostics.contains(marker));
        assert!(!persisted
            .windows(marker.len())
            .any(|bytes| bytes == marker.as_bytes()));
    }
}

#[tokio::test]
async fn unusable_state_fails_before_the_application_database_is_created() {
    let directory = Directory::new();
    let state = directory.0.join("synthetic-private-not-a-directory");
    std::fs::write(&state, b"synthetic-private-storage").unwrap();
    let database = directory.0.join("application.sqlite");
    let output = tokio::time::timeout(
        Duration::from_secs(5),
        write_server()
            .env("FYI_MCP_EPHEMERAL", "0")
            .env(
                "DATABASE_URL",
                format!("sqlite://{}?mode=rwc", database.display()),
            )
            .env("FYI_MCP_REMOTE_WRITE_STATE_DIR", &state)
            .output(),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(!database.exists());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("remote write journal is unavailable"));
    assert!(!stderr.contains("synthetic-private"));
    assert!(!stderr.contains(directory.0.to_str().unwrap()));
}
