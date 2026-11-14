//! CPython midpoint oracles for the counterexamples in rust-lang/rust#130936.
use fyi_core::acquisition_receipts::{canonical_json_bytes, verify_receipt};
use serde::Deserialize;
use serde_json::Value;

const CORPUS: &str =
    include_str!("../../../conformance/acquisition/numeric-v1-midpoints-python.json");
const RECEIPT: &str =
    include_str!("../../../conformance/acquisition/receipt-v1-midpoints-python.json");

#[derive(Deserialize)]
struct Corpus {
    cases: Vec<Case>,
}

#[derive(Deserialize)]
struct Case {
    bits: String,
    canonical: String,
}

#[test]
fn shortest_decimal_midpoints_match_cpython_ties_to_even() {
    let corpus: Corpus = serde_json::from_str(CORPUS).unwrap();
    assert_eq!(corpus.cases.len(), 15);
    let mut mismatches = Vec::new();
    for case in corpus.cases {
        let bits = u64::from_str_radix(&case.bits, 16).unwrap();
        let value = Value::from(f64::from_bits(bits));
        let actual = String::from_utf8(canonical_json_bytes(&value).unwrap()).unwrap();
        if actual != case.canonical {
            mismatches.push((case.bits, actual, case.canonical));
        }
    }
    assert!(mismatches.is_empty(), "midpoint mismatches: {mismatches:?}");
}

#[test]
fn python_midpoint_receipt_verifies_without_changing_its_digest() {
    let receipt: Value = serde_json::from_str(RECEIPT).unwrap();
    verify_receipt(&receipt).unwrap();
    assert_eq!(canonical_json_bytes(&receipt).unwrap(), RECEIPT.as_bytes());
}
