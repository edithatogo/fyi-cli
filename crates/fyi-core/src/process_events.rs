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
    #[error("attachment sidecar must be a regular JSON array: {0}")]
    InvalidAttachmentSidecar(PathBuf),
    #[error("attachment locator is invalid or contains a signed path")]
    InvalidAttachmentLocator,
    #[error("attachment sidecar WARC record ID is invalid")]
    InvalidAttachmentWarcId,
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

fn timestamp(event: &Map<String, Value>) -> (Option<String>, &'static str) {
    let value = event
        .get("occurred_at")
        .or_else(|| event.get("created_at"))
        .or_else(|| event.get("updated_at"))
        .and_then(Value::as_str);
    match value {
        Some(value) if !value.is_empty() => {
            if crate::time_util::parse_strict_rfc3339(value).is_ok() {
                (Some(value.to_string()), "valid")
            } else {
                (None, "invalid")
            }
        }
        _ => (None, "missing"),
    }
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

/// Reject the known signed attachment path shape after bounded percent decoding.
/// This is not a general privacy assessment of arbitrary source paths.
fn known_attachment_path_is_safe(path: &str) -> bool {
    let mut candidate = path.to_owned();
    for _ in 0..=4 {
        if candidate
            .split_once("/attachments/")
            .is_some_and(|(_, tail)| tail.split('/').any(|segment| segment.contains("--")))
        {
            return false;
        }
        let Ok(decoded) = percent_encoding::percent_decode_str(&candidate).decode_utf8() else {
            return false;
        };
        if decoded == candidate {
            return true;
        }
        candidate = decoded.into_owned();
    }
    false
}

fn attachment_locator(item: &Map<String, Value>) -> Result<Value, ProcessEventError> {
    let raw = match item.get("url") {
        None | Some(Value::Null) => return Ok(Value::Null),
        Some(Value::String(raw)) if raw.is_empty() => return Ok(Value::Null),
        Some(Value::String(raw)) => raw,
        Some(_) => return Err(ProcessEventError::InvalidAttachmentLocator),
    };
    let mut url =
        reqwest::Url::parse(raw).map_err(|_| ProcessEventError::InvalidAttachmentLocator)?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(ProcessEventError::InvalidAttachmentLocator);
    }
    if !known_attachment_path_is_safe(url.path()) {
        return Err(ProcessEventError::InvalidAttachmentLocator);
    }
    if url.query().is_some() {
        url.set_query(Some("REDACTED"));
    }
    url.set_fragment(None);
    Ok(json!({"uri": url.as_str()}))
}

