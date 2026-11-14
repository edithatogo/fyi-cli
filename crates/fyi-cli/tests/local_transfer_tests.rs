//! Private local transfer and explicit legacy adoption, without source acquisition.

use fyi_core::db::DbPool;
use serde_json::{json, Map, Value};
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const LEGACY_SQL: &str = include_str!("../../../conformance/local-management/legacy-v1.sql");
const LEGACY_EXPECTED: &str =
    include_str!("../../../conformance/local-management/legacy-v1-expected.json");
const AUTHORITIES: &str = "SELECT json_group_array(json_object(
    'slug', slug, 'name', name, 'url', url))
    FROM (SELECT * FROM authorities ORDER BY slug)";
const TRACKED: &str = "SELECT json_group_array(json_object(
    'id', id, 'authority_slug', authority_slug, 'title', title, 'body', body,
    'tags', tags, 'fyi_url', fyi_url, 'fyi_request_id', fyi_request_id,
    'status', status, 'created_at', created_at, 'updated_at', updated_at,
    'last_seen_at', last_seen_at, 'last_event_title', last_event_title))
    FROM (SELECT * FROM tracked_requests ORDER BY id)";
const EVENTS: &str = "SELECT json_group_array(json_object(
    'id', id, 'tracked_request_id', tracked_request_id, 'kind', kind,
    'old_status', old_status, 'new_status', new_status, 'occurred_at', occurred_at))
    FROM (SELECT * FROM local_request_events ORDER BY id)";
const FEEDS: &str = "SELECT json_group_array(json_object(
    'id', id, 'feed_url', feed_url, 'event_id', event_id, 'title', title,
    'link', link, 'published', published, 'summary', summary,
    'request_id_guess', request_id_guess, 'tracked_request_id', tracked_request_id,
    'raw_json', raw_json, 'seen_at', seen_at))
    FROM (SELECT * FROM feed_events ORDER BY id)";
const SNAPSHOTS: &str = "SELECT json_group_array(json_object(
    'id', id, 'fyi_request_id', fyi_request_id, 'source_url', source_url,
    'raw_json', raw_json, 'fetched_at', fetched_at))
    FROM (SELECT * FROM request_snapshots ORDER BY id)";
const SEQUENCES: &str = "SELECT json_group_array(json_object('name', name, 'seq', seq))
    FROM (SELECT * FROM sqlite_sequence ORDER BY name)";
const REMOTE: &str = "SELECT json_array(
    (SELECT json_group_array(json_array(id, title, body, user_name, status,
        created_at, updated_at, url, tags)) FROM (SELECT * FROM requests ORDER BY id)),
    (SELECT json_group_array(json_array(id, request_id, direction, body, sent_at,
        state, attachments)) FROM (SELECT * FROM correspondence ORDER BY id)),
    (SELECT json_group_array(json_array(request_id, last_synced_at, remote_updated_at,
        local_updated_at, sync_status, conflict_version, remote_request_id))
        FROM (SELECT * FROM sync_metadata ORDER BY request_id)),
    (SELECT json_group_array(json_array(id, request_id, field_name, old_value,
        new_value, changed_at, synced_at)) FROM (SELECT * FROM sync_field_changes ORDER BY id)),
    (SELECT json_group_array(json_array(id, request_id, operation, payload, status,
        remote_request_id, attempts, last_error, created_at, updated_at))
        FROM (SELECT * FROM sync_outgoing_queue ORDER BY id)))";

struct Workspace(PathBuf);

