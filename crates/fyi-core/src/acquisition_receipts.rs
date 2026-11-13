//! Offline, fail-closed verifier for Python-compatible acquisition receipts v1.
//!
//! This is a boundary contract, not an HTTP client. It verifies canonical
//! content digests and structural invariants without contacting source sites.
//! The v1 fixture originates in the existing Python receipt schema/serializer.
use crate::provenance::sha256_hex;
use anyhow::{bail, ensure, Context, Result};
use chrono::DateTime;
use serde_json::Value;

/// Canonical UTF-8 JSON with stable key ordering and exactly one newline.
///
/// This matches Python's `json.dumps(sort_keys=True, separators=(",", ":"),
/// ensure_ascii=False, allow_nan=False) + "\n"` for the receipt-v1 subset.
pub fn canonical_json_bytes(value: &Value) -> Result<Vec<u8>> {
    let mut data = serde_json::to_vec(value).context("receipt cannot be serialized")?;
    data.push(b'\n');
    Ok(data)
}

fn string<'a>(value: &'a Value, name: &str) -> Result<&'a str> {
    value
        .get(name)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .with_context(|| format!("receipt missing nonempty {name}"))
}

fn object_field<'a>(value: &'a Value, name: &str) -> Result<&'a Value> {
    value
        .get(name)
        .filter(|v| v.is_object())
        .with_context(|| format!("receipt missing object {name}"))
}

fn integer(value: &Value, name: &str) -> Result<u64> {
    value
        .get(name)
        .and_then(Value::as_u64)
        .with_context(|| format!("receipt missing nonnegative integer {name}"))
}

fn hash(value: &Value, name: &str) -> Result<()> {
    let s = string(value, name)?;
    ensure!(
        s.len() == 64
            && s.bytes()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
        "receipt {name} is not a lowercase SHA-256 digest"
    );
    Ok(())
}

fn safe_url(value: &Value, name: &str) -> Result<()> {
    let raw = string(value, name)?;
    let url = reqwest::Url::parse(raw).context("invalid receipt URL")?;
    ensure!(
        url.scheme() == "http" || url.scheme() == "https",
        "receipt URL must use HTTP(S)"
    );
    ensure!(url.host_str().is_some(), "receipt URL missing host");
    ensure!(
        url.username().is_empty() && url.password().is_none(),
        "receipt URL contains credentials"
    );
    ensure!(url.fragment().is_none(), "receipt URL contains a fragment");
    ensure!(
        url.query().is_none_or(|q| q == "REDACTED"),
        "receipt URL contains an unredacted query"
    );
    Ok(())
}

fn optional_hash(value: &Value, name: &str) -> Result<()> {
    if !value.get(name).is_some_and(Value::is_null) {
        hash(value, name)?;
    }
    Ok(())
}

/// Validate a v1 receipt, including its Python-compatible self digest, response
/// counts, URLs, source metadata and failure semantics. No external I/O.
pub fn verify_receipt(receipt: &Value) -> Result<()> {
    let source = receipt
        .as_object()
        .context("receipt must be a JSON object")?;
    ensure!(
        source.get("schema").and_then(Value::as_str) == Some("urn:fyi-cli:acquisition-receipt"),
        "unrecognised receipt schema"
    );
    ensure!(
        source.get("schema_version").and_then(Value::as_str) == Some("1.0.0"),
        "unsupported receipt schema version"
    );
    hash(receipt, "receipt_sha256")?;
    string(receipt, "command")?;
    let adapter = object_field(receipt, "adapter")?;
    string(adapter, "id")?;
    string(adapter, "version")?;
    safe_url(object_field(receipt, "source")?, "url")?;
    object_field(receipt, "request_bounds")?;
    let start = DateTime::parse_from_rfc3339(string(receipt, "started_at")?)
        .context("invalid started_at")?;
    let end = DateTime::parse_from_rfc3339(string(receipt, "completed_at")?)
        .context("invalid completed_at")?;
    ensure!(end >= start, "receipt completion precedes start");
    let status = string(receipt, "status")?;
    ensure!(
        status == "succeeded" || status == "failed",
        "invalid receipt status"
    );
    if status == "failed" {
        string(object_field(receipt, "failure")?, "type")?;
    } else {
        ensure!(
            receipt.get("failure").is_none(),
            "successful receipt cannot have failure"
        );
    }
    let rate_limit = object_field(receipt, "rate_limit")?;
    ensure!(rate_limit.get("name").is_some(), "missing rate limit name");
    ensure!(
        rate_limit.get("minimum_interval_seconds").is_some_and(|v| {
            v.is_null() || v.as_f64().is_some_and(|f| f.is_finite() && f >= 0.0)
        }),
        "invalid minimum interval"
    );
    let checkpoint = object_field(receipt, "checkpoint")?;
    optional_hash(checkpoint, "before_sha256")?;
    optional_hash(checkpoint, "after_sha256")?;
    let projection = object_field(receipt, "result")?;
    ensure!(
        projection.get("representation").and_then(Value::as_str)
            == Some("canonical_result_projection"),
        "unsupported result representation"
    );
    string(projection, "media_type")?;
    integer(projection, "bytes")?;
    hash(projection, "sha256")?;
    let responses = receipt
        .get("responses")
        .and_then(Value::as_array)
        .context("missing response array")?;
    let mut bytes = 0_u64;
    let mut retries = 0_u64;
    for response in responses {
        safe_url(response, "url")?;
        let code = integer(response, "status")?;
        ensure!((100..=599).contains(&code), "invalid HTTP status");
        hash(response, "sha256")?;
        bytes = bytes
            .checked_add(integer(response, "bytes")?)
            .context("response byte count overflow")?;
        let attempts = integer(response, "attempts")?;
        ensure!(attempts >= 1, "response attempts cannot be zero");
        retries = retries
            .checked_add(attempts - 1)
            .context("retry count overflow")?;
        let delays = response
            .get("retry_delays_seconds")
            .and_then(Value::as_array)
            .context("missing retry delay array")?;
        for delay in delays {
            ensure!(
                delay.as_f64().is_some_and(|f| f.is_finite() && f >= 0.0),
                "invalid retry delay"
            );
        }
    }
    let totals = object_field(receipt, "totals")?;
    ensure!(
        integer(totals, "requests")? == responses.len() as u64,
        "request total mismatch"
    );
    ensure!(
        integer(totals, "bytes")? == bytes,
        "response byte total mismatch"
    );
    ensure!(
        integer(totals, "retries")? == retries,
        "retry total mismatch"
    );
    let mut unsigned = source.clone();
    unsigned.remove("receipt_sha256");
    let canonical = canonical_json_bytes(&Value::Object(unsigned))?;
    if sha256_hex(&canonical) != string(receipt, "receipt_sha256")? {
        bail!("receipt_sha256 mismatch");
    }
    Ok(())
}
