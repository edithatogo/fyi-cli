//! Public-safe, deterministic process-event export for archive consumers.

use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

pub const SCHEMA_VERSION: &str = "1.0.0";

#[derive(Debug, Error)]
pub enum ProcessEventError {
    #[error("derived request store does not exist: {0}")]
    MissingStore(PathBuf),
    #[error("invalid JSON in {path}: {source}")]
    Json {
        path: PathBuf,
        source: serde_json::Error,
    },
    #[error("request record must be an object: {0}")]
    InvalidRecord(PathBuf),
    #[error("checkpoint must contain an event_digests object")]
    InvalidCheckpoint,
    #[error("request record must contain a positive integer id")]
    InvalidRequestId,
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExportSummary {
    pub schema_version: String,
    pub event_count: usize,
    pub total_event_count: usize,
    pub attachment_count: usize,
}

fn digest(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn request_id(row: &Map<String, Value>) -> Result<i64, ProcessEventError> {
    row.get("request_id")
        .or_else(|| row.get("id"))
        .and_then(Value::as_i64)
        .filter(|value| *value > 0)
        .ok_or(ProcessEventError::InvalidRequestId)
}

fn collect_request_files(root: &Path, output: &mut Vec<PathBuf>) -> Result<(), ProcessEventError> {
    if !root.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(root)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_request_files(&path, output)?;
        } else if path.file_name().and_then(|name| name.to_str()) == Some("request.json") {
            output.push(path);
        }
    }
    Ok(())
}

fn event_rows(row: &Map<String, Value>) -> Vec<&Map<String, Value>> {
    if let Some(Value::Array(events)) = row.get("info_request_events") {
        return events.iter().filter_map(Value::as_object).collect();
    }
    let mut found = Vec::new();
    fn walk<'a>(value: &'a Value, path: &str, found: &mut Vec<&'a Map<String, Value>>) {
        match value {
            Value::Object(map) => {
                let eventish = path.contains("event")
                    || path.contains("history")
                    || path.contains("message")
                    || map.contains_key("event_type");
                if eventish && (map.contains_key("created_at") || map.contains_key("event_type")) {
                    found.push(map);
                }
                for (key, child) in map {
                    walk(child, &format!("{path}.{key}"), found);
                }
            }
            Value::Array(items) => {
                for (index, child) in items.iter().enumerate() {
                    walk(child, &format!("{path}[{index}]"), found);
                }
            }
            _ => {}
        }
    }
    for (key, value) in row {
        walk(value, key, &mut found);
    }
    found
}

fn activity(event: &Map<String, Value>) -> String {
    event
        .get("event_type")
        .or_else(|| event.get("type"))
        .or_else(|| event.get("described_state"))
        .or_else(|| event.get("state"))
        .and_then(Value::as_str)
        .unwrap_or("observed")
        .trim()
        .to_lowercase()
        .replace(' ', "_")
}

fn source_ref(event: &Map<String, Value>, index: usize) -> String {
    event
        .get("id")
        .or_else(|| event.get("event_id"))
        .and_then(Value::as_str)
        .map(String::from)
        .unwrap_or_else(|| format!("source-index:{index}"))
}

fn load_checkpoint(path: Option<&Path>) -> Result<BTreeMap<String, Value>, ProcessEventError> {
    let Some(path) = path else {
        return Ok(BTreeMap::new());
    };
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    let value: Value = serde_json::from_str(&fs::read_to_string(path)?)?;
    let Some(events) = value.get("event_digests").and_then(Value::as_object) else {
        return Err(ProcessEventError::InvalidCheckpoint);
    };
    Ok(events
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect())
}

