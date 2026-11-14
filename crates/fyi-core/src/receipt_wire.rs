//! Unambiguous, bounded receipt JSON ingestion before schema/hash verification.
//!
//! A pre-parsed `Value` cannot reveal duplicate members discarded by its parser.
//! Consumers of external receipt bytes should use this boundary rather than
//! deserializing directly into `Value`. This module never rewrites evidence.
use crate::acquisition_receipts::verify_receipt;
use anyhow::{ensure, Context, Result};
use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::Deserialize;
use serde_json::{Map, Number, Value};
use std::fmt;
use std::io::Read;

/// Maximum receipt document size, including whitespace; not a payload limit.
pub const MAX_RECEIPT_BYTES: usize = 16 * 1024 * 1024;

/// Decode one complete, duplicate-free JSON document, then verify its v1
/// schema invariants and self-digest. Harmless whitespace and key order remain
/// accepted. Serde's recursion protection and exact float parsing are retained.
pub fn parse_receipt_bytes(bytes: &[u8]) -> Result<Value> {
    ensure!(
        bytes.len() <= MAX_RECEIPT_BYTES,
        "receipt document exceeds the 16 MiB byte limit"
    );
    let StrictValue(receipt) = serde_json::from_slice(bytes).context("invalid receipt JSON")?;
    verify_receipt(&receipt).context("invalid acquisition receipt")?;
    Ok(receipt)
}

/// Read at most the document limit plus one byte, then validate the receipt.
/// The extra byte distinguishes an exactly full document from oversized input.
/// This bounds input buffering, not the heap overhead of the parsed JSON tree.
pub fn read_receipt(reader: impl Read) -> Result<Value> {
    let mut bytes = Vec::new();
    reader
        .take((MAX_RECEIPT_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .context("reading receipt failed")?;
    parse_receipt_bytes(&bytes)
}

struct StrictValue(Value);

impl<'de> Deserialize<'de> for StrictValue {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_any(StrictVisitor)
    }
}

struct StrictVisitor;

impl<'de> Visitor<'de> for StrictVisitor {
    type Value = StrictValue;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON value without duplicate object members")
    }

    fn visit_bool<E>(self, value: bool) -> std::result::Result<Self::Value, E> {
        Ok(StrictValue(Value::Bool(value)))
    }

    fn visit_i64<E>(self, value: i64) -> std::result::Result<Self::Value, E> {
        Ok(StrictValue(Value::Number(value.into())))
    }

    fn visit_u64<E>(self, value: u64) -> std::result::Result<Self::Value, E> {
        Ok(StrictValue(Value::Number(value.into())))
    }

    fn visit_f64<E>(self, value: f64) -> std::result::Result<Self::Value, E>
    where
        E: de::Error,
    {
        Number::from_f64(value)
            .map(Value::Number)
            .map(StrictValue)
            .ok_or_else(|| E::custom("nonfinite number in receipt"))
    }

    fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E> {
        Ok(StrictValue(Value::String(value.to_owned())))
    }

    fn visit_string<E>(self, value: String) -> std::result::Result<Self::Value, E> {
        Ok(StrictValue(Value::String(value)))
    }

    fn visit_unit<E>(self) -> std::result::Result<Self::Value, E> {
        Ok(StrictValue(Value::Null))
    }

    fn visit_seq<A>(self, mut sequence: A) -> std::result::Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::new();
        while let Some(StrictValue(value)) = sequence.next_element()? {
            values.push(value);
        }
        Ok(StrictValue(Value::Array(values)))
    }

    fn visit_map<A>(self, mut object: A) -> std::result::Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut values = Map::new();
        while let Some(key) = object.next_key::<String>()? {
            // Keys are already decoded, so escape-equivalent names collide.
            // Do not echo untrusted names or values in the diagnostic.
            if values.contains_key(&key) {
                return Err(de::Error::custom("duplicate object member in receipt"));
            }
            let StrictValue(value) = object.next_value()?;
            values.insert(key, value);
        }
        Ok(StrictValue(Value::Object(values)))
    }
}
