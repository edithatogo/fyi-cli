use fyi_core::acquisition_receipts::canonical_json_bytes;
use fyi_core::provenance::sha256_hex;
use fyi_core::receipt_wire::{parse_receipt_bytes, read_receipt, MAX_RECEIPT_BYTES};
use proptest::prelude::*;
use serde_json::{json, Value};
use std::io::{self, Read};

const FIXTURE: &[u8] = include_bytes!("../../../conformance/acquisition/receipt-v1-python.json");

fn with_bounds(bounds: Value) -> Vec<u8> {
    let mut receipt: Value = serde_json::from_slice(FIXTURE).unwrap();
    receipt["request_bounds"] = bounds;
    receipt.as_object_mut().unwrap().remove("receipt_sha256");
    let digest = sha256_hex(&canonical_json_bytes(&receipt).unwrap());
    receipt["receipt_sha256"] = Value::String(digest);
    canonical_json_bytes(&receipt).unwrap()
}

#[test]
fn checked_in_python_receipts_remain_valid() {
    for bytes in [
        FIXTURE,
        include_bytes!("../../../conformance/acquisition/receipt-v1-exponent-python.json"),
        include_bytes!("../../../conformance/acquisition/receipt-v1-midpoints-python.json"),
    ] {
        let expected: Value = serde_json::from_slice(bytes).unwrap();
        assert_eq!(parse_receipt_bytes(bytes).unwrap(), expected);
        assert_eq!(read_receipt(bytes).unwrap(), expected);
    }
}

#[test]
fn repeated_names_are_scoped_to_each_object() {
    let bytes = with_bounds(json!({"a": {"x": 1}, "b": [{"x": 2}, {"x": 3}]}));
    assert!(parse_receipt_bytes(&bytes).is_ok());
}

#[test]
fn duplicates_in_free_form_nested_arrays_are_rejected() {
    let bytes = with_bounds(json!({"nested": [{"x": 1}]}));
    let wire = String::from_utf8(bytes)
        .unwrap()
        .replace("\"x\":1", "\"x\":0,\"x\":1");
    let error = parse_receipt_bytes(wire.as_bytes()).unwrap_err();
    assert!(format!("{error:#}").contains("duplicate"));
}

#[test]
fn unicode_escape_equivalent_keys_are_duplicates() {
    let bytes = with_bounds(json!({"λ": 1}));
    let wire = String::from_utf8(bytes)
        .unwrap()
        .replace("\"λ\":1", "\"\\u03bb\":0,\"λ\":1");
    let error = parse_receipt_bytes(wire.as_bytes()).unwrap_err();
    assert!(format!("{error:#}").contains("duplicate"));
}

#[test]
fn duplicate_errors_do_not_echo_names_or_values() {
    let wire = br#"{"synthetic-private-marker":0,"synthetic-private-marker":1}"#;
    let message = format!("{:#}", parse_receipt_bytes(wire).unwrap_err());
    assert!(message.contains("duplicate"));
    assert!(!message.contains("synthetic-private-marker"));
}

#[test]
fn integer_float_boolean_null_and_string_types_are_preserved() {
    let bytes = with_bounds(json!({
        "ints": [i64::MIN, u64::MAX, 0],
        "floats": [-0.0, 1.0, 1e-7, 1e16],
        "other": [true, false, null, "λ\n\"\\"],
    }));
    let receipt = parse_receipt_bytes(&bytes).unwrap();
    assert_eq!(canonical_json_bytes(&receipt).unwrap(), bytes);
    assert_eq!(receipt["request_bounds"]["ints"][0].as_i64(), Some(i64::MIN));
    assert_eq!(receipt["request_bounds"]["ints"][1].as_u64(), Some(u64::MAX));
    let zero = receipt["request_bounds"]["floats"][0].as_f64().unwrap();
    assert_eq!(zero.to_bits(), (-0.0_f64).to_bits());
}

