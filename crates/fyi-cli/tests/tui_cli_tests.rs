//! Launch contracts for the real executable, with no terminal or database side effects.
use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn tui_rejects_redirected_io_before_opening_a_database() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("fyi-tui-cli-{}-{nonce}", std::process::id()));
    fs::create_dir(&path).unwrap();
    for arguments in [
        vec!["--db", "PRIVATE-PATH.db", "tui"],
        vec!["tui", "--db", "PRIVATE-PATH.db", "-vv"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_fyi-cli"))
            .current_dir(&path)
            .env("RUST_LOG", "trace")
            .env("TERM", "xterm-256color")
            .args(arguments)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert_eq!(
            String::from_utf8(output.stderr).unwrap(),
            "TUI failed: An interactive terminal is required.\n"
        );
        assert_eq!(fs::read_dir(&path).unwrap().count(), 0);
    }
    fs::remove_dir(path).unwrap();
}

#[test]
fn tui_json_output_is_an_explicit_static_error() {
    let output = Command::new(env!("CARGO_BIN_EXE_fyi-cli"))
        .args(["--output-format", "json", "tui"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(
        error,
        serde_json::json!({"error": {
            "kind": "tui",
            "message": "The TUI requires text output."
        }})
    );
}

#[test]
fn tui_help_identifies_the_local_read_only_snapshot() {
    let output = Command::new(env!("CARGO_BIN_EXE_fyi-cli"))
        .args(["tui", "--help"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).unwrap();
    assert!(help.contains("read-only local request snapshot"));
    assert!(help.contains("--db"));
    assert!(output.stderr.is_empty());
}

#[cfg(target_os = "linux")]
#[test]
fn tui_real_terminal_launch_privacy_resize_and_restoration() {
    let script =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tui_terminal.py");
    let output = Command::new("python3")
        .arg(script)
        .arg(env!("CARGO_BIN_EXE_fyi-cli"))
        .output()
        .expect("Python 3 is required for the Linux standard-library PTY contract");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("PTY acceptance:"));
}
