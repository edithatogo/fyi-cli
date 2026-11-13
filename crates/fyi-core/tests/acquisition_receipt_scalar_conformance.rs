//! Receipt scalar cases shared with the packaged Python JSON Schema validator.
use fyi_core::acquisition_receipts::{canonical_json_bytes, verify_receipt};
use fyi_core::provenance::sha256_hex;
use serde_json::{json, Value};

const GOLDEN: &str = include_str!("../../../conformance/acquisition/receipt-v1-python.json");
const CASES: &str = include_str!("../../../conformance/acquisition/receipt-v1-scalar-cases.json");

fn check_cases(pointer: &str) {
    let corpus: Value = serde_json::from_str(CASES).unwrap();
    assert_eq!(corpus["schema_version"], "1.0.0");
    let mut checked = 0;
    for case in corpus["cases"].as_array().unwrap() {
        if case["pointer"].as_str().unwrap() != pointer {
            continue;
        }
        let mut receipt: Value = serde_json::from_str(GOLDEN).unwrap();
        *receipt.pointer_mut(pointer).unwrap() = case["value"].clone();
        receipt.as_object_mut().unwrap().remove("receipt_sha256");
        let digest = sha256_hex(&canonical_json_bytes(&receipt).unwrap());
        receipt["receipt_sha256"] = json!(digest);
        let result = verify_receipt(&receipt);
        assert_eq!(
            result.is_ok(),
            case["valid"].as_bool().unwrap(),
            "case {}: {result:?}",
            case["name"],
        );
        checked += 1;
    }
    assert!(checked > 0, "conformance case selection must not be empty");
}

#[test]
fn adapter_identifiers_match_packaged_schema() {
    check_cases("/adapter/id");
}

#[test]
fn rate_limit_names_match_packaged_schema() {
    check_cases("/rate_limit/name");
}
