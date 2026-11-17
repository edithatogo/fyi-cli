//! Evidence destinations must be validated before any database or source I/O.
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

struct Workspace(PathBuf);

impl Workspace {
    fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        let directory = fs::canonicalize(std::env::temp_dir()).unwrap().join(format!(
            "fyi-fetch-preflight-{}-{}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&directory).unwrap();
        Self(directory)
    }

    fn run(&self, raw: &Path, result: &Path, receipt: &Path) -> Output {
        // Deliberately malformed: even the regression baseline cannot send HTTP.
        Command::new(env!("CARGO_BIN_EXE_fyi-cli"))
            .args(["fetch-request-page", "42", "--base-url", "not a URL"])
            .arg("--shared-rate-limit-db")
            .arg(self.0.join("pacing.db"))
            .arg("--raw-file")
            .arg(raw)
            .arg("--result-file")
            .arg(result)
            .arg("--receipt")
            .arg(receipt)
            .output()
            .unwrap()
    }

    fn assert_preflight_error(&self, output: &Output, expected: &str) {
        assert!(!output.status.success(), "{output:?}");
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains(expected), "expected {expected:?}: {error}");
        assert!(!error.contains("invalid FYI base URL"), "{error}");
        assert!(!self.0.join("pacing.db").exists());
        assert!(output.stdout.is_empty());
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn fetch_rejects_lexically_equivalent_evidence_paths_before_source_validation() {
    let workspace = Workspace::new();
    fs::create_dir(workspace.0.join("nested")).unwrap();
    let raw = workspace.0.join("response.raw");
    let result = workspace.0.join("nested/../response.raw");
    let receipt = workspace.0.join("receipt.json");
    let output = workspace.run(&raw, &result, &receipt);
    workspace.assert_preflight_error(&output, "paths must be distinct");
    assert!(!raw.exists() && !receipt.exists());
}

#[test]
fn fetch_rejects_nested_evidence_destinations() {
    let workspace = Workspace::new();
    let raw = workspace.0.join("response.raw");
    let output = workspace.run(
        &raw,
        &raw.join("result.json"),
        &workspace.0.join("receipt.json"),
    );
    workspace.assert_preflight_error(&output, "non-overlapping");
    assert!(!raw.exists());
}

#[test]
fn fetch_rejects_pacing_database_aliases_and_sidecars() {
    let workspace = Workspace::new();
    fs::create_dir(workspace.0.join("nested")).unwrap();
    for raw in [
        "nested/../pacing.db",
        "pacing.db-wal",
        "pacing.db-shm",
        "pacing.db-journal",
        "pacing.db-wal/response.raw",
    ] {
        let output = workspace.run(
            &workspace.0.join(raw),
            &workspace.0.join("result.json"),
            &workspace.0.join("receipt.json"),
        );
        let expected = if raw.contains("pacing.db-") {
            "database sidecar"
        } else {
            "paths must be distinct"
        };
        workspace.assert_preflight_error(&output, expected);
        assert!(!workspace.0.join(raw).exists());
    }
}

#[test]
fn fetch_preserves_existing_evidence_and_rejects_directories() {
    let workspace = Workspace::new();
    let raw = workspace.0.join("response.raw");
    let result = workspace.0.join("result.json");
    let receipt = workspace.0.join("receipt.json");
    fs::write(&receipt, b"previous immutable receipt").unwrap();
    let output = workspace.run(&raw, &result, &receipt);
    workspace.assert_preflight_error(&output, "already exists");
    assert_eq!(fs::read(&receipt).unwrap(), b"previous immutable receipt");
    assert!(!raw.exists() && !result.exists());

    fs::create_dir(&raw).unwrap();
    let output = workspace.run(&raw, &result, &workspace.0.join("new-receipt.json"));
    workspace.assert_preflight_error(&output, "already exists");
    assert!(raw.is_dir());
}

#[cfg(unix)]
#[test]
fn fetch_rejects_symlinked_evidence_ancestors() {
    let workspace = Workspace::new();
    let destination = workspace.0.join("destination");
    fs::create_dir(&destination).unwrap();
    std::os::unix::fs::symlink(&destination, workspace.0.join("alias")).unwrap();
    let output = workspace.run(
        &workspace.0.join("alias/response.raw"),
        &workspace.0.join("result.json"),
        &workspace.0.join("receipt.json"),
    );
    workspace.assert_preflight_error(&output, "symlink");
    assert!(fs::read_dir(destination).unwrap().next().is_none());
}

#[test]
fn valid_fresh_paths_reach_source_validation_without_database_creation() {
    let workspace = Workspace::new();
    let raw = workspace.0.join("response.raw");
    let result = workspace.0.join("result.json");
    let receipt = workspace.0.join("receipt.json");
    let output = workspace.run(&raw, &result, &receipt);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("invalid FYI base URL"));
    assert!(!workspace.0.join("pacing.db").exists());
    assert!(!raw.exists() && !result.exists() && !receipt.exists());
}