impl Workspace {
    fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let sequence = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "fyi-local-transfer-{}-{nonce}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_fyi-cli"))
            .current_dir(&self.0)
            .args(args)
            .output()
            .unwrap()
    }

    fn success(&self, args: &[&str]) -> String {
        let output = self.run(args);
        assert!(output.status.success(), "{args:?}: {output:?}");
        String::from_utf8(output.stdout).unwrap()
    }

    fn json(&self, args: &[&str]) -> Value {
        let mut command = vec!["--output-format", "json"];
        command.extend_from_slice(args);
        serde_json::from_str(&self.success(&command)).unwrap()
    }

    fn write_json(&self, name: &str, document: &Value) {
        fs::write(self.0.join(name), serde_json::to_vec(document).unwrap()).unwrap();
    }

    fn initialize(&self, database: &str) {
        self.success(&["init-db", "--db", database]);
    }

    async fn database(&self, name: &str) -> DbPool {
        DbPool::new(&format!("sqlite://{}?mode=rwc", self.0.join(name).display()))
            .await
            .unwrap()
    }

    async fn legacy(&self) {
        let db = self.database("legacy.db").await;
        sqlx::raw_sql(LEGACY_SQL).execute(db.pool()).await.unwrap();
        db.pool().close().await;
    }

    fn migrate(&self) {
        self.success(&["init-db", "--migrate-legacy", "--db", "legacy.db"]);
    }

    fn export(&self, database: &str, output: &str) -> Value {
        let acknowledgement =
            self.json(&["export-requests", "--db", database, "--output", output]);
        let document: Value =
            serde_json::from_slice(&fs::read(self.0.join(output)).unwrap()).unwrap();
        assert_eq!(
            acknowledgement,
            json!({
                "output": output,
                "exported_requests": document["requests"].as_array().unwrap().len(),
                "schema_version": 1
            })
        );
        document
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn expected() -> Value {
    serde_json::from_str(LEGACY_EXPECTED).unwrap()
}

fn native_document() -> Value {
    serde_json::from_str(include_str!(
        "../../../conformance/local-management/requests-v1-transfer.json"
    ))
    .unwrap()
}

async fn rows(db: &DbPool, query: &'static str) -> Value {
    let output: String = sqlx::query_scalar(query)
        .fetch_one(db.pool())
        .await
        .unwrap();
    serde_json::from_str(&output).unwrap()
}

async fn legacy_snapshot(db: &DbPool) -> Value {
    let mut snapshot = Map::new();
    for (name, query) in [
        ("authorities", AUTHORITIES),
        ("tracked_requests", TRACKED),
        ("feed_events", FEEDS),
        ("request_snapshots", SNAPSHOTS),
        ("sqlite_sequence", SEQUENCES),
        (
            "run_log",
            "SELECT json_group_array(json_object('id', id, 'job_name', job_name,
             'status', status, 'detail', detail, 'ran_at', ran_at))
             FROM (SELECT * FROM run_log ORDER BY id)",
        ),
        (
            "http_cache",
            "SELECT json_group_array(json_object('url', url, 'etag', etag,
             'last_modified', last_modified, 'response_body', response_body,
             'cached_at', cached_at)) FROM (SELECT * FROM http_cache ORDER BY url)",
        ),
        (
            "shared_rate_limit_state",
            "SELECT json_group_array(json_object('name', name,
             'next_allowed_at', next_allowed_at, 'last_acquired_at', last_acquired_at,
             'last_owner_id', last_owner_id, 'last_sleep_seconds', last_sleep_seconds,
             'interval_seconds', interval_seconds, 'jitter_seconds', jitter_seconds))
             FROM (SELECT * FROM shared_rate_limit_state ORDER BY name)",
        ),
        (
            "shared_rate_limit_events",
            "SELECT json_group_array(json_object('id', id, 'name', name, 'kind', kind,
             'owner_id', owner_id, 'status_code', status_code,
             'delay_seconds', delay_seconds, 'next_allowed_at', next_allowed_at,
             'observed_at', observed_at))
             FROM (SELECT * FROM shared_rate_limit_events ORDER BY id)",
        ),
    ] {
        snapshot.insert(name.to_owned(), rows(db, query).await);
    }
    Value::Object(snapshot)
}

