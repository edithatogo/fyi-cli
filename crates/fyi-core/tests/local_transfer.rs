//! Local transfer invariants beyond the public CLI fixture contracts.

use fyi_core::db::local::transfer::{
    encode_local_export, parse_local_import, LocalImport, LocalTransferDocument,
};
use fyi_core::db::local::{LocalAuthority, LocalError};
use fyi_core::db::DbPool;
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const NATIVE_FIXTURE: &str =
    include_str!("../../../conformance/local-management/requests-v1-transfer.json");

fn native_document() -> Value {
    let mut document: Value = serde_json::from_str(NATIVE_FIXTURE).unwrap();
    document["events"] = json!([
        {
            "id": 41,
            "tracked_request_id": 7,
            "kind": "created",
            "old_status": null,
            "new_status": "draft",
            "occurred_at": "2026-01-02T03:04:05Z"
        },
        {
            "id": 42,
            "tracked_request_id": 7,
            "kind": "status_changed",
            "old_status": "draft",
            "new_status": "waiting_response",
            "occurred_at": "2026-01-03T04:05:06+00:00"
        }
    ]);
    document["sequences"]["local_request_events"] = json!(50);
    document
}

fn parsed(document: &Value) -> LocalImport {
    parse_local_import(&serde_json::to_vec(document).unwrap()).unwrap()
}

async fn database() -> DbPool {
    let db = DbPool::new_in_memory().await.unwrap();
    db.run_migrations().await.unwrap();
    db
}

async fn exported(db: &DbPool) -> Vec<u8> {
    let document: LocalTransferDocument = db.export_local_requests().await.unwrap();
    serde_json::to_vec(&document).unwrap()
}

async fn state(db: &DbPool) -> Value {
    // Include every row, even evidence that export legitimately omits as unrelated.
    let projection: String = sqlx::query_scalar(
        "SELECT json_array(
            (SELECT json_group_array(json_array(slug, name, url))
             FROM (SELECT * FROM authorities ORDER BY slug)),
            (SELECT json_group_array(json_array(
                id, authority_slug, title, body, tags, fyi_url, fyi_request_id,
                status, created_at, updated_at, last_seen_at, last_event_title))
             FROM (SELECT * FROM tracked_requests ORDER BY id)),
            (SELECT json_group_array(json_array(
                id, tracked_request_id, kind, old_status, new_status, occurred_at))
             FROM (SELECT * FROM local_request_events ORDER BY id)),
            (SELECT json_group_array(json_array(
                id, feed_url, event_id, title, link, published, summary,
                request_id_guess, tracked_request_id, raw_json, seen_at))
             FROM (SELECT * FROM feed_events ORDER BY id)),
            (SELECT json_group_array(json_array(
                id, fyi_request_id, source_url, raw_json, fetched_at))
             FROM (SELECT * FROM request_snapshots ORDER BY id)),
            (SELECT json_group_array(json_array(name, seq))
             FROM (SELECT * FROM sqlite_sequence ORDER BY name)))",
    )
    .fetch_one(db.pool())
    .await
    .unwrap();
    serde_json::from_str(&projection).unwrap()
}

struct Workspace(PathBuf);

