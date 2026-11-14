//! Test the actual file-to-verdict boundary, not a pre-parsed JSON map.
use fyi_core::acquisition_receipts::canonical_json_bytes;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

const FIXTURE: &str = include_str!("../../../conformance/acquisition/receipt-v1-python.json");
const MAX_BYTES: usize = 16 * 1024 * 1024;
static NEXT_FILE: AtomicU64 = AtomicU64::new(0);

struct EvidenceFile(PathBuf);

impl EvidenceFile {
    fn new(bytes: &[u8]) -> Self {
        let path = std::env::temp_dir().join(format!(
            "fyi-receipt-wire-{}-{}.json",
            std::process::id(),
            NEXT_FILE.fetch_add(1, Ordering::Relaxed)
        ));
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        std::io::Write::write_all(&mut file, bytes).unwrap();
        Self(path)
    }

    fn verify(&self) -> Output {
        Command::new(env!("CARGO_BIN_EXE_fyi-cli"))
            .arg("verify-acquisition-receipt")
            .arg(&self.0)
            .output()
            .unwrap()
    }
}

impl Drop for EvidenceFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn compact_fixture() -> String {
    let value: serde_json::Value = serde_json::from_str(FIXTURE).unwrap();
    String::from_utf8(canonical_json_bytes(&value).unwrap()).unwrap()
}

fn reject_duplicate(needle: &str, replacement: &str) {
    let original = compact_fixture();
    assert!(original.contains(needle));
    let ambiguous = original.replacen(needle, replacement, 1);
    let file = EvidenceFile::new(ambiguous.as_bytes());
    let output = file.verify();
    assert!(!output.status.success(), "ambiguous receipt was accepted");
    assert!(output.stdout.is_empty());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("duplicate"), "{error}");
    assert!(!error.contains("synthetic-discarded-marker"));
    assert_eq!(std::fs::read(&file.0).unwrap(), ambiguous.as_bytes());
}

#[test]
fn cli_rejects_conflicting_top_level_fields_before_hashing() {
    reject_duplicate(
        "\"command\":\"discover\"",
        "\"command\":\"synthetic-discarded-marker\",\"command\":\"discover\"",
    );
}

#[test]
fn cli_rejects_identical_duplicate_fields() {
    reject_duplicate(
        "\"command\":\"discover\"",
        "\"command\":\"discover\",\"command\":\"discover\"",
    );
}

#[test]
fn cli_rejects_escape_equivalent_duplicate_names() {
    reject_duplicate(
        "\"command\":\"discover\"",
        "\"\\u0063ommand\":\"synthetic-discarded-marker\",\"command\":\"discover\"",
    );
}

#[test]
fn cli_rejects_nested_adapter_duplicates() {
    reject_duplicate(
        "\"id\":\"alaveteli-search-feed\"",
        "\"id\":\"synthetic-discarded-marker\",\"id\":\"alaveteli-search-feed\"",
    );
}

#[test]
fn cli_rejects_duplicates_inside_response_arrays() {
    reject_duplicate("\"attempts\":2", "\"attempts\":99,\"attempts\":2");
}

#[test]
fn cli_rejects_duplicates_inside_free_form_request_bounds() {
    reject_duplicate("\"max_pages\":1", "\"max_pages\":99,\"max_pages\":1");
}

#[test]
fn cli_rejects_receipt_files_larger_than_the_document_limit() {
    let mut bytes = FIXTURE.as_bytes().to_vec();
    bytes.resize(MAX_BYTES + 1, b' ');
    let file = EvidenceFile::new(&bytes);
    let output = file.verify();
    assert!(!output.status.success(), "oversized receipt was accepted");
    assert!(String::from_utf8_lossy(&output.stderr).contains("limit"));
    assert_eq!(std::fs::read(&file.0).unwrap(), bytes);
}

#[test]
fn cli_still_accepts_noncanonical_whitespace_and_key_order() {
    let file = EvidenceFile::new(FIXTURE.as_bytes());
    let output = file.verify();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(std::fs::read(&file.0).unwrap(), FIXTURE.as_bytes());
}
