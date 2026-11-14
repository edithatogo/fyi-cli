//! Actual-process write durability contracts. All identities are synthetic.

use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;

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
