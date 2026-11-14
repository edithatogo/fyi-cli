//! Shared Python/Rust numeric-schema conformance; no network or fixture rewriting.
use fyi_core::acquisition_receipts::{
    canonical_json_bytes, verify_receipt, verify_receipt_raw_bytes,
    verify_receipt_result_bytes, write_receipt_atomic,
};
use fyi_core::provenance::sha256_hex;
use fyi_core::receipt_wire::parse_receipt_bytes;
use proptest::prelude::*;
use serde_json::{json, Value};

const GOLDEN: &str = include_str!("../../../conformance/acquisition/receipt-v1-python.json");
const CORPUS: &str =
    include_str!("../../../conformance/acquisition/integer-schema-cases-v1.json");

fn golden() -> Value {
    serde_json::from_str(GOLDEN).unwrap()
}

fn resign(receipt: &mut Value) {
    receipt.as_object_mut().unwrap().remove("receipt_sha256");
    receipt["receipt_sha256"] = json!(sha256_hex(&canonical_json_bytes(receipt).unwrap()));
}

fn apply_case(case: &Value) -> Value {
    let mut receipt = golden();
    for (pointer, replacement) in case["patches"].as_object().unwrap() {
        *receipt.pointer_mut(pointer).expect("known receipt field") = replacement.clone();
    }
    resign(&mut receipt);
    receipt
}

#[test]
fn shared_numeric_corpus_matches_rust_capacity_and_semantic_policy() {
    let corpus: Value = serde_json::from_str(CORPUS).unwrap();
    for case in corpus["cases"].as_array().unwrap() {
        let receipt = apply_case(case);
        let bytes = canonical_json_bytes(&receipt).unwrap();
        let expected = case["rust_valid"].as_bool().unwrap();
        let result = verify_receipt(&receipt);
        assert_eq!(result.is_ok(), expected, "case {}: {result:?}", case["id"]);
        assert_eq!(
            parse_receipt_bytes(&bytes).is_ok(),
            expected,
            "wire case {}",
            case["id"]
        );
        assert_eq!(canonical_json_bytes(&receipt).unwrap(), bytes);
    }
}

#[test]
fn integral_decimal_sizes_verify_original_payload_and_projection_bytes() {
    let corpus: Value = serde_json::from_str(CORPUS).unwrap();
    let receipt = apply_case(&corpus["cases"][0]);
    verify_receipt_raw_bytes(&receipt, b"payload").unwrap();
    verify_receipt_result_bytes(&receipt, b"[]\n").unwrap();
    assert!(verify_receipt_raw_bytes(&receipt, b"payloaD").is_err());
    assert!(verify_receipt_result_bytes(&receipt, b"{}\n").is_err());
}

#[test]
fn decimal_receipt_publication_preserves_hash_and_numeric_representation() {
    let corpus: Value = serde_json::from_str(CORPUS).unwrap();
    let receipt = apply_case(&corpus["cases"][0]);
    let directory = std::env::temp_dir().join(format!(
        "fyi-receipt-integer-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let path = directory.join("receipt.json");
    let expected = canonical_json_bytes(&receipt).unwrap();
    write_receipt_atomic(&path, &receipt).unwrap();
    write_receipt_atomic(&path, &receipt).unwrap();
    let stored = std::fs::read(&path).unwrap();
    assert_eq!(stored, expected);
    assert_eq!(parse_receipt_bytes(&stored).unwrap(), receipt);
    assert!(String::from_utf8(stored).unwrap().contains("\"bytes\":7.0"));
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn integral_float_conversion_does_not_saturate_at_u64_maximum() {
    let mut receipt = golden();
    receipt["responses"][0]["bytes"] = json!(u64::MAX as f64);
    receipt["totals"]["bytes"] = json!(u64::MAX as f64);
    resign(&mut receipt);
    assert!(verify_receipt(&receipt).is_err());
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    #[test]
    fn integral_decimal_byte_counts_obey_schema_semantics(count in 0_u32..1_000_000) {
        let mut receipt = golden();
        receipt["responses"][0]["bytes"] = json!(f64::from(count));
        receipt["totals"]["bytes"] = json!(f64::from(count));
        resign(&mut receipt);
        prop_assert!(verify_receipt(&receipt).is_ok());
    }

    #[test]
    fn nonintegral_byte_counts_remain_rejected(count in 0_u32..1_000_000) {
        let mut receipt = golden();
        receipt["responses"][0]["bytes"] = json!(f64::from(count) + 0.5);
        receipt["totals"]["bytes"] = json!(f64::from(count) + 0.5);
        resign(&mut receipt);
        prop_assert!(verify_receipt(&receipt).is_err());
    }
}
