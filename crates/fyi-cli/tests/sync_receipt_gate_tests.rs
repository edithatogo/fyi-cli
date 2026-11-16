//! The incomplete sync receipt path must fail before database or network I/O.
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

struct Workspace(PathBuf);

impl Workspace {
    fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "fyi-sync-receipt-gate-{}-{nonce}-{}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn sync_receipt_requests_fail_before_database_or_network_operations() {
    for operation in ["pull", "push"] {
        let workspace = Workspace::new();
        let receipt = workspace.0.join("receipt.json");
        let database = workspace.0.join("sync.db");
        fs::write(&receipt, b"existing evidence").unwrap();
        // This malformed URL cannot make a network request even on the old path.
        let output = Command::new(env!("CARGO_BIN_EXE_fyi-cli"))
            .args(["sync", operation, "--base-url", "not a URL", "--db"])
            .arg(&database)
            .arg("--receipt")
            .arg(&receipt)
            .output()
            .unwrap();
        assert!(!output.status.success(), "{operation}: {output:?}");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("sync receipt transport evidence"),
            "{operation}: {output:?}"
        );
        assert!(!database.exists(), "{operation} created the database");
        assert_eq!(fs::read(&receipt).unwrap(), b"existing evidence");
    }
}

#[test]
fn sync_without_receipt_reaches_existing_validation() {
    for operation in ["pull", "push"] {
        let workspace = Workspace::new();
        let output = Command::new(env!("CARGO_BIN_EXE_fyi-cli"))
            .args(["sync", operation, "--base-url", "not a URL", "--db"])
            .arg(workspace.0.join("sync.db"))
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("invalid FYI base URL"),
            "{operation}: {output:?}"
        );
    }
}

#[test]
fn json_output_reports_the_gate_without_creating_a_database() {
    let workspace = Workspace::new();
    let database = workspace.0.join("sync.db");
    let output = Command::new(env!("CARGO_BIN_EXE_fyi-cli"))
        .args([
            "--output-format",
            "json",
            "sync",
            "push",
            "--base-url",
            "not a URL",
            "--db",
        ])
        .arg(&database)
        .arg("--receipt")
        .arg(workspace.0.join("receipt.json"))
        .output()
        .unwrap();
    assert!(!output.status.success());
    let payload: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(payload["error"]["kind"], "sync");
    assert!(payload["error"]["message"]
        .as_str()
        .unwrap()
        .contains("sync receipt transport evidence"));
    assert!(!database.exists());
}

#[test]
fn offline_cli_verifier_rejects_an_ambiguous_legacy_sync_receipt() {
    use fyi_core::acquisition_receipts::canonical_json_bytes;
    use serde_json::{json, Value};

    let workspace = Workspace::new();
    let mut receipt: Value = serde_json::from_str(include_str!(
        "../../../conformance/acquisition/receipt-v1-python.json"
    ))
    .unwrap();
    receipt["command"] = json!("sync-pull");
    receipt["adapter"]["id"] = json!("alaveteli-sync");
    receipt["responses"] = json!([]);
    receipt["totals"] = json!({"requests": 0, "bytes": 0, "retries": 0});
    let mut unsigned = receipt.as_object().unwrap().clone();
    unsigned.remove("receipt_sha256");
    receipt["receipt_sha256"] = json!(fyi_core::provenance::sha256_hex(
        &canonical_json_bytes(&Value::Object(unsigned)).unwrap()
    ));
    let path = workspace.0.join("receipt.json");
    fs::write(&path, canonical_json_bytes(&receipt).unwrap()).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_fyi-cli"))
        .arg("verify-acquisition-receipt")
        .arg(&path)
        .output()
        .unwrap();
    assert!(!output.status.success(), "{output:?}");
    assert!(String::from_utf8_lossy(&output.stderr).contains("sync receipt transport evidence"));
}
