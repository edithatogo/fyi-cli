//! Cross-language golden tests for Python acquisition-receipt-v1.
use fyi_core::acquisition_receipts::{canonical_json_bytes, verify_receipt};
use serde_json::{json, Value};

const GOLDEN: &str = include_str!("../../../conformance/acquisition/receipt-v1-python.json");

fn golden() -> Value {
    serde_json::from_str(GOLDEN).unwrap()
}

#[test]
fn verifies_python_v1_receipt_and_canonical_hash() {
    let receipt = golden();
    verify_receipt(&receipt).unwrap();
    let mut unsigned = receipt.as_object().unwrap().clone();
    unsigned.remove("receipt_sha256");
    let canonical = canonical_json_bytes(&Value::Object(unsigned)).unwrap();
    assert_eq!(
        fyi_core::provenance::sha256_hex(&canonical),
        receipt["receipt_sha256"].as_str().unwrap(),
    );
}

#[test]
fn rejects_modified_receipt_and_inconsistent_response_totals() {
    let mut receipt = golden();
    receipt["command"] = json!("changed");
    assert!(verify_receipt(&receipt).is_err());
    let mut receipt = golden();
    receipt["totals"]["bytes"] = json!(999);
    assert!(verify_receipt(&receipt).is_err());
}

#[test]
fn rejects_sensitive_urls_even_with_recalculated_digest() {
    let mut receipt = golden();
    receipt["source"]["url"] = json!("https://user:password@example.test/search?token=secret");
    let mut unsigned = receipt.as_object().unwrap().clone();
    unsigned.remove("receipt_sha256");
    receipt["receipt_sha256"] = json!(fyi_core::provenance::sha256_hex(
        &canonical_json_bytes(&Value::Object(unsigned)).unwrap()
    ));
    assert!(verify_receipt(&receipt).is_err());
}

#[test]
fn canonicalization_is_order_independent() {
    let a: Value = serde_json::from_str(r#"{"b":2,"a":{"z":1,"c":3}}"#).unwrap();
    let b: Value = serde_json::from_str(r#"{"a":{"c":3,"z":1},"b":2}"#).unwrap();
    assert_eq!(
        canonical_json_bytes(&a).unwrap(),
        canonical_json_bytes(&b).unwrap()
    );
}

#[test]
fn atomic_receipt_writer_is_idempotent_and_rejects_replacement() {
    use fyi_core::acquisition_receipts::write_receipt_atomic;
    let root = std::env::temp_dir().join(format!(
        "fyi-receipt-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let path = root.join("receipts").join("receipt.json");
    let original = golden();
    write_receipt_atomic(&path, &original).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(bytes, canonical_json_bytes(&original).unwrap());

    // Identical writes are safe; a different *valid* receipt must not replace
    // existing immutable evidence, even if the two writes race.
    write_receipt_atomic(&path, &original).unwrap();
    let mut different = original.clone();
    different["command"] = json!("capture");
    resign(&mut different);
    assert!(write_receipt_atomic(&path, &different).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), bytes);

    let entries = std::fs::read_dir(path.parent().unwrap())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect::<Vec<_>>();
    assert_eq!(entries.len(), 1, "temporary evidence files must be removed");
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn atomic_writer_refuses_invalid_existing_evidence() {
    use fyi_core::acquisition_receipts::write_receipt_atomic;
    let root = std::env::temp_dir().join(format!(
        "fyi-receipt-corrupt-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("receipt.json");
    std::fs::write(&path, b"not a valid receipt").unwrap();
    assert!(write_receipt_atomic(&path, &golden()).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"not a valid receipt");
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn concurrent_identical_writes_preserve_one_valid_receipt() {
    use fyi_core::acquisition_receipts::write_receipt_atomic;
    use std::sync::Arc;
    let root = std::env::temp_dir().join(format!(
        "fyi-receipt-race-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let path = Arc::new(root.join("receipt.json"));
    let mut workers = Vec::new();
    for _ in 0..8 {
        let path = Arc::clone(&path);
        workers.push(std::thread::spawn(move || {
            write_receipt_atomic(&path, &golden())
        }));
    }
    for worker in workers {
        worker.join().unwrap().unwrap();
    }
    let stored: Value = serde_json::from_slice(&std::fs::read(&*path).unwrap()).unwrap();
    verify_receipt(&stored).unwrap();
    assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
    std::fs::remove_dir_all(&root).unwrap();
}

fn resign(receipt: &mut Value) {
    let object = receipt.as_object_mut().unwrap();
    object.remove("receipt_sha256");
    let unsigned = Value::Object(object.clone());
    receipt["receipt_sha256"] = json!(fyi_core::provenance::sha256_hex(
        &canonical_json_bytes(&unsigned).unwrap()
    ));
}

#[test]
fn receipt_verifier_rejects_extra_schema_fields_even_if_resigned() {
    let mut receipt = golden();
    receipt["unexpected_source"] = json!("injected");
    resign(&mut receipt);
    assert!(verify_receipt(&receipt).is_err());
    let mut receipt = golden();
    receipt["adapter"]["debug_secret"] = json!("should not be recorded");
    resign(&mut receipt);
    assert!(verify_receipt(&receipt).is_err());
}


#[cfg(unix)]
#[test]
fn writer_does_not_accept_symlink_as_committed_evidence() {
    use fyi_core::acquisition_receipts::write_receipt_atomic;
    use std::os::unix::fs::symlink;
    let root = std::env::temp_dir().join(format!(
        "fyi-receipt-symlink-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let real = root.join("real.json");
    write_receipt_atomic(&real, &golden()).unwrap();
    let link = root.join("link.json");
    symlink(&real, &link).unwrap();
    assert!(write_receipt_atomic(&link, &golden()).is_err());
    assert!(std::fs::symlink_metadata(&link)
        .unwrap()
        .file_type()
        .is_symlink());
    std::fs::remove_dir_all(&root).unwrap();
}