impl Workspace {
    fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let sequence = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let directory = std::env::temp_dir().join(format!(
            "fyi-transfer-concurrent-{}-{nonce}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&directory).unwrap();
        Self(directory)
    }

    async fn database(&self, name: &str) -> DbPool {
        let path = self.0.join(name);
        fs::write(&path, []).unwrap();
        let db = DbPool::new(&format!("sqlite://{}", path.display()))
            .await
            .unwrap();
        db.run_migrations().await.unwrap();
        db
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn duplicate_nullable_keys_are_rejected_before_normalization_at_every_record_level() {
    let document = native_document();
    let encoded = serde_json::to_string(&document).unwrap();
    for field in ["url", "tags", "old_status", "summary", "fetched_at"] {
        let key = format!("\"{field}\":");
        assert!(encoded.contains(&key));
        let duplicate = encoded.replacen(&key, &format!("{key}null,{key}"), 1);
        assert!(parse_local_import(duplicate.as_bytes()).is_err(), "{field}");
    }
    let legacy = serde_json::to_string(&document["requests"]).unwrap();
    let duplicate = legacy.replacen("\"tags\":", "\"tags\":null,\"tags\":", 1);
    assert!(parse_local_import(duplicate.as_bytes()).is_err());
}

#[tokio::test]
async fn oversized_export_is_rejected_without_disclosing_the_private_body() {
    let db = database().await;
    db.import_local_requests(&parsed(&native_document()), false)
        .await
        .unwrap();
    let mut document = db.export_local_requests().await.unwrap();
    assert!(encode_local_export(&document).is_ok());
    document.requests[0].body =
        format!("SYNTHETIC-PRIVATE-OVERSIZE:{}", "x".repeat(17 * 1024 * 1024));
    let error = encode_local_export(&document).unwrap_err();
    assert!(matches!(&error, LocalError::InvalidInput(_)));
    let diagnostic = error.to_string();
    assert!(diagnostic.contains("16 MiB"));
    assert!(!diagnostic.contains("SYNTHETIC-PRIVATE-OVERSIZE"));
    assert!(diagnostic.len() < 256);
}

#[tokio::test]
async fn late_authority_failure_rolls_back_earlier_explicit_replacement_updates() {
    let db = database().await;
    db.import_local_authorities(&[LocalAuthority {
        slug: "synthetic-local".to_owned(),
        name: "Original retained catalogue name".to_owned(),
        url: None,
    }])
    .await
    .unwrap();
    let before = state(&db).await;
    sqlx::query(
        "CREATE TRIGGER reject_late_authority BEFORE INSERT ON authorities
         WHEN NEW.slug = 'synthetic-maori'
         BEGIN SELECT RAISE(ABORT, 'synthetic authority failure'); END",
    )
    .execute(db.pool())
    .await
    .unwrap();
    assert!(db
        .import_local_requests(&parsed(&native_document()), true)
        .await
        .is_err());
    assert_eq!(state(&db).await, before);
}

#[tokio::test]
async fn late_history_failure_rolls_back_authorities_requests_history_and_sequences() {
    let db = database().await;
    let before = state(&db).await;
    sqlx::query(
        "CREATE TRIGGER reject_late_history BEFORE INSERT ON local_request_events
         WHEN NEW.id = 42
         BEGIN SELECT RAISE(ABORT, 'synthetic history failure'); END",
    )
    .execute(db.pool())
    .await
    .unwrap();
    assert!(db
        .import_local_requests(&parsed(&native_document()), false)
        .await
        .is_err());
    assert_eq!(state(&db).await, before);
}

#[tokio::test]
async fn late_snapshot_failure_rolls_back_the_entire_provenance_import() {
    let db = database().await;
    let before = state(&db).await;
    sqlx::query(
        "CREATE TRIGGER reject_late_snapshot BEFORE INSERT ON request_snapshots
         WHEN NEW.id = 9
         BEGIN SELECT RAISE(ABORT, 'synthetic snapshot failure'); END",
    )
    .execute(db.pool())
    .await
    .unwrap();
    assert!(db
        .import_local_requests(&parsed(&native_document()), false)
        .await
        .is_err());
    assert_eq!(state(&db).await, before);
}

#[tokio::test]
async fn existing_event_ids_cannot_be_reparented_by_merge_or_explicit_replace() {
    let db = database().await;
    let mut document = native_document();
    assert_eq!(
        db.import_local_requests(&parsed(&document), false)
            .await
            .unwrap(),
        2
    );
    let before = state(&db).await;
    for event in document["events"].as_array_mut().unwrap() {
        event["tracked_request_id"] = json!(2147483650_i64);
    }
    document["requests"][1]["status"] = json!("waiting_response");
    let replacement = parsed(&document);
    // Prove the changed document is internally valid: only existing event identity
    // prevents replacement, rather than a disconnected history or final status.
    let control = database().await;
    assert_eq!(
        control
            .import_local_requests(&replacement, false)
            .await
            .unwrap(),
        2
    );
    for replace in [false, true] {
        assert!(db
            .import_local_requests(&replacement, replace)
            .await
            .is_err());
        assert_eq!(state(&db).await, before);
    }
}

#[tokio::test]
async fn native_export_order_is_independent_of_import_order_and_preserves_high_water_marks() {
    let document = native_document();
    let mut reversed = document.clone();
    for collection in [
        "authorities",
        "requests",
        "events",
        "feed_events",
        "request_snapshots",
    ] {
        reversed[collection].as_array_mut().unwrap().reverse();
    }
    let first = database().await;
    let second = database().await;
    first
        .import_local_requests(&parsed(&document), false)
        .await
        .unwrap();
    second
        .import_local_requests(&parsed(&reversed), false)
        .await
        .unwrap();
    let first_bytes = exported(&first).await;
    assert_eq!(exported(&first).await, first_bytes);
    assert_eq!(exported(&second).await, first_bytes);
    assert_eq!(serde_json::from_slice::<Value>(&first_bytes).unwrap(), document);
}

#[tokio::test]
async fn export_rejects_invalid_stored_references_and_allocator_damage_in_omitted_rows() {
    let db = database().await;
    db.import_local_requests(&parsed(&native_document()), false)
        .await
        .unwrap();
    sqlx::query("UPDATE feed_events SET tracked_request_id = 404 WHERE id = 3")
        .execute(db.pool())
        .await
        .unwrap();
    let invalid_reference = state(&db).await;
    assert!(db.export_local_requests().await.is_err());
    assert_eq!(state(&db).await, invalid_reference);

    sqlx::query("UPDATE feed_events SET tracked_request_id = 7 WHERE id = 3")
        .execute(db.pool())
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO feed_events (id, feed_url, tracked_request_id)
         VALUES (100, 'https://unrelated.example.invalid/feed', NULL)",
    )
    .execute(db.pool())
    .await
    .unwrap();
    // An unrelated row is intentionally omitted, but its allocator state is retained.
    let valid: Value = serde_json::from_slice(&exported(&db).await).unwrap();
    assert_eq!(valid["feed_events"].as_array().unwrap().len(), 2);
    assert_eq!(valid["sequences"]["feed_events"], 100);
    sqlx::query("UPDATE sqlite_sequence SET seq = 99 WHERE name = 'feed_events'")
        .execute(db.pool())
        .await
        .unwrap();
    let invalid_allocator = state(&db).await;
    assert!(db.export_local_requests().await.is_err());
    assert_eq!(state(&db).await, invalid_allocator);
}

#[tokio::test]
async fn legacy_array_conflict_rolls_back_a_preceding_new_identity_without_remapping() {
    let db = database().await;
    let document = native_document();
    db.import_local_requests(&parsed(&document), false)
        .await
        .unwrap();
    let before = state(&db).await;
    let mut new_request = document["requests"][0].clone();
    new_request["id"] = json!(25);
    let mut conflicting = document["requests"][0].clone();
    conflicting["body"] = json!("SYNTHETIC-CHANGED-BODY");
    let incoming = parsed(&json!([new_request, conflicting]));
    assert!(db.import_local_requests(&incoming, false).await.is_err());
    assert_eq!(state(&db).await, before);
    assert!(db.get_local_request(25).await.unwrap().is_none());
}

#[tokio::test]
async fn concurrent_imports_are_idempotent_or_conflict_atomically_on_shared_file_databases() {
    let workspace = Workspace::new();
    let db = workspace.database("identical.db").await;
    let document = native_document();
    let incoming = parsed(&document);
    let (first, second) = tokio::join!(
        db.import_local_requests(&incoming, false),
        db.import_local_requests(&incoming, false),
    );
    assert_eq!(first.unwrap(), 2);
    assert_eq!(second.unwrap(), 2);
    assert_eq!(
        serde_json::from_slice::<Value>(&exported(&db).await).unwrap(),
        document
    );
    db.pool().close().await;

    let competing = workspace.database("competing.db").await;
    let mut alternative = document.clone();
    alternative["requests"][0]["body"] = json!("SYNTHETIC-COMPETING-BODY");
    let other = parsed(&alternative);
    let (first, second) = tokio::join!(
        competing.import_local_requests(&incoming, false),
        competing.import_local_requests(&other, false),
    );
    assert_ne!(first.is_ok(), second.is_ok());
    let expected = if first.is_ok() { document } else { alternative };
    assert_eq!(
        serde_json::from_slice::<Value>(&exported(&competing).await).unwrap(),
        expected
    );
    competing.pool().close().await;
}
