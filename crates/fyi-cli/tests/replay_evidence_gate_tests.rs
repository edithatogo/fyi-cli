//! Invalid replay input and conflicting evidence must fail before network I/O.
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const SELECTION: &[u8] =
    include_bytes!("../../../tests/fixtures/internet_archive_replay/approved-selection.json");

struct Workspace(PathBuf);

impl Workspace {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = fs::canonicalize(std::env::temp_dir())
            .unwrap()
            .join(format!(
                "fyi-replay-cli-{}-{nonce}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn run(&self, receipt: &Path, extra: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_fyi-cli"))
            .args([
                "internet-archive-replay",
                "--allowed-target-host",
                "example.test",
                "--selection",
            ])
            .arg(self.0.join("selection.json"))
            .arg("--output-dir")
            .arg(self.0.join("payloads"))
            .arg("--responses-dir")
            .arg(self.0.join("responses"))
            .arg("--checkpoint")
            .arg(self.0.join("checkpoint.json"))
            .arg("--result")
            .arg(self.0.join("result.json"))
            .arg("--receipt")
            .arg(receipt)
            .args(extra)
            .output()
            .unwrap()
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn cli_rejects_unapproved_input_without_creating_outputs() {
    let workspace = Workspace::new();
    fs::write(workspace.0.join("selection.json"), b"{}\n").unwrap();
    let output = workspace.run(&workspace.0.join("receipt.json"), &[]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("invalid replay selection"));
    for name in [
        "payloads",
        "responses",
        "checkpoint.json",
        "result.json",
        "receipt.json",
    ] {
        assert!(!workspace.0.join(name).exists(), "unexpected {name}");
    }
}

#[test]
fn cli_preserves_preexisting_result_before_any_acquisition() {
    let workspace = Workspace::new();
    fs::write(workspace.0.join("selection.json"), SELECTION).unwrap();
    fs::write(workspace.0.join("result.json"), b"preserve").unwrap();
    let output = workspace.run(&workspace.0.join("receipt.json"), &[]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("result already exists"));
    assert_eq!(
        fs::read(workspace.0.join("result.json")).unwrap(),
        b"preserve"
    );
    assert!(!workspace.0.join("responses").exists());
    assert!(!workspace.0.join("checkpoint.json").exists());
}

#[test]
fn cli_rejects_path_aliases_and_nonfinite_budgets() {
    let workspace = Workspace::new();
    let selection = workspace.0.join("selection.json");
    fs::write(&selection, SELECTION).unwrap();
    let alias = workspace.run(&selection, &[]);
    assert!(!alias.status.success());
    assert!(String::from_utf8_lossy(&alias.stderr).contains("paths must be distinct"));
    assert_eq!(fs::read(&selection).unwrap(), SELECTION);
    let invalid = workspace.run(
        &workspace.0.join("receipt.json"),
        &["--max-runtime-seconds", "NaN"],
    );
    assert!(!invalid.status.success());
    assert!(!workspace.0.join("responses").exists());
}

#[test]
fn cli_requires_both_archive_paths_and_rejects_overlaps_before_network() {
    let workspace = Workspace::new();
    fs::write(workspace.0.join("selection.json"), SELECTION).unwrap();
    let receipt = workspace.0.join("receipt.json");
    let wacz = workspace.0.join("capture.wacz");
    let link = workspace.0.join("link.json");
    let missing = workspace.run(&receipt, &["--wacz", wacz.to_str().unwrap()]);
    assert!(!missing.status.success());
    assert!(!workspace.0.join("responses").exists());
    let overlap = workspace.run(
        &receipt,
        &[
            "--wacz",
            wacz.to_str().unwrap(),
            "--archive-link",
            workspace.0.join("selection.json").to_str().unwrap(),
        ],
    );
    assert!(!overlap.status.success());
    assert_eq!(
        fs::read(workspace.0.join("selection.json")).unwrap(),
        SELECTION
    );
    assert!(!workspace.0.join("responses").exists());
    assert!(!wacz.exists());
    assert!(!link.exists());
}

#[test]
fn cli_verifies_ordered_response_bytes_and_rejects_tampering() {
    use fyi_core::archive_format::{write_wacz, HttpResource, WarcWriter};
    use fyi_core::{acquisition_receipts::canonical_json_bytes, provenance::sha256_hex};
    use reqwest::{Url, Version};
    use serde_json::{json, Value};
    use time::OffsetDateTime;

    let workspace = Workspace::new();
    let responses = workspace.0.join("responses");
    fs::create_dir(&responses).unwrap();
    let raw = b"synthetic HTTP entity";
    let digest = sha256_hex(raw);
    let raw_path = responses.join(format!("000000-{digest}.raw"));
    fs::write(&raw_path, raw).unwrap();
    let mut receipt: Value = serde_json::from_str(include_str!(
        "../../../conformance/acquisition/receipt-v1-python.json"
    ))
    .unwrap();
    receipt["command"] = json!("internet-archive-replay");
    receipt["adapter"]["id"] = json!("internet-archive-replay");
    receipt["source"]["url"] = json!("https://web.archive.org/");
    receipt["responses"] = json!([{
        "url": "https://web.archive.org/web/20250101000000id_/https://example.test/",
        "status": 200, "bytes": raw.len(), "sha256": digest,
        "attempts": 1, "retry_delays_seconds": [],
    }]);
    receipt["totals"] = json!({"requests": 1, "bytes": raw.len(), "retries": 0});
    let mut unsigned = receipt.as_object().unwrap().clone();
    unsigned.remove("receipt_sha256");
    receipt["receipt_sha256"] = json!(sha256_hex(
        &canonical_json_bytes(&Value::Object(unsigned)).unwrap()
    ));
    let receipt_path = workspace.0.join("receipt.json");
    fs::write(&receipt_path, canonical_json_bytes(&receipt).unwrap()).unwrap();
    let warc_path = workspace.0.join("capture.warc.gz");
    let wacz_path = workspace.0.join("capture.wacz");
    let link_path = workspace.0.join("archive-link.json");
    let url = Url::parse(receipt["responses"][0]["url"].as_str().unwrap()).unwrap();
    let mut writer = WarcWriter::new(fs::File::create(&warc_path).unwrap());
    let record = writer
        .append(&HttpResource {
            kind: "wayback-replay",
            url: &url,
            status: 200,
            headers: &[],
            payload: raw,
            captured_at: OffsetDateTime::UNIX_EPOCH,
            http_version: Version::HTTP_11,
        })
        .unwrap();
    writer.finish().unwrap();
    let summary = write_wacz(&wacz_path, &warc_path, std::slice::from_ref(&record), &[]).unwrap();
    let mut link = json!({
        "schema": "urn:fyi-cli:wayback-wacz-link:1",
        "receipt_sha256": receipt["receipt_sha256"],
        "wacz_sha256": summary.sha256,
        "wacz_bytes": summary.bytes,
        "records": [{
            "response_index": 0,
            "record_id": record.warc_record_id,
            "offset": record.offset,
            "length": record.length,
        }],
    });
    link["link_sha256"] = json!(sha256_hex(&canonical_json_bytes(&link).unwrap()));
    let link_bytes = canonical_json_bytes(&link).unwrap();
    fs::write(&link_path, &link_bytes).unwrap();
    let verify = || {
        Command::new(env!("CARGO_BIN_EXE_fyi-cli"))
            .arg("verify-acquisition-receipt")
            .arg(&receipt_path)
            .arg("--responses-dir")
            .arg(&responses)
            .arg("--wacz")
            .arg(&wacz_path)
            .arg("--archive-link")
            .arg(&link_path)
            .output()
            .unwrap()
    };
    assert!(verify().status.success());
    link["records"][0]["record_id"] = json!("<urn:uuid:00000000-0000-0000-0000-000000000000>");
    let mut unsigned = link.as_object().unwrap().clone();
    unsigned.remove("link_sha256");
    link["link_sha256"] = json!(sha256_hex(
        &canonical_json_bytes(&Value::Object(unsigned)).unwrap()
    ));
    fs::write(&link_path, canonical_json_bytes(&link).unwrap()).unwrap();
    let output = verify();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("archive link record mismatch"));
    fs::write(&link_path, link_bytes).unwrap();
    fs::write(&raw_path, b"modified HTTP entity!").unwrap();
    let output = verify();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("mismatch"));
}
