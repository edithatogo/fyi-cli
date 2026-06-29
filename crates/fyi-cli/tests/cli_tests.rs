use std::process::Command;

#[test]
fn test_help_message() {
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "--bin", "fyi-cli", "--", "--help"]);
    let output = cmd.output().expect("failed to execute process");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("fyi-cli"));
    assert!(stdout.contains("init-db"));
    assert!(stdout.contains("register-request"));
    assert!(stdout.contains("serve"));
    assert!(stdout.contains("mcp-server"));
    assert!(stdout.contains("tui"));
}

#[test]
fn test_init_db_defaults() {
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "--bin", "fyi-cli", "--", "init-db"]);
    let output = cmd.output().expect("failed to execute process");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success() || stdout.contains("Initialized"));
}
