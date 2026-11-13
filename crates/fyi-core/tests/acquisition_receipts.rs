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
