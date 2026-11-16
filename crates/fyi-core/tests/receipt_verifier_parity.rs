//! Shared semantic receipt cases, independently exercised in Rust and Python.
use fyi_core::acquisition_receipts::{canonical_json_bytes, verify_receipt};
use fyi_core::receipt_wire::parse_receipt_bytes;
use serde_json::{json, Value};

const GOLDEN: &str = include_str!("../../../conformance/acquisition/receipt-v1-python.json");
const CORPUS: &str =
    include_str!("../../../conformance/acquisition/receipt-verifier-parity-v1.json");

#[test]
fn rust_receipt_semantics_match_the_shared_contract() {
    let corpus: Value = serde_json::from_str(CORPUS).unwrap();
    assert_eq!(corpus["contract"], "urn:fyi-cli:receipt-verifier-parity:1");
    for case in corpus["cases"].as_array().unwrap() {
        let mut receipt: Value = serde_json::from_str(GOLDEN).unwrap();
        for (pointer, replacement) in case["patches"].as_object().unwrap() {
            if pointer == "/failure" {
                receipt["failure"] = replacement.clone();
            } else {
                *receipt.pointer_mut(pointer).unwrap() = replacement.clone();
            }
        }
        receipt.as_object_mut().unwrap().remove("receipt_sha256");
        receipt["receipt_sha256"] = json!(fyi_core::provenance::sha256_hex(
            &canonical_json_bytes(&receipt).unwrap()
        ));
        let result = verify_receipt(&receipt);
        assert_eq!(
            result.is_ok(),
            case["valid"].as_bool().unwrap(),
            "{}: {:?}",
            case["id"],
            result.err()
        );
    }
}

#[test]
fn wire_recursion_boundary_counts_containers_not_string_content() {
    for (arrays, valid) in [(125, true), (126, false)] {
        let mut receipt: Value = serde_json::from_str(GOLDEN).unwrap();
        let mut nested = json!(0);
        for _ in 0..arrays {
            nested = json!([nested]);
        }
        receipt["request_bounds"] = json!({"nested": nested, "brackets_in_string": "[{\"\\\\]"});
        receipt.as_object_mut().unwrap().remove("receipt_sha256");
        receipt["receipt_sha256"] = json!(fyi_core::provenance::sha256_hex(
            &canonical_json_bytes(&receipt).unwrap()
        ));
        let wire = canonical_json_bytes(&receipt).unwrap();
        assert_eq!(parse_receipt_bytes(&wire).is_ok(), valid, "arrays={arrays}");
    }
}

#[test]
fn wire_checks_python_signed_integer_bounds_at_serde_capacity() {
    for (number, valid) in [
        ("-9223372036854775809", false),
        ("-9223372036854775808", true),
        ("18446744073709551615", true),
        ("18446744073709551616", false),
    ] {
        let mut receipt: Value = serde_json::from_str(GOLDEN).unwrap();
        receipt["request_bounds"] = json!({"nested": [0]});
        receipt.as_object_mut().unwrap().remove("receipt_sha256");
        let canonical = String::from_utf8(canonical_json_bytes(&receipt).unwrap())
            .unwrap()
            .replace("\"nested\":[0]", &format!("\"nested\":[{number}]"));
        receipt["receipt_sha256"] = json!(fyi_core::provenance::sha256_hex(canonical.as_bytes()));
        let wire = String::from_utf8(canonical_json_bytes(&receipt).unwrap())
            .unwrap()
            .replace("\"nested\":[0]", &format!("\"nested\":[{number}]"));
        assert_eq!(
            parse_receipt_bytes(wire.as_bytes()).is_ok(),
            valid,
            "{number}"
        );
    }
}
