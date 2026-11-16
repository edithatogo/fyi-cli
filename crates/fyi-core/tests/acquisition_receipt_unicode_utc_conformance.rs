//! Cross-runtime Unicode, escaping, key-ordering, and RFC 3339 UTC timestamp receipt cases.
use fyi_core::acquisition_receipts::{canonical_json_bytes, verify_receipt};
use fyi_core::provenance::sha256_hex;
use serde_json::{json, Value};

const GOLDEN: &str = include_str!("../../../conformance/acquisition/receipt-v1-python.json");
const CASES: &str =
    include_str!("../../../conformance/acquisition/receipt-v1-unicode-utc-cases.json");

#[test]
fn canonical_json_matches_python_unicode_escaping_and_key_ordering() {
    let corpus: Value = serde_json::from_str(CASES).unwrap();
    assert_eq!(corpus["schema_version"], "1.0.0");
    let cases = corpus["canonical_cases"].as_array().unwrap();
    assert!(!cases.is_empty());

    for case in cases {
        let name = case["name"].as_str().unwrap();
        let canonical = canonical_json_bytes(&case["value"]).unwrap();
        let canonical_str = std::str::from_utf8(&canonical).unwrap();
        assert_eq!(
            canonical_str,
            case["expected_canonical"].as_str().unwrap(),
            "canonical JSON mismatch for {name}"
        );
        assert_eq!(
            sha256_hex(&canonical),
            case["expected_sha256"].as_str().unwrap(),
            "canonical SHA-256 mismatch for {name}"
        );
    }
}

#[test]
fn timestamp_semantics_enforce_rfc3339_and_utc_chronology() {
    let corpus: Value = serde_json::from_str(CASES).unwrap();
    assert_eq!(corpus["schema_version"], "1.0.0");
    let cases = corpus["timestamp_cases"].as_array().unwrap();
    assert!(!cases.is_empty());

    for case in cases {
        let name = case["name"].as_str().unwrap();
        let mut receipt: Value = serde_json::from_str(GOLDEN).unwrap();
        receipt["started_at"] = case["started_at"].clone();
        receipt["completed_at"] = case["completed_at"].clone();
        receipt.as_object_mut().unwrap().remove("receipt_sha256");
        let digest = sha256_hex(&canonical_json_bytes(&receipt).unwrap());
        receipt["receipt_sha256"] = json!(digest);

        let expected =
            case["schema_valid"].as_bool().unwrap() && case["chronology_valid"].as_bool().unwrap();
        let result = verify_receipt(&receipt);
        assert_eq!(
            result.is_ok(),
            expected,
            "timestamp case {name}: {result:?}"
        );
    }
}