fn attachment_rows(
    path: &Path,
    row: &Map<String, Value>,
) -> Result<(Vec<Value>, &'static str), ProcessEventError> {
    if let Some(Value::Array(items)) = row.get("attachments") {
        return Ok((items.clone(), "request.json"));
    }
    let sidecar = path.with_file_name("attachments.json");
    match fs::symlink_metadata(&sidecar) {
        Ok(metadata) => {
            if !metadata.file_type().is_file() {
                return Err(ProcessEventError::InvalidAttachmentSidecar(sidecar));
            }
            let data = fs::read(&sidecar)?;
            let value: Value =
                serde_json::from_slice(&data).map_err(|source| ProcessEventError::Json {
                    path: sidecar.clone(),
                    source,
                })?;
            let items = value
                .as_array()
                .filter(|items| items.iter().all(Value::is_object))
                .ok_or(ProcessEventError::InvalidAttachmentSidecar(sidecar))?;
            Ok((items.clone(), "attachments.json"))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok((
            row.get("files")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default(),
            "request.json",
        )),
        Err(error) => Err(error.into()),
    }
}

fn attachment_warc_id(
    item: &Map<String, Value>,
    row: &Map<String, Value>,
    source_path: &str,
) -> Result<Value, ProcessEventError> {
    let value = item
        .get("warc_record_id")
        .or_else(|| row.get("warc_record_id"))
        .cloned()
        .unwrap_or(Value::Null);
    if source_path != "attachments.json" || value.is_null() {
        return Ok(value);
    }
    let raw = value
        .as_str()
        .ok_or(ProcessEventError::InvalidAttachmentWarcId)?;
    let raw = match raw.strip_prefix('<') {
        Some(inner) => inner
            .strip_suffix('>')
            .ok_or(ProcessEventError::InvalidAttachmentWarcId)?,
        None => raw,
    };
    let uuid = raw
        .strip_prefix("urn:uuid:")
        .ok_or(ProcessEventError::InvalidAttachmentWarcId)?;
    let parts: Vec<_> = uuid.split('-').collect();
    if parts.len() != 5
        || parts
            .iter()
            .zip([8, 4, 4, 4, 12])
            .any(|(part, len)| part.len() != len || !part.bytes().all(|c| c.is_ascii_hexdigit()))
    {
        return Err(ProcessEventError::InvalidAttachmentWarcId);
    }
    Ok(value)
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
        rows.push((request_id(row)?, path, row.clone()));
    }
    rows.sort_by_key(|(id, _, _)| *id);

    let mut events = Vec::new();
    let mut attachments = Vec::new();
    let mut current = BTreeSet::new();
    let mut digests = BTreeMap::new();
    for (request_id, request_path, row) in rows {
        let logical_id = format!("urn:fyi:nz-fyi:request:{request_id}");
        for (index, raw) in event_rows(&row).into_iter().enumerate() {
            let reference = source_ref(raw, index);
            let event_id = format!(
                "{logical_id}:event:{}",
                digest(&format!("{reference}:{index}"))
            );
            current.insert(event_id.clone());
            let (event_timestamp, timestamp_status) = timestamp(raw);
            let mut event = json!({
                "schema_version": SCHEMA_VERSION,
                "event_id": event_id.clone(),
                "logical_request_id": logical_id.clone(),
                "activity": activity(raw),
                "state": raw.get("state").or_else(|| raw.get("described_state")).cloned().unwrap_or(Value::Null),
                "timestamp": event_timestamp,
                "timestamp_status": timestamp_status,
                "source_order": {
                    "source": "urn:fyi-cli:site:fyi.org.nz",
                    "request_sequence": request_id,
                    "event_sequence": index
                },
                "provenance": {
                    "source_ref": reference,
                    "message_reference_id": raw.get("message_id").or_else(|| raw.get("message_reference_id")).cloned().unwrap_or(Value::Null),
                    "captured_at": captured_at
                },
                "operation": "upsert"
            });
            let event_digest = digest(&serde_json::to_string(&event)?);
            let old = previous.get(event_id.as_str());
            let previous_revision = old
                .and_then(|value| value.get("revision"))
                .and_then(Value::as_u64)
                .unwrap_or(0);
            let unchanged = old
                .filter(|value| value.get("deleted").and_then(Value::as_bool) != Some(true))
                .and_then(|value| value.get("digest"))
                .and_then(Value::as_str)
                == Some(event_digest.as_str());
            let revision = if unchanged {
                previous_revision
            } else {
                previous_revision + 1
            };
            event["revision"] = json!(revision);
            digests.insert(
                event_id,
                json!({"digest": event_digest, "revision": revision}),
            );
            if !unchanged {
                events.push(event);
            }
        }
        let (items, source_path) = attachment_rows(&request_path, &row)?;
        for (index, item) in items.iter().filter_map(Value::as_object).enumerate() {
            if source_path == "attachments.json"
                && (item
                    .get("content_type")
                    .is_some_and(|value| !value.is_null() && !value.is_string())
                    || item
                        .get("size")
                        .is_some_and(|value| !value.is_null() && value.as_u64().is_none()))
            {
                return Err(ProcessEventError::InvalidAttachmentSidecar(
                    request_path.with_file_name("attachments.json"),
                ));
            }
            let locator = attachment_locator(item)?;
            // Sidecar paths can contain request titles and attachment names.
            let public_locator = if source_path == "attachments.json" {
                Value::Null
            } else {
                locator.clone()
            };
            let identity_locator = if source_path == "attachments.json" {
                locator.get("uri").and_then(Value::as_str).unwrap_or("")
            } else {
                item.get("url").and_then(Value::as_str).unwrap_or("")
            };
            let warc_record_id = attachment_warc_id(item, &row, source_path)?;
            attachments.push(json!({
                "schema_version": SCHEMA_VERSION,
                "attachment_id": format!(
                    "{logical_id}:attachment:{}",
                    digest(&format!("{identity_locator}:{index}"))
                ),
                "logical_request_id": logical_id.clone(),
                "source_order": {
                    "source": "urn:fyi-cli:site:fyi.org.nz",
                    "request_sequence": request_id,
                    "attachment_sequence": index
                },
                "content_type": item.get("content_type").cloned().unwrap_or(Value::Null),
                "byte_size": item.get("size").cloned().unwrap_or(Value::Null),
                "locator": public_locator,
                "warc_record_id": warc_record_id,
                "provenance": {"captured_at": captured_at, "source_path": source_path}
            }));
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
    fn unchanged_checkpoint_preserves_revisions_after_restore() {
        let root = tempfile::tempdir().unwrap();
        let request = root.path().join("store/agency/701/request.json");
        fs::create_dir_all(request.parent().unwrap()).unwrap();
        let mut case = json!({
            "id": 701,
            "info_request_events": [
                {"id": "sent", "event_type": "sent"},
                {"id": "reply", "event_type": "response"},
                {"id": "follow_up", "event_type": "follow_up"}
            ]
        });
        fs::write(&request, serde_json::to_vec(&case).unwrap()).unwrap();
        let output = root.path().join("events.jsonl");
        let checkpoint = root.path().join("checkpoint.json");
        let export = || {
            export_process_events(
                &root.path().join("store"),
                &output,
                "2026-01-04T00:00:00Z",
                Some(&checkpoint),
                None,
            )
            .unwrap()
        };
        assert_eq!(export().event_count, 3);
        let saved = fs::read(&checkpoint).unwrap();
        assert_eq!(export().event_count, 0);
        assert_eq!(fs::read(&checkpoint).unwrap(), saved);
        case["info_request_events"][2]["event_type"] = json!("updated_follow_up");
        fs::write(&request, serde_json::to_vec(&case).unwrap()).unwrap();
        assert_eq!(export().event_count, 1);
        let event: Value = serde_json::from_slice(&fs::read(&output).unwrap()).unwrap();
        assert_eq!(event["revision"], 2);
        fs::write(&checkpoint, saved).unwrap();
        case["info_request_events"][2]["event_type"] = json!("follow_up");
        fs::write(&request, serde_json::to_vec(&case).unwrap()).unwrap();
        assert_eq!(export().event_count, 0);
    }

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

    #[test]
    fn exports_legacy_attachment_sidecar_without_local_path_or_query_secrets() {
        let root = tempfile::tempdir().unwrap();
        let store = root.path().join("Agency").join("134");
        fs::create_dir_all(&store).unwrap();
        fs::write(
            store.join("request.json"),
            r#"{"id":134,"info_request_events":[]}"#,
        )
        .unwrap();
        fs::write(
            store.join("attachments.json"),
            r#"[{"url":"https://source.invalid/request/example/response/1/attach/2/a.txt?cookie_passthrough=1","content_type":"text/plain","size":30,"path":"/private/fictional/a.txt","warc_record_id":"urn:uuid:00000000-0000-4000-8000-000000000001"}]"#,
        )
        .unwrap();
        let events = root.path().join("events.ndjson");
        let attachments = root.path().join("attachments.ndjson");
        let summary = export_process_events(
            root.path(),
            &events,
            "2026-01-01T00:00:00Z",
            None,
            Some(&attachments),
        )
        .unwrap();
        assert_eq!(summary.attachment_count, 1);
        let text = fs::read_to_string(attachments).unwrap();
        let row: Value = serde_json::from_str(text.trim()).unwrap();
        assert_eq!(row["byte_size"], 30);
        assert_eq!(row["content_type"], "text/plain");
        assert_eq!(
            row["warc_record_id"],
            "urn:uuid:00000000-0000-4000-8000-000000000001"
        );
        assert!(row["locator"].is_null());
        assert!(!text.contains("cookie_passthrough"));
        assert!(!text.contains("/private/fictional"));
        assert!(!text.contains("a.txt"));
    }

    #[test]
    fn inline_attachment_identity_is_stable_when_its_query_is_redacted() {
        let root = tempfile::tempdir().unwrap();
        let store = root.path().join("Agency").join("134");
        fs::create_dir_all(&store).unwrap();
        let url = "https://source.invalid/attachments/file.txt?cookie_passthrough=1";
        fs::write(
            store.join("request.json"),
            serde_json::to_vec(&json!({"id": 134, "attachments": [{"url": url}]})).unwrap(),
        )
        .unwrap();
        let attachments = root.path().join("attachments.ndjson");
        export_process_events(
            root.path(),
            &root.path().join("events.ndjson"),
            "2026-01-01T00:00:00Z",
            None,
            Some(&attachments),
        )
        .unwrap();
        let row: Value = serde_json::from_slice(&fs::read(attachments).unwrap()).unwrap();
        assert_eq!(
            row["attachment_id"],
            format!(
                "urn:fyi:nz-fyi:request:134:attachment:{}",
                digest(&format!("{url}:0"))
            )
        );
        assert!(row["locator"]["uri"]
            .as_str()
            .unwrap()
            .ends_with("?REDACTED"));
    }

    #[test]
    fn malformed_attachment_sidecar_fails_before_outputs() {
        let root = tempfile::tempdir().unwrap();
        let store = root.path().join("Agency").join("134");
        fs::create_dir_all(&store).unwrap();
        fs::write(store.join("request.json"), r#"{"id":134}"#).unwrap();
        fs::write(store.join("attachments.json"), "not-json").unwrap();
        let events = root.path().join("events.ndjson");
        let result = export_process_events(
            root.path(),
            &events,
            "2026-01-01T00:00:00Z",
            None,
            Some(&root.path().join("attachments.ndjson")),
        );
        assert!(result.is_err());
        assert!(!events.exists());
    }

    #[test]
    fn attachment_sidecar_rejects_credentialed_or_signed_locators() {
        for url in [
            "https://placeholder-user:placeholder-password@source.invalid/attachments/file.txt",
            "https://source.invalid/attachments/fictional--signature/wait",
            "https://source.invalid/attachments/fictional%2D%2Dsignature/wait",
        ] {
            let root = tempfile::tempdir().unwrap();
            let store = root.path().join("Agency").join("134");
            fs::create_dir_all(&store).unwrap();
            fs::write(store.join("request.json"), r#"{"id":134}"#).unwrap();
            fs::write(
                store.join("attachments.json"),
                serde_json::to_vec(&json!([{"url": url}])).unwrap(),
            )
            .unwrap();
            let events = root.path().join("events.ndjson");
            let error = export_process_events(
                root.path(),
                &events,
                "2026-01-01T00:00:00Z",
                None,
                Some(&root.path().join("attachments.ndjson")),
            )
            .unwrap_err();
            assert!(matches!(error, ProcessEventError::InvalidAttachmentLocator));
            assert!(!events.exists());
        }
    }

    #[test]
    fn inline_attachment_rejects_encoded_signed_paths_before_output() {
        for url in [
            "https://source.invalid/%61ttachments/fictional--signature/wait",
            "https://source.invalid/attachments%2Ffictional--signature/wait",
            "https://source.invalid/%2561ttachments/fictional--signature/wait",
        ] {
            let root = tempfile::tempdir().unwrap();
            let store = root.path().join("Agency").join("134");
            fs::create_dir_all(&store).unwrap();
            fs::write(
                store.join("request.json"),
                serde_json::to_vec(&json!({"id": 134, "attachments": [{"url": url}]})).unwrap(),
            )
            .unwrap();
            let events = root.path().join("events.ndjson");
            let error = export_process_events(
                root.path(),
                &events,
                "2026-01-01T00:00:00Z",
                None,
                Some(&root.path().join("attachments.ndjson")),
            )
            .unwrap_err();
            assert!(matches!(error, ProcessEventError::InvalidAttachmentLocator));
            assert!(!events.exists());
        }
    }

    #[test]
    fn attachment_sidecar_rejects_non_record_warc_ids() {
        let item = json!({"warc_record_id": "not-a-warc-record-id"});
        let row = serde_json::Map::new();
        assert!(matches!(
            attachment_warc_id(item.as_object().unwrap(), &row, "attachments.json"),
            Err(ProcessEventError::InvalidAttachmentWarcId)
        ));
    }

    #[test]
    fn attachment_sidecar_rejects_invalid_metadata_without_outputs() {
        for body in [
            r#"[1]"#,
            r#"[{"size":-1}]"#,
            r#"[{"content_type":23}]"#,
            r#"[{"url":23}]"#,
        ] {
            let root = tempfile::tempdir().unwrap();
            let store = root.path().join("Agency").join("134");
            fs::create_dir_all(&store).unwrap();
            fs::write(store.join("request.json"), r#"{"id":134}"#).unwrap();
            fs::write(store.join("attachments.json"), body).unwrap();
            let events = root.path().join("events.ndjson");
            assert!(export_process_events(
                root.path(),
                &events,
                "2026-01-01T00:00:00Z",
                None,
                Some(&root.path().join("attachments.ndjson")),
            )
            .is_err());
            assert!(!events.exists());
        }
    }
}