async fn local_snapshot(db: &DbPool) -> Value {
    let mut snapshot = Map::new();
    for (name, query) in [
        ("authorities", AUTHORITIES),
        ("tracked_requests", TRACKED),
        ("events", EVENTS),
        ("feed_events", FEEDS),
        ("request_snapshots", SNAPSHOTS),
        ("sqlite_sequence", SEQUENCES),
        (
            "migrations",
            "SELECT json_group_array(json_object('version', version,
             'description', description, 'installed_on', installed_on,
             'success', success, 'checksum', hex(checksum),
             'execution_time', execution_time))
             FROM (SELECT * FROM _sqlx_migrations ORDER BY version)",
        ),
    ] {
        snapshot.insert(name.to_owned(), rows(db, query).await);
    }
    Value::Object(snapshot)
}

async fn schema_snapshot(db: &DbPool) -> Value {
    rows(
        db,
        "SELECT json_group_array(json_object('type', type, 'name', name,
         'tbl_name', tbl_name, 'sql', sql))
         FROM (SELECT * FROM sqlite_schema ORDER BY type, name)",
    )
    .await
}

async fn assert_remote_tables_empty(db: &DbPool) {
    let counts: (i64, i64, i64, i64, i64) = sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM requests),
                (SELECT COUNT(*) FROM correspondence),
                (SELECT COUNT(*) FROM sync_metadata),
                (SELECT COUNT(*) FROM sync_field_changes),
                (SELECT COUNT(*) FROM sync_outgoing_queue)",
    )
    .fetch_one(db.pool())
    .await
    .unwrap();
    assert_eq!(counts, (0, 0, 0, 0, 0));
}

fn assert_private_failure(output: &Output) {
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(output.stdout.is_empty());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(!error.contains("SYNTHETIC-PRIVATE"));
    assert!(!error.contains("SYNTHETIC-CHANGED-BODY"));
}

#[tokio::test]
async fn legacy_migration_is_explicit_lossless_and_repeatable() {
    let work = Workspace::new();
    work.legacy().await;
    let db = work.database("legacy.db").await;
    let original = legacy_snapshot(&db).await;
    let schema = schema_snapshot(&db).await;
    assert_eq!(original, expected()["tables"]);
    db.pool().close().await;

    assert_private_failure(&work.run(&["init-db", "--db", "legacy.db"]));
    let db = work.database("legacy.db").await;
    assert_eq!(legacy_snapshot(&db).await, original);
    assert_eq!(schema_snapshot(&db).await, schema);
    db.pool().close().await;

    work.migrate();
    let db = work.database("legacy.db").await;
    assert_eq!(legacy_snapshot(&db).await, original);
    assert_eq!(rows(&db, EVENTS).await, json!([]));
    assert_remote_tables_empty(&db).await;
    let migrated = local_snapshot(&db).await;
    db.pool().close().await;
    assert_eq!(
        work.success(&["list-requests", "--db", "legacy.db"]),
        expected()["python_outputs"]["list_requests"]["stdout"]
            .as_str()
            .unwrap()
    );
    // Preserve the frozen projections and timestamp bytes while ordering actual instants.
    // Python orders the same mixed-format timestamps lexically instead.
    let reference = expected();
    let timeline = &reference["python_outputs"]["request_timeline"]["7"];
    let chronological: Vec<_> = [0, 1, 3, 2, 5, 4]
        .into_iter()
        .map(|index| timeline[index].clone())
        .collect();
    assert_eq!(
        work.json(&["request-timeline", "7", "--db", "legacy.db"]),
        json!(chronological)
    );
    assert_eq!(
        work.json(&["request-timeline", "2147483650", "--db", "legacy.db"]),
        json!([])
    );
    assert_private_failure(&work.run(&["request-timeline", "404", "--db", "legacy.db"]));
    work.migrate();
    let db = work.database("legacy.db").await;
    assert_eq!(legacy_snapshot(&db).await, original);
    assert_eq!(local_snapshot(&db).await, migrated);
    db.pool().close().await;
    assert_eq!(
        work.json(&[
            "register-request",
            "synthetic-local",
            "Synthetic next request",
            "Synthetic body",
            "--db",
            "legacy.db",
        ]),
        json!({"id": 2147483651_i64})
    );
}

