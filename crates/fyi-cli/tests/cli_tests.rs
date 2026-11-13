use std::process::Command;

#[test]
fn test_help_message() {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_fyi-cli"));
    cmd.args(["--help"]);
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
    let work_dir =
        std::env::temp_dir().join(format!("fyi-cli-default-init-{}", std::process::id()));
    std::fs::create_dir(&work_dir).expect("create isolated test directory");
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_fyi-cli"));
    cmd.current_dir(&work_dir).args(["init-db"]);
    let output = cmd.output().expect("failed to execute process");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    assert!(stdout.contains("Initialized"));
    assert!(work_dir.join("fyi_system.db").exists());
    std::fs::remove_dir_all(work_dir).expect("remove isolated test directory");
}
