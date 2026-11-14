//! Offline filesystem/process evidence. This does not simulate storage power loss.
use fyi_core::acquisition_receipts::{canonical_json_bytes, write_receipt_atomic};
use fyi_core::provenance::sha256_hex;
use fyi_core::receipt_wire::read_receipt;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const GOLDEN: &str = include_str!("../../../conformance/acquisition/receipt-v1-python.json");
static NEXT: AtomicU64 = AtomicU64::new(0);

struct Sandbox(PathBuf);

impl Sandbox {
    fn new() -> Self {
        let serial = NEXT.fetch_add(1, Ordering::Relaxed);
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "fyi-receipt-portability-{}-{stamp}-{serial}-λ",
            std::process::id()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn receipt(&self) -> PathBuf {
        self.0.join("receipt.json")
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn receipt(command: &str) -> Value {
    let mut value: Value = serde_json::from_str(GOLDEN).unwrap();
    value["command"] = json!(command);
    value.as_object_mut().unwrap().remove("receipt_sha256");
    let digest = sha256_hex(&canonical_json_bytes(&value).unwrap());
    value["receipt_sha256"] = json!(digest);
    value
}

fn wait_until(mut ready: impl FnMut() -> bool, deadline: Instant) {
    while !ready() {
        assert!(Instant::now() < deadline, "publication worker timed out");
        std::thread::sleep(Duration::from_millis(10));
    }
}

struct Workers(Vec<Child>);

impl Drop for Workers {
    fn drop(&mut self) {
        for child in &mut self.0 {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn run_workers(root: &Path, variants: &[&str], abrupt: bool) -> Vec<ExitStatus> {
    let deadline = Instant::now() + Duration::from_secs(45);
    let mut workers = Workers(Vec::new());
    for (index, variant) in variants.iter().enumerate() {
        workers.0.push(
            Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "publication_worker", "--ignored", "--nocapture"])
                .env("FYI_RECEIPT_TEST_ROOT", root)
                .env("FYI_RECEIPT_TEST_INDEX", index.to_string())
                .env("FYI_RECEIPT_TEST_VARIANT", variant)
                .env("FYI_RECEIPT_TEST_ABRUPT", if abrupt { "1" } else { "0" })
                .stdout(Stdio::null())
                .stderr(Stdio::inherit())
                .spawn()
                .unwrap(),
        );
    }
    wait_until(
        || (0..variants.len()).all(|index| root.join(format!("ready-{index}")).exists()),
        deadline,
    );
    fs::write(root.join("start"), b"start").unwrap();
    let mut results = Vec::new();
    for child in &mut workers.0 {
        let mut status = None;
        wait_until(
            || {
                status = child.try_wait().unwrap();
                status.is_some()
            },
            deadline,
        );
        results.push(status.unwrap());
    }
    results
}

#[test]
#[ignore = "subprocess entry point, exercised explicitly by the parent tests"]
fn publication_worker() {
    let root = PathBuf::from(std::env::var_os("FYI_RECEIPT_TEST_ROOT").unwrap());
    let index = std::env::var("FYI_RECEIPT_TEST_INDEX").unwrap();
    let variant = std::env::var("FYI_RECEIPT_TEST_VARIANT").unwrap();
    fs::write(root.join(format!("ready-{index}")), b"ready").unwrap();
    wait_until(
        || root.join("start").exists(),
        Instant::now() + Duration::from_secs(45),
    );
    let outcome = write_receipt_atomic(&root.join("receipt.json"), &receipt(&variant));
    let report = match &outcome {
        Ok(()) => "committed".to_owned(),
        Err(error) => format!("rejected: {error:#}"),
    };
    fs::write(root.join(format!("result-{index}")), report).unwrap();
    if std::env::var("FYI_RECEIPT_TEST_ABRUPT").unwrap() == "1" {
        outcome.unwrap();
        // Deliberately bypass Rust destructors after the real writer returned.
        // This is process-exit recovery evidence, not a power-cut simulation.
        std::process::exit(73);
    }
}

fn assert_no_temporary_files(root: &Path) {
    for entry in fs::read_dir(root).unwrap() {
        let name = entry.unwrap().file_name();
        assert!(!name.to_string_lossy().ends_with(".tmp"), "{name:?}");
    }
}

#[test]
fn independent_processes_publish_identical_receipts_idempotently() {
    let root = Sandbox::new();
    let statuses = run_workers(&root.0, &["same", "same", "same", "same"], false);
    assert!(statuses.iter().all(ExitStatus::success));
    for index in 0..statuses.len() {
        let report = fs::read_to_string(root.0.join(format!("result-{index}"))).unwrap();
        assert_eq!(report, "committed");
    }
    let stored = read_receipt(fs::File::open(root.receipt()).unwrap()).unwrap();
    assert_eq!(stored, receipt("same"));
    assert_no_temporary_files(&root.0);
}

#[test]
fn independent_processes_cannot_replace_each_others_evidence() {
    let root = Sandbox::new();
    let variants = ["one", "two", "three", "four"];
    let statuses = run_workers(&root.0, &variants, false);
    assert!(statuses.iter().all(ExitStatus::success));
    let mut winners = Vec::new();
    for (index, variant) in variants.iter().enumerate() {
        let result = fs::read_to_string(root.0.join(format!("result-{index}"))).unwrap();
        if result == "committed" {
            winners.push(*variant);
        } else {
            assert!(
                result.contains("different evidence already exists"),
                "{result}"
            );
        }
    }
    assert_eq!(winners.len(), 1);
    let stored = fs::read(root.receipt()).unwrap();
    assert_eq!(stored, canonical_json_bytes(&receipt(winners[0])).unwrap());
    assert!(read_receipt(stored.as_slice()).is_ok());
    assert_no_temporary_files(&root.0);
}

#[test]
fn committed_evidence_survives_abrupt_writer_exit_and_is_retryable() {
    let root = Sandbox::new();
    let statuses = run_workers(&root.0, &["abrupt"], true);
    assert_eq!(statuses[0].code(), Some(73));
    let before = fs::read(root.receipt()).unwrap();
    let stored = read_receipt(before.as_slice()).unwrap();
    assert_eq!(stored, receipt("abrupt"));
    write_receipt_atomic(&root.receipt(), &stored).unwrap();
    assert_eq!(fs::read(root.receipt()).unwrap(), before);
    assert_no_temporary_files(&root.0);
}

#[test]
fn simulated_orphan_tempfile_is_preserved_but_never_promoted() {
    let root = Sandbox::new();
    let orphan = root.0.join(".receipt.json.999999999.0.tmp");
    let partial = b"{\"schema\":\"interrupted";
    fs::write(&orphan, partial).unwrap();
    assert!(!root.receipt().exists());
    write_receipt_atomic(&root.receipt(), &receipt("fresh")).unwrap();
    assert_eq!(fs::read(&orphan).unwrap(), partial);
    let stored = read_receipt(fs::File::open(root.receipt()).unwrap()).unwrap();
    assert_eq!(stored, receipt("fresh"));
    assert_eq!(fs::read_dir(&root.0).unwrap().count(), 2);
}

#[test]
fn directory_destination_and_parent_file_collisions_preserve_existing_entries() {
    let root = Sandbox::new();
    fs::create_dir(root.receipt()).unwrap();
    assert!(write_receipt_atomic(&root.receipt(), &receipt("new")).is_err());
    assert!(root.receipt().is_dir());
    let blocker = root.0.join("parent-file");
    fs::write(&blocker, b"existing-parent").unwrap();
    assert!(write_receipt_atomic(&blocker.join("receipt.json"), &receipt("new")).is_err());
    assert_eq!(fs::read(&blocker).unwrap(), b"existing-parent");
    assert_no_temporary_files(&root.0);
}

#[cfg(any(unix, windows))]
#[test]
fn dangling_and_existing_file_symlinks_are_rejected_without_modification() {
    #[cfg(unix)]
    use std::os::unix::fs::symlink as file_symlink;
    #[cfg(windows)]
    use std::os::windows::fs::symlink_file as file_symlink;
    let root = Sandbox::new();
    let target = root.0.join("target.json");
    let link = root.0.join("link.json");
    // The hosted platform must support the test; failure is not silently skipped.
    file_symlink(&target, &link).unwrap();
    assert!(write_receipt_atomic(&link, &receipt("new")).is_err());
    assert!(!target.exists());
    write_receipt_atomic(&target, &receipt("same")).unwrap();
    let before = fs::read(&target).unwrap();
    assert!(write_receipt_atomic(&link, &receipt("same")).is_err());
    assert!(fs::symlink_metadata(&link).unwrap().file_type().is_symlink());
    assert_eq!(fs::read(&target).unwrap(), before);
    assert_no_temporary_files(&root.0);
}

#[cfg(unix)]
#[test]
fn newly_published_receipt_is_owner_private_on_posix() {
    use std::os::unix::fs::PermissionsExt;
    let root = Sandbox::new();
    write_receipt_atomic(&root.receipt(), &receipt("private")).unwrap();
    let mode = fs::metadata(root.receipt()).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o600);
}