#[tokio::test]
async fn malformed_orphaned_and_future_legacy_schemas_are_unchanged_on_failure() {
    for mutation in [
        "ALTER TABLE tracked_requests ADD COLUMN unexpected TEXT",
        "UPDATE feed_events SET tracked_request_id = 404 WHERE id = 3",
        "PRAGMA user_version = 99",
    ] {
        let work = Workspace::new();
        work.legacy().await;
        let db = work.database("legacy.db").await;
        sqlx::query(mutation).execute(db.pool()).await.unwrap();
        let original = legacy_snapshot(&db).await;
        let schema = schema_snapshot(&db).await;
        let version: i64 = sqlx::query_scalar("PRAGMA user_version")
            .fetch_one(db.pool())
            .await
            .unwrap();
        db.pool().close().await;
        assert_private_failure(&work.run(&[
            "init-db",
            "--migrate-legacy",
            "--db",
            "legacy.db",
        ]));
        let db = work.database("legacy.db").await;
        assert_eq!(legacy_snapshot(&db).await, original);
        assert_eq!(schema_snapshot(&db).await, schema);
        let after_version: i64 = sqlx::query_scalar("PRAGMA user_version")
            .fetch_one(db.pool())
            .await
            .unwrap();
        assert_eq!(after_version, version);
        let ledger: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_schema WHERE name = '_sqlx_migrations'",
        )
        .fetch_one(db.pool())
        .await
        .unwrap();
        assert_eq!(ledger, 0);
        db.pool().close().await;
    }
}

#[tokio::test]
async fn native_roundtrip_preserves_provenance_nullable_fields_and_deleted_highwater() {
    let work = Workspace::new();
    work.legacy().await;
    work.migrate();
    work.success(&["set-status", "7", "successful", "--db", "legacy.db"]);
    let db = work.database("legacy.db").await;
    sqlx::query("UPDATE sqlite_sequence SET seq = 4000000000 WHERE name = 'tracked_requests'")
        .execute(db.pool())
        .await
        .unwrap();
    sqlx::query("UPDATE sqlite_sequence SET seq = 100 WHERE name = 'local_request_events'")
        .execute(db.pool())
        .await
        .unwrap();
    db.pool().close().await;

    let document = work.export("legacy.db", "native.json");
    let reference = native_document();
    assert_eq!(document["format"], "fyi-local-requests");
    assert_eq!(document["schema_version"], 1);
    assert_eq!(document.as_object().unwrap().len(), 8);
    assert_eq!(document["authorities"], reference["authorities"]);
    assert_eq!(document["feed_events"], reference["feed_events"]);
    assert_eq!(document["request_snapshots"], reference["request_snapshots"]);
    assert_eq!(document["requests"][1], reference["requests"][1]);
    assert_eq!(
        document["requests"][0]["body"],
        reference["requests"][0]["body"]
    );
    assert_eq!(document["events"].as_array().unwrap().len(), 1);
    assert_eq!(document["events"][0]["tracked_request_id"], 7);
    assert_eq!(document["sequences"]["tracked_requests"], 4000000000_i64);
    assert_eq!(document["sequences"]["local_request_events"], 100);
    assert_eq!(document["sequences"]["feed_events"], 19);
    assert_eq!(document["sequences"]["request_snapshots"], 14);
    for row in document["requests"].as_array().unwrap() {
        assert_eq!(row.as_object().unwrap().len(), 12);
    }

    work.initialize("restored.db");
    assert_eq!(
        work.success(&["import-requests", "native.json", "--db", "restored.db"]),
        "2\n"
    );
    assert_eq!(work.export("restored.db", "restored.json"), document);
    assert_eq!(
        fs::read(work.0.join("native.json")).unwrap(),
        fs::read(work.0.join("restored.json")).unwrap()
    );
    let db = work.database("restored.db").await;
    assert_remote_tables_empty(&db).await;
    assert!(sqlx::query("PRAGMA foreign_key_check")
        .fetch_all(db.pool())
        .await
        .unwrap()
        .is_empty());
    db.pool().close().await;
    assert_eq!(
        work.json(&[
            "register-request",
            "synthetic-local",
            "Synthetic next request",
            "Synthetic body",
            "--db",
            "restored.db",
        ]),
        json!({"id": 4000000001_i64})
    );
}

