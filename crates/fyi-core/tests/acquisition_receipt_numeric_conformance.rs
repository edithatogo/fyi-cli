//! Frozen CPython bytes are the independent oracle, not Rust reserialization.
use fyi_core::acquisition_receipts::{canonical_json_bytes, verify_receipt};
use proptest::prelude::*;
use serde::Deserialize;
use serde_json::{json, Value};

const CORPUS: &str = include_str!("../../../conformance/acquisition/numeric-v1-python.json");
const RECEIPT: &str = include_str!("../../../conformance/acquisition/receipt-v1-exponent-python.json");

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
fn finite_float_serialization_matches_cpython_bytes() {
    let corpus: Corpus = serde_json::from_str(CORPUS).unwrap();
    assert_eq!(corpus.cases.len(), 60);
    for case in corpus.cases {
        let bits = u64::from_str_radix(&case.bits, 16).unwrap();
        let value = Value::from(f64::from_bits(bits));
        assert_eq!(
            canonical_json_bytes(&value).unwrap(),
            case.canonical.as_bytes(),
            "CPython canonical bytes differ for bits {}",
            case.bits
        );
    }
}

#[test]
fn python_float_literals_preserve_their_exact_bits() {
    let corpus: Corpus = serde_json::from_str(CORPUS).unwrap();
    for case in corpus.cases {
        let value: Value = serde_json::from_str(&case.canonical).unwrap();
        let expected = u64::from_str_radix(&case.bits, 16).unwrap();
        assert_eq!(value.as_f64().unwrap().to_bits(), expected, "{}", case.bits);
    }
}

#[test]
fn python_receipt_with_exponent_intervals_verifies_without_rehashing() {
    let receipt: Value = serde_json::from_str(RECEIPT).unwrap();
    verify_receipt(&receipt).unwrap();
    assert_eq!(canonical_json_bytes(&receipt).unwrap(), RECEIPT.as_bytes());
    let mut changed = receipt;
    changed["rate_limit"]["minimum_interval_seconds"] = json!(2e-7);
    assert!(verify_receipt(&changed).is_err());
}

#[test]
fn canonical_numbers_preserve_integer_types_and_nested_strings() {
    let value = json!({
        "z": [0, 1.0, -0.0, 1e-7, null, true],
        "a": {"max": u64::MAX, "min": i64::MIN, "text": "λ\n\"\\"},
    });
    let expected = concat!(
        "{\"a\":{\"max\":18446744073709551615,\"min\":-9223372036854775808,",
        "\"text\":\"λ\\n\\\"\\\\\"},\"z\":[0,1.0,-0.0,1e-07,null,true]}\n"
    );
    assert_eq!(canonical_json_bytes(&value).unwrap(), expected.as_bytes());
}

proptest! {
    #[test]
    fn finite_float_canonical_bytes_round_trip_without_changing_bits(bits in any::<u64>()) {
        let value = f64::from_bits(bits);
        prop_assume!(value.is_finite());
        let bytes = canonical_json_bytes(&Value::from(value)).unwrap();
        let parsed: Value = serde_json::from_slice(&bytes).unwrap();
        prop_assert_eq!(parsed.as_f64().unwrap().to_bits(), bits);
        prop_assert_eq!(canonical_json_bytes(&parsed).unwrap(), bytes);
    }
}