#[test]
fn invalid_json_and_nonreceipt_roots_fail_closed() {
    for bytes in [
        b"".as_slice(),
        b"null",
        b"[]",
        b"true",
        b"42",
        b"{}",
        b"{",
        b"{\"a\":NaN}",
        b"{\"a\":Infinity}",
        b"{\"a\":1e999}",
        b"{\"a\":\"\xff\"}",
        b"{\"a\":\"\\ud800\"}",
    ] {
        assert!(parse_receipt_bytes(bytes).is_err(), "{bytes:?}");
    }
}

#[test]
fn extra_json_documents_and_trailing_junk_are_rejected() {
    for suffix in [b"{}".as_slice(), b"null", b"garbage", b"\0"] {
        let mut bytes = FIXTURE.to_vec();
        bytes.extend_from_slice(suffix);
        assert!(parse_receipt_bytes(&bytes).is_err());
    }
}

#[test]
fn schema_and_hash_verification_cannot_be_bypassed() {
    let mut receipt: Value = serde_json::from_slice(FIXTURE).unwrap();
    receipt["command"] = json!("different");
    assert!(parse_receipt_bytes(&canonical_json_bytes(&receipt).unwrap()).is_err());
    receipt.as_object_mut().unwrap().remove("command");
    assert!(parse_receipt_bytes(&canonical_json_bytes(&receipt).unwrap()).is_err());
}

#[test]
fn recursion_protection_is_not_disabled() {
    let nested = format!("{}0{}", "[".repeat(150), "]".repeat(150));
    let message = format!("{:#}", parse_receipt_bytes(nested.as_bytes()).unwrap_err());
    assert!(message.contains("recursion"), "{message}");
}

#[test]
fn exactly_the_document_limit_is_accepted_and_one_more_byte_is_rejected() {
    let mut bytes = FIXTURE.to_vec();
    bytes.resize(MAX_RECEIPT_BYTES, b' ');
    assert!(read_receipt(bytes.as_slice()).is_ok());
    bytes.push(b' ');
    let message = format!("{:#}", read_receipt(bytes.as_slice()).unwrap_err());
    assert!(message.contains("limit"));
}

struct EndlessWhitespace {
    consumed: usize,
}

impl Read for EndlessWhitespace {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        buffer.fill(b' ');
        self.consumed += buffer.len();
        assert!(self.consumed <= MAX_RECEIPT_BYTES + 1);
        Ok(buffer.len())
    }
}

#[test]
fn reader_stops_at_the_fixed_byte_budget_without_requiring_eof() {
    let mut reader = EndlessWhitespace { consumed: 0 };
    assert!(read_receipt(&mut reader).is_err());
    assert_eq!(reader.consumed, MAX_RECEIPT_BYTES + 1);
}

struct BrokenReader;

impl Read for BrokenReader {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::other("synthetic read failure"))
    }
}

#[test]
fn io_failures_never_yield_a_receipt() {
    let message = format!("{:#}", read_receipt(BrokenReader).unwrap_err());
    assert!(message.contains("reading receipt failed"));
}

proptest! {
    #[test]
    fn inserting_a_duplicate_never_preserves_acceptance(key in "[a-z]{1,12}", number in any::<i32>()) {
        let key = format!("fixture-{key}");
        let mut inner = serde_json::Map::new();
        inner.insert(key.clone(), json!(number));
        let bytes = with_bounds(json!({"nested": [inner]}));
        prop_assert!(parse_receipt_bytes(&bytes).is_ok());
        let wire = String::from_utf8(bytes).unwrap();
        let needle = format!("\"{key}\":");
        let replacement = format!("\"{key}\":null,\"{key}\":");
        let ambiguous = wire.replacen(&needle, &replacement, 1);
        prop_assert_ne!(&wire, &ambiguous);
        prop_assert!(parse_receipt_bytes(ambiguous.as_bytes()).is_err());
    }
}