#[tokio::test]
async fn identical_native_import_is_a_database_noop_with_stable_acknowledgements() {
    let work = Workspace::new();
    work.initialize("local.db");
    work.write_json("native.json", &native_document());
    assert_eq!(
        work.json(&["import-requests", "native.json", "--db", "local.db"]),
        json!({"imported_requests": 2})
    );
    let db = work.database("local.db").await;
    let original = local_snapshot(&db).await;
    db.pool().close().await;
    for _ in 0..2 {
        assert_eq!(
            work.success(&["import-requests", "native.json", "--db", "local.db"]),
            "2\n"
        );
        let db = work.database("local.db").await;
        assert_eq!(local_snapshot(&db).await, original);
        assert_remote_tables_empty(&db).await;
        db.pool().close().await;
    }
}

#[tokio::test]
async fn conflicting_late_request_rolls_back_new_rows_and_preserves_old_body() {
    let work = Workspace::new();
    work.initialize("local.db");
    let mut document = native_document();
    work.write_json("native.json", &document);
    work.success(&["import-requests", "native.json", "--db", "local.db"]);
    let db = work.database("local.db").await;
    let original = local_snapshot(&db).await;
    db.pool().close().await;

    document["authorities"].as_array_mut().unwrap().push(json!({
        "slug": "synthetic-new",
        "name": "SYNTHETIC-PRIVATE-AUTHORITY",
        "url": null
    }));
    let mut new_request = document["requests"][0].clone();
    new_request["id"] = json!(25);
    new_request["authority_slug"] = json!("synthetic-new");
    document["requests"]
        .as_array_mut()
        .unwrap()
        .insert(0, new_request);
    document["requests"][1]["body"] = json!("SYNTHETIC-CHANGED-BODY");
    work.write_json("conflict.json", &document);
    let failure = work.run(&[
        "--output-format",
        "json",
        "import-requests",
        "conflict.json",
        "--db",
        "local.db",
    ]);
    assert_private_failure(&failure);
    let error: Value = serde_json::from_slice(&failure.stderr).unwrap();
    assert_eq!(error["error"]["kind"], "local");
    let db = work.database("local.db").await;
    assert_eq!(local_snapshot(&db).await, original);
    assert_remote_tables_empty(&db).await;
    db.pool().close().await;
}