pub fn export_process_events(
    derived_dir: &Path,
    output: &Path,
    captured_at: &str,
    checkpoint_path: Option<&Path>,
    attachments_output: Option<&Path>,
) -> Result<ExportSummary, ProcessEventError> {
    if !derived_dir.is_dir() {
        return Err(ProcessEventError::MissingStore(derived_dir.to_path_buf()));
    }
    let previous = load_checkpoint(checkpoint_path)?;
    let mut paths = Vec::new();
    collect_request_files(derived_dir, &mut paths)?;
    paths.sort();
    let mut rows = Vec::new();
    for path in paths {
        let value: Value = serde_json::from_str(&fs::read_to_string(&path)?)?;
        let Some(row) = value.as_object() else {
            return Err(ProcessEventError::InvalidRecord(path));
        };
        rows.push((request_id(row)?, row.clone()));
    }
    rows.sort_by_key(|(id, _)| *id);

    let mut events = Vec::new();
    let mut attachments = Vec::new();
    let mut current = BTreeSet::new();
    let mut digests = BTreeMap::new();
    for (request_id, row) in rows {
        let logical_id = format!("urn:fyi:nz-fyi:request:{request_id}");
        for (index, raw) in event_rows(&row).into_iter().enumerate() {
            let reference = source_ref(raw, index);
            let event_id = format!(
                "{logical_id}:event:{}",
                digest(&format!("{reference}:{index}"))
            );
            current.insert(event_id.clone());
            let mut event = json!({
                "schema_version": SCHEMA_VERSION,
                "event_id": event_id.clone(),
                "logical_request_id": logical_id.clone(),
                "activity": activity(raw),
                "timestamp": raw.get("occurred_at")
                    .or_else(|| raw.get("created_at"))
                    .or_else(|| raw.get("updated_at"))
                    .and_then(Value::as_str)
                    .unwrap_or(captured_at),
                "source_order": {
                    "source": "urn:fyi-cli:site:fyi.org.nz",
                    "request_sequence": request_id,
                    "event_sequence": index
                },
                "provenance": {"source_ref": reference, "captured_at": captured_at},
                "operation": "upsert"
            });
            let event_digest = digest(&serde_json::to_string(&event)?);
            let old = previous.get(event_id.as_str());
            let revision = old
                .and_then(|value| value.get("revision"))
                .and_then(Value::as_u64)
                .unwrap_or(0)
                + 1;
            event["revision"] = json!(revision);
            let unchanged = old
                .filter(|value| value.get("deleted").and_then(Value::as_bool) != Some(true))
                .and_then(|value| value.get("digest"))
                .and_then(Value::as_str)
                == Some(event_digest.as_str());
            digests.insert(
                event_id,
                json!({"digest": event_digest, "revision": revision}),
            );
            if !unchanged {
                events.push(event);
            }
        }
        if let Some(Value::Array(items)) = row.get("attachments").or_else(|| row.get("files")) {
            for (index, item) in items.iter().filter_map(Value::as_object).enumerate() {
                attachments.push(json!({
                    "schema_version": SCHEMA_VERSION,
                    "attachment_id": format!(
                        "{logical_id}:attachment:{}",
                        digest(&format!("{}:{index}", item.get("url").and_then(Value::as_str).unwrap_or("")))
                    ),
                    "logical_request_id": logical_id.clone(),
                    "source_order": {
                        "source": "urn:fyi-cli:site:fyi.org.nz",
                        "request_sequence": request_id,
                        "attachment_sequence": index
                    },
                    "content_type": item.get("content_type").cloned().unwrap_or(Value::Null),
                    "byte_size": item.get("size").cloned().unwrap_or(Value::Null),
                    "locator": item.get("url").map(|url| json!({"uri": url})).unwrap_or(Value::Null),
                    "warc_record_id": row.get("warc_record_id").cloned().unwrap_or(Value::Null),
                    "provenance": {"captured_at": captured_at, "source_path": "request.json"}
                }));
            }
        }
    }
    for (event_id, old) in &previous {
        if !current.contains(event_id) && old.get("deleted").and_then(Value::as_bool) != Some(true)
        {
            let logical_id = event_id.split(":event:").next().unwrap_or(event_id);
            let revision = old.get("revision").and_then(Value::as_u64).unwrap_or(1) + 1;
            events.push(json!({
                "schema_version": SCHEMA_VERSION,
                "event_id": event_id,
                "logical_request_id": logical_id,
                "activity": "removed",
                "timestamp": captured_at,
                "source_order": {
                    "source": "urn:fyi-cli:site:fyi.org.nz",
                    "request_sequence": 0,
                    "event_sequence": events.len()
                },
                "provenance": {"captured_at": captured_at},
                "operation": "delete",
                "revision": revision
            }));
            digests.insert(
                event_id.clone(),
                json!({"digest": old.get("digest").cloned().unwrap_or(Value::Null), "revision": revision, "deleted": true}),
            );
        }
    }
    write_ndjson(output, &events)?;
    if let Some(path) = attachments_output {
        write_ndjson(path, &attachments)?;
    }
    if let Some(path) = checkpoint_path {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(
            path,
            serde_json::to_string_pretty(&json!({
                "schema_version": SCHEMA_VERSION,
                "captured_at": captured_at,
                "event_digests": digests
            }))? + "\n",
        )?;
    }
    Ok(ExportSummary {
        schema_version: SCHEMA_VERSION.to_string(),
        event_count: events.len(),
        total_event_count: digests.len(),
        attachment_count: attachments.len(),
    })
}

fn write_ndjson(path: &Path, rows: &[Value]) -> Result<(), ProcessEventError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut output = String::new();
    for row in rows {
        output.push_str(&serde_json::to_string(row)?);
        output.push('\n');
    }
    fs::write(path, output)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn exports_source_order_and_redacts_attachment_names() {
        let root = std::env::temp_dir().join(format!(
            "fyi-process-events-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let store = root.join("Agency").join("42");
        fs::create_dir_all(&store).expect("store");
        fs::write(
            store.join("request.json"),
            r#"{"id":42,"title":"private","info_request_events":[{"id":"a","event_type":"closed"},{"id":"b","event_type":"opened"}],"files":[{"name":"private.pdf","url":"https://fyi.example/attach/1","size":12}]}"#,
        )
        .expect("request");
        let events = root.join("events.ndjson");
        let attachments = root.join("attachments.ndjson");
        let summary = export_process_events(
            &root.join("Agency"),
            &events,
            "2026-01-01T00:00:00Z",
            None,
            Some(&attachments),
        )
        .expect("export");
        assert_eq!(summary.event_count, 2);
        let rows: Vec<Value> = fs::read_to_string(&events)
            .expect("events")
            .lines()
            .map(|line| serde_json::from_str(line).expect("json"))
            .collect();
        assert_eq!(rows[0]["source_order"]["event_sequence"], 0);
        assert_eq!(rows[1]["source_order"]["event_sequence"], 1);
        assert!(!fs::read_to_string(&attachments)
            .expect("attachments")
            .contains("private.pdf"));
        let _ = fs::remove_dir_all(root);
    }
}