#[tokio::test]
async fn strict_native_documents_reject_ambiguous_versions_fields_ids_and_references() {
    let work = Workspace::new();
    work.initialize("local.db");
    let db = work.database("local.db").await;
    let original = local_snapshot(&db).await;
    db.pool().close().await;
    let baseline = native_document();
    let mut variants = Vec::new();
    for (field, value) in [
        ("schema_version", json!(2)),
        ("schema_version", json!(true)),
        ("schema_version", json!(1.0)),
        ("format", json!("unknown-local-format")),
        ("unexpected", json!("SYNTHETIC-PRIVATE-UNKNOWN")),
    ] {
        let mut document = baseline.clone();
        document[field] = value;
        variants.push(serde_json::to_string(&document).unwrap());
    }
    let mut missing_nullable = baseline.clone();
    missing_nullable["requests"][0]
        .as_object_mut()
        .unwrap()
        .remove("tags");
    variants.push(serde_json::to_string(&missing_nullable).unwrap());
    let mut duplicate_id = baseline.clone();
    duplicate_id["requests"]
        .as_array_mut()
        .unwrap()
        .push(baseline["requests"][0].clone());
    variants.push(serde_json::to_string(&duplicate_id).unwrap());
    for invalid_id in [json!(0), json!(-1), json!(true), json!(7.5)] {
        let mut document = baseline.clone();
        document["requests"][0]["id"] = invalid_id;
        variants.push(serde_json::to_string(&document).unwrap());
    }
    let mut orphan = baseline.clone();
    orphan["feed_events"][0]["tracked_request_id"] = json!(404);
    variants.push(serde_json::to_string(&orphan).unwrap());
    let mut short_sequence = baseline.clone();
    short_sequence["sequences"]["tracked_requests"] = json!(1);
    variants.push(serde_json::to_string(&short_sequence).unwrap());
    let encoded = serde_json::to_string(&baseline).unwrap();
    variants.push(format!("{{\"schema_version\":1,{}", &encoded[1..]));
    variants.push(encoded.replacen(
        "\"body\":",
        "\"body\":\"SYNTHETIC-PRIVATE-DUPLICATE\",\"body\":",
        1,
    ));
    variants.push(format!("{encoded} null"));
    for input in variants {
        fs::write(work.0.join("invalid.json"), input).unwrap();
        assert_private_failure(&work.run(&[
            "import-requests",
            "invalid.json",
            "--db",
            "local.db",
        ]));
        let db = work.database("local.db").await;
        assert_eq!(local_snapshot(&db).await, original);
        db.pool().close().await;
    }
}

#[tokio::test]
async fn legacy_arrays_preserve_original_ids_and_reject_missing_identity() {
    let work = Workspace::new();
    work.initialize("local.db");
    fs::write(
        work.0.join("authorities.csv"),
        "slug,name\nsynthetic-maori,Synthetic Authority\nsynthetic-local,Synthetic Local\n",
    )
    .unwrap();
    work.success(&["import-authorities", "authorities.csv", "--db", "local.db"]);
    let fixture = expected();
    let requests = fixture["tables"]["tracked_requests"].clone();
    work.write_json("legacy.json", &requests);
    assert_eq!(
        work.success(&["import-requests", "legacy.json", "--db", "local.db"]),
        "2\n"
    );
    let db = work.database("local.db").await;
    assert_eq!(rows(&db, TRACKED).await, requests);
    assert_eq!(rows(&db, EVENTS).await, json!([]));
    let original = local_snapshot(&db).await;
    db.pool().close().await;
    work.success(&["import-requests", "legacy.json", "--db", "local.db"]);
    let mut missing_id = requests.clone();
    missing_id[0].as_object_mut().unwrap().remove("id");
    work.write_json("missing-id.json", &missing_id);
    assert_private_failure(&work.run(&[
        "import-requests",
        "missing-id.json",
        "--db",
        "local.db",
    ]));
    let db = work.database("local.db").await;
    assert_eq!(local_snapshot(&db).await, original);
    assert_remote_tables_empty(&db).await;
    db.pool().close().await;
}

#[tokio::test]
async fn replace_changes_only_the_local_set_and_never_resets_id_allocation() {
    let work = Workspace::new();
    work.initialize("local.db");
    fs::write(work.0.join("authorities.csv"), "slug,name\nalpha,Alpha\n").unwrap();
    work.success(&["import-authorities", "authorities.csv", "--db", "local.db"]);
    for title in ["Keep request", "Remove request"] {
        work.success(&[
            "register-request",
            "alpha",
            title,
            "Original body",
            "--db",
            "local.db",
        ]);
    }
    let db = work.database("local.db").await;
    sqlx::raw_sql(
        "INSERT INTO requests (id, title, body, status)
         VALUES (1, 'Synthetic remote request', 'SYNTHETIC-PRIVATE-REMOTE', 'draft');
         INSERT INTO correspondence (id, request_id, direction, body, sent_at)
         VALUES (1, 1, 'incoming', 'Synthetic remote correspondence', '2026-01-02T00:00:00Z');
         INSERT INTO sync_metadata (request_id, local_updated_at, sync_status)
         VALUES (1, '2026-01-02T00:00:00Z', 'dirty');
         INSERT INTO sync_field_changes (request_id, field_name, old_value, new_value)
         VALUES (1, 'body', 'Synthetic old', 'Synthetic new');
         INSERT INTO sync_outgoing_queue (request_id, payload)
         VALUES (1, '{\"synthetic\":true}');",
    )
    .execute(db.pool())
    .await
    .unwrap();
    let remote_before = rows(&db, REMOTE).await;
    db.pool().close().await;
    let mut replacement = work.export("local.db", "original.json");
    replacement["requests"]
        .as_array_mut()
        .unwrap()
        .retain(|row| row["id"] == 1);
    replacement["events"]
        .as_array_mut()
        .unwrap()
        .retain(|row| row["tracked_request_id"] == 1);
    replacement["requests"][0]["body"] = json!("SYNTHETIC-CHANGED-BODY");
    work.write_json("replacement.json", &replacement);
    assert_eq!(
        work.json(&[
            "import-requests",
            "replacement.json",
            "--replace",
            "--db",
            "local.db",
        ]),
        json!({"imported_requests": 1})
    );
    let db = work.database("local.db").await;
    assert_eq!(rows(&db, TRACKED).await, replacement["requests"]);
    assert_eq!(rows(&db, EVENTS).await, replacement["events"]);
    let remote_after = rows(&db, REMOTE).await;
    assert_eq!(remote_after, remote_before);
    db.pool().close().await;
    assert_eq!(
        work.json(&[
            "register-request",
            "alpha",
            "Next request",
            "Synthetic body",
            "--db",
            "local.db",
        ]),
        json!({"id": 3})
    );
}

#[tokio::test]
async fn replace_rejects_orphaned_detached_or_conflicting_provenance_atomically() {
    let work = Workspace::new();
    work.legacy().await;
    work.migrate();
    let baseline = work.export("legacy.db", "original.json");
    let db = work.database("legacy.db").await;
    let original = legacy_snapshot(&db).await;
    let local = local_snapshot(&db).await;
    db.pool().close().await;
    let mut variants = Vec::new();
    let mut omitted = baseline.clone();
    omitted["requests"]
        .as_array_mut()
        .unwrap()
        .retain(|row| row["id"] != 7);
    omitted["feed_events"] = json!([]);
    omitted["request_snapshots"] = json!([]);
    variants.push(omitted);
    let mut detached = baseline.clone();
    detached["requests"][0]["fyi_request_id"] = json!(9008);
    detached["request_snapshots"] = json!([]);
    variants.push(detached);
    let mut changed_feed = baseline.clone();
    changed_feed["feed_events"][0]["raw_json"] = json!("SYNTHETIC-CHANGED-BODY");
    variants.push(changed_feed);
    let mut changed_snapshot = baseline.clone();
    changed_snapshot["request_snapshots"][0]["raw_json"] = json!("SYNTHETIC-CHANGED-BODY");
    variants.push(changed_snapshot);
    for document in variants {
        work.write_json("invalid-replacement.json", &document);
        assert_private_failure(&work.run(&[
            "import-requests",
            "invalid-replacement.json",
            "--replace",
            "--db",
            "legacy.db",
        ]));
        let db = work.database("legacy.db").await;
        assert_eq!(legacy_snapshot(&db).await, original);
        assert_eq!(local_snapshot(&db).await, local);
        db.pool().close().await;
    }
}

#[tokio::test]
async fn legacy_array_replace_cannot_silently_remove_native_event_history() {
    let work = Workspace::new();
    work.initialize("local.db");
    fs::write(work.0.join("authorities.csv"), "slug,name\nalpha,Alpha\n").unwrap();
    work.success(&["import-authorities", "authorities.csv", "--db", "local.db"]);
    work.success(&[
        "register-request",
        "alpha",
        "Local request",
        "Original body",
        "--db",
        "local.db",
    ]);
    let db = work.database("local.db").await;
    let original = local_snapshot(&db).await;
    let mut requests = rows(&db, TRACKED).await;
    requests[0]["body"] = json!("SYNTHETIC-CHANGED-BODY");
    db.pool().close().await;
    work.write_json("legacy.json", &requests);
    assert_private_failure(&work.run(&[
        "import-requests",
        "legacy.json",
        "--replace",
        "--db",
        "local.db",
    ]));
    let db = work.database("local.db").await;
    assert_eq!(local_snapshot(&db).await, original);
    db.pool().close().await;
}

#[tokio::test]
async fn transfer_files_are_bounded_no_clobber_and_do_not_create_missing_databases() {
    let work = Workspace::new();
    work.initialize("local.db");
    work.write_json("native.json", &native_document());
    work.success(&["import-requests", "native.json", "--db", "local.db"]);
    let db = work.database("local.db").await;
    let original = local_snapshot(&db).await;
    db.pool().close().await;
    assert_eq!(
        work.success(&[
            "export-requests",
            "--db",
            "local.db",
            "--output",
            "export.json",
        ]),
        "export.json\n"
    );
    let exported = fs::read(work.0.join("export.json")).unwrap();
    let repeat = work.run(&[
        "export-requests",
        "--db",
        "local.db",
        "--output",
        "export.json",
    ]);
    assert!(repeat.status.success(), "{repeat:?}");
    assert_eq!(fs::read(work.0.join("export.json")).unwrap(), exported);
    fs::write(work.0.join("occupied.json"), b"Preserve existing output").unwrap();
    for destination in ["occupied.json", "local.db", "."] {
        assert_private_failure(&work.run(&[
            "export-requests",
            "--db",
            "local.db",
            "--output",
            destination,
        ]));
    }
    assert_eq!(
        fs::read(work.0.join("occupied.json")).unwrap(),
        b"Preserve existing output"
    );
    fs::write(
        work.0.join("oversized.json"),
        vec![b' '; 16 * 1024 * 1024 + 1],
    )
    .unwrap();
    for input in ["missing.json", "oversized.json", "."] {
        assert_private_failure(&work.run(&["import-requests", input, "--db", "local.db"]));
    }
    assert_private_failure(&work.run(&[
        "import-requests",
        "native.json",
        "--db",
        "missing.db",
    ]));
    assert_private_failure(&work.run(&["export-requests", "--db", "missing.db"]));
    assert!(!work.0.join("missing.db").exists());
    let db = work.database("local.db").await;
    assert_eq!(local_snapshot(&db).await, original);
    db.pool().close().await;
}

#[cfg(unix)]
#[tokio::test]
async fn export_has_private_permissions_and_cannot_follow_a_destination_symlink() {
    use std::os::unix::fs::PermissionsExt;

    let work = Workspace::new();
    work.initialize("local.db");
    work.write_json("native.json", &native_document());
    work.success(&["import-requests", "native.json", "--db", "local.db"]);
    work.export("local.db", "private.json");
    assert_eq!(
        fs::metadata(work.0.join("private.json"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    fs::write(work.0.join("target.json"), b"Preserve symlink target").unwrap();
    std::os::unix::fs::symlink(work.0.join("target.json"), work.0.join("link.json")).unwrap();
    assert_private_failure(&work.run(&[
        "export-requests",
        "--db",
        "local.db",
        "--output",
        "link.json",
    ]));
    assert_eq!(
        fs::read(work.0.join("target.json")).unwrap(),
        b"Preserve symlink target"
    );
    assert!(fs::symlink_metadata(work.0.join("link.json"))
        .unwrap()
        .file_type()
        .is_symlink());
}
