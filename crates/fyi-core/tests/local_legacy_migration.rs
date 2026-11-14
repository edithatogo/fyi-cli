//! Unsupported legacy layouts must remain unchanged after explicit adoption fails.

use fyi_core::db::DbPool;
use serde_json::{json, Map, Value};
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

const LEGACY_SQL: &str = include_str!("../../../conformance/local-management/legacy-v1.sql");
const LEGACY_EXPECTED: &str =
    include_str!("../../../conformance/local-management/legacy-v1-expected.json");
const LEDGER_SCHEMA: &str = "CREATE TABLE _sqlx_migrations (
    version BIGINT PRIMARY KEY,
    description TEXT NOT NULL,
    installed_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    success BOOLEAN NOT NULL,
    checksum BLOB NOT NULL,
    execution_time BIGINT NOT NULL
)";

const ORIGINAL_ROWS: [(&str, &str); 9] = [
    (
        "authorities",
        "SELECT json_group_array(json_object('slug', slug, 'name', name, 'url', url))
         FROM (SELECT * FROM authorities ORDER BY slug)",
    ),
    (
        "tracked_requests",
        "SELECT json_group_array(json_object('id', id, 'authority_slug', authority_slug,
         'title', title, 'body', body, 'tags', tags, 'fyi_url', fyi_url,
         'fyi_request_id', fyi_request_id, 'status', status, 'created_at', created_at,
         'updated_at', updated_at, 'last_seen_at', last_seen_at,
         'last_event_title', last_event_title))
         FROM (SELECT * FROM tracked_requests ORDER BY id)",
    ),
    (
        "feed_events",
        "SELECT json_group_array(json_object('id', id, 'feed_url', feed_url,
         'event_id', event_id, 'title', title, 'link', link, 'published', published,
         'summary', summary, 'request_id_guess', request_id_guess,
         'tracked_request_id', tracked_request_id, 'raw_json', raw_json, 'seen_at', seen_at))
         FROM (SELECT * FROM feed_events ORDER BY id)",
    ),
    (
        "request_snapshots",
        "SELECT json_group_array(json_object('id', id, 'fyi_request_id', fyi_request_id,
         'source_url', source_url, 'raw_json', raw_json, 'fetched_at', fetched_at))
         FROM (SELECT * FROM request_snapshots ORDER BY id)",
    ),
    (
        "run_log",
        "SELECT json_group_array(json_object('id', id, 'job_name', job_name,
         'status', status, 'detail', detail, 'ran_at', ran_at))
         FROM (SELECT * FROM run_log ORDER BY id)",
    ),
    (
        "shared_rate_limit_state",
        "SELECT json_group_array(json_object('name', name, 'next_allowed_at', next_allowed_at,
         'last_acquired_at', last_acquired_at, 'last_owner_id', last_owner_id,
         'last_sleep_seconds', last_sleep_seconds, 'interval_seconds', interval_seconds,
         'jitter_seconds', jitter_seconds))
         FROM (SELECT * FROM shared_rate_limit_state ORDER BY name)",
    ),
    (
        "shared_rate_limit_events",
        "SELECT json_group_array(json_object('id', id, 'name', name, 'kind', kind,
         'owner_id', owner_id, 'status_code', status_code, 'delay_seconds', delay_seconds,
         'next_allowed_at', next_allowed_at, 'observed_at', observed_at))
         FROM (SELECT * FROM shared_rate_limit_events ORDER BY id)",
    ),
    (
        "http_cache",
        "SELECT json_group_array(json_object('url', url, 'etag', etag,
         'last_modified', last_modified, 'response_body', response_body, 'cached_at', cached_at))
         FROM (SELECT * FROM http_cache ORDER BY url)",
    ),
    (
        "sqlite_sequence",
        "SELECT json_group_array(json_object('name', name, 'seq', seq))
         FROM (SELECT * FROM sqlite_sequence ORDER BY name, rowid)",
    ),
];

async fn table_exists(db: &DbPool, name: &str) -> bool {
    sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM sqlite_schema WHERE type = 'table' AND name = ?)",
    )
    .bind(name)
    .fetch_one(db.pool())
    .await
    .unwrap()
}

async fn json_rows(db: &DbPool, statement: &'static str) -> Value {
    let encoded: String = sqlx::query_scalar(statement)
        .fetch_one(db.pool())
        .await
        .unwrap();
    serde_json::from_str(&encoded).unwrap()
}

async fn snapshot(db: &DbPool) -> Value {
    let mut tables = Map::new();
    for (name, statement) in ORIGINAL_ROWS {
        if table_exists(db, name).await {
            tables.insert(name.to_owned(), json_rows(db, statement).await);
        }
    }
    let schema = json_rows(
        db,
        "SELECT json_group_array(json_object('type', type, 'name', name,
         'tbl_name', tbl_name, 'sql', sql))
         FROM (SELECT * FROM sqlite_schema ORDER BY type, name)",
    )
    .await;
    let ledger = if table_exists(db, "_sqlx_migrations").await {
        json_rows(
            db,
            "SELECT json_group_array(json_object('version', version, 'description', description,
             'installed_on', installed_on, 'success', success, 'checksum', hex(checksum),
             'execution_time', execution_time))
             FROM (SELECT * FROM _sqlx_migrations ORDER BY version)",
        )
        .await
    } else {
        Value::Null
    };
    let user_version: i64 = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(db.pool())
        .await
        .unwrap();
    let application_id: i64 = sqlx::query_scalar("PRAGMA application_id")
        .fetch_one(db.pool())
        .await
        .unwrap();
    json!({
        "tables": tables,
        "schema": schema,
        "ledger": ledger,
        "user_version": user_version,
        "application_id": application_id
    })
}

async fn legacy_database() -> DbPool {
    let db = DbPool::new_in_memory().await.unwrap();
    sqlx::raw_sql(LEGACY_SQL).execute(db.pool()).await.unwrap();
    let expected: Value = serde_json::from_str(LEGACY_EXPECTED).unwrap();
    assert_eq!(snapshot(&db).await["tables"], expected["tables"]);
    db
}

async fn assert_rejected_unchanged(db: &DbPool) {
    let before = snapshot(db).await;
    let error = db
        .migrate_legacy_local_schema()
        .await
        .expect_err("unsupported legacy adoption must fail");
    assert!(!error.to_string().contains("SYNTHETIC-PRIVATE"));
    assert_eq!(snapshot(db).await, before);
}

#[tokio::test]
async fn incomplete_or_failed_sqlx_ledger_does_not_adopt_a_python_database() {
    for (success, valid_checksum, future_version, skip_first) in [
        (true, true, false, false),
        (false, true, false, false),
        (true, false, false, false),
        (true, true, true, false),
        (true, true, false, true),
    ] {
        let db = legacy_database().await;
        sqlx::query(LEDGER_SCHEMA).execute(db.pool()).await.unwrap();
        let migrations = sqlx::migrate!("./migrations");
        let first = migrations.iter().nth(usize::from(skip_first)).unwrap();
        let checksum = if valid_checksum {
            first.checksum.as_ref().to_vec()
        } else {
            vec![0]
        };
        let version = if future_version {
            20990101000000_i64
        } else {
            first.version
        };
        sqlx::query(
            "INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time)
             VALUES (?, 'Synthetic legacy ledger', ?, ?, 0)",
        )
        .bind(version)
        .bind(success)
        .bind(checksum)
        .execute(db.pool())
        .await
        .unwrap();
        assert_rejected_unchanged(&db).await;
        assert!(!table_exists(&db, "requests").await);
        db.pool().close().await;
    }
}

#[tokio::test]
async fn a_genuine_rust_prefix_with_unadopted_legacy_tables_is_a_rejected_hybrid() {
    let db = DbPool::new_in_memory().await.unwrap();
    sqlx::migrate!("./migrations")
        .run_to(20261114000000, db.pool())
        .await
        .unwrap();
    sqlx::raw_sql(LEGACY_SQL).execute(db.pool()).await.unwrap();
    sqlx::query(
        "INSERT INTO requests (id, title, body, status)
         VALUES (7, 'Synthetic remote record', 'SYNTHETIC-PRIVATE-REMOTE', 'draft')",
    )
    .execute(db.pool())
    .await
    .unwrap();
    assert_rejected_unchanged(&db).await;
    let remote: (i64, String, String, String) =
        sqlx::query_as("SELECT id, title, body, status FROM requests")
            .fetch_one(db.pool())
            .await
            .unwrap();
    assert_eq!(
        remote,
        (
            7,
            "Synthetic remote record".to_owned(),
            "SYNTHETIC-PRIVATE-REMOTE".to_owned(),
            "draft".to_owned()
        )
    );
    assert!(!table_exists(&db, "local_request_events").await);
    db.pool().close().await;
}

#[tokio::test]
async fn an_orphaned_authority_is_not_repaired_or_partially_migrated() {
    let db = legacy_database().await;
    sqlx::query("PRAGMA foreign_keys = OFF")
        .execute(db.pool())
        .await
        .unwrap();
    sqlx::query(
        "UPDATE tracked_requests SET authority_slug = 'synthetic-missing-authority' WHERE id = 7",
    )
    .execute(db.pool())
    .await
    .unwrap();
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(db.pool())
        .await
        .unwrap();
    assert_rejected_unchanged(&db).await;
    assert!(!table_exists(&db, "_sqlx_migrations").await);
    db.pool().close().await;
}

#[tokio::test]
async fn every_known_python_table_is_required_before_adoption() {
    for statement in [
        "DROP TABLE authorities",
        "DROP TABLE tracked_requests",
        "DROP TABLE feed_events",
        "DROP TABLE request_snapshots",
        "DROP TABLE run_log",
        "DROP TABLE shared_rate_limit_state",
        "DROP TABLE shared_rate_limit_events",
        "DROP TABLE http_cache",
    ] {
        let db = legacy_database().await;
        sqlx::query("PRAGMA foreign_keys = OFF")
            .execute(db.pool())
            .await
            .unwrap();
        sqlx::query(statement).execute(db.pool()).await.unwrap();
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(db.pool())
            .await
            .unwrap();
        assert_rejected_unchanged(&db).await;
        assert!(!table_exists(&db, "_sqlx_migrations").await);
        db.pool().close().await;
    }
}

#[tokio::test]
async fn triggers_and_generated_columns_cannot_hide_in_an_otherwise_known_layout() {
    for statement in [
        "CREATE TRIGGER synthetic_unexpected_trigger AFTER INSERT ON tracked_requests
         BEGIN UPDATE run_log SET detail = 'Synthetic trigger side effect' WHERE id = 4; END",
        "ALTER TABLE tracked_requests
         ADD COLUMN synthetic_body_length INTEGER GENERATED ALWAYS AS (length(body)) VIRTUAL",
        "ALTER TABLE authorities
         ADD COLUMN synthetic_folded_name TEXT GENERATED ALWAYS AS (lower(name)) VIRTUAL",
    ] {
        let db = legacy_database().await;
        sqlx::query(statement).execute(db.pool()).await.unwrap();
        assert_rejected_unchanged(&db).await;
        assert!(!table_exists(&db, "_sqlx_migrations").await);
        db.pool().close().await;
    }
}

#[tokio::test]
async fn corrupt_autoincrement_markers_are_rejected_without_normalizing_them() {
    for statement in [
        "UPDATE sqlite_sequence SET seq = -1 WHERE name = 'tracked_requests'",
        "UPDATE sqlite_sequence SET seq = 1 WHERE name = 'tracked_requests'",
        "UPDATE sqlite_sequence SET seq = 2147483650.5 WHERE name = 'tracked_requests'",
        "UPDATE sqlite_sequence SET seq = '2147483650' WHERE name = 'tracked_requests'",
        "INSERT INTO sqlite_sequence (name, seq) VALUES ('tracked_requests', 2147483650)",
        "DELETE FROM sqlite_sequence WHERE name = 'tracked_requests'",
        "UPDATE sqlite_sequence SET seq = -1 WHERE name = 'feed_events'",
        "UPDATE sqlite_sequence SET seq = 1 WHERE name = 'request_snapshots'",
    ] {
        let db = legacy_database().await;
        sqlx::query(statement).execute(db.pool()).await.unwrap();
        assert_rejected_unchanged(&db).await;
        assert!(!table_exists(&db, "_sqlx_migrations").await);
        db.pool().close().await;
    }
}

#[tokio::test]
async fn reserved_migration_name_conflicts_leave_no_ledger_or_partial_native_schema() {
    // Whether caught during preflight or migration execution, all original objects survive.
    for (definition, insertion, contents) in [
        (
            "CREATE TABLE requests (marker TEXT NOT NULL)",
            "INSERT INTO requests VALUES ('Synthetic preserved marker')",
            "SELECT marker FROM requests",
        ),
        (
            "CREATE TABLE idx_sync_metadata_status (marker TEXT NOT NULL)",
            "INSERT INTO idx_sync_metadata_status VALUES ('Synthetic preserved marker')",
            "SELECT marker FROM idx_sync_metadata_status",
        ),
        (
            "CREATE TABLE local_request_events (marker TEXT NOT NULL)",
            "INSERT INTO local_request_events VALUES ('Synthetic preserved marker')",
            "SELECT marker FROM local_request_events",
        ),
    ] {
        let db = legacy_database().await;
        sqlx::query(definition).execute(db.pool()).await.unwrap();
        sqlx::query(insertion).execute(db.pool()).await.unwrap();
        assert_rejected_unchanged(&db).await;
        let marker: String = sqlx::query_scalar(contents)
            .fetch_one(db.pool())
            .await
            .unwrap();
        assert_eq!(marker, "Synthetic preserved marker");
        assert!(!table_exists(&db, "_sqlx_migrations").await);
        assert!(!table_exists(&db, "sync_metadata").await);
        assert!(!table_exists(&db, "correspondence").await);
        db.pool().close().await;
    }
}

#[tokio::test]
async fn a_foreign_application_marker_is_not_relabelled_as_a_local_database() {
    let db = legacy_database().await;
    sqlx::query("PRAGMA application_id = 123456")
        .execute(db.pool())
        .await
        .unwrap();
    assert_rejected_unchanged(&db).await;
    assert!(!table_exists(&db, "_sqlx_migrations").await);
    db.pool().close().await;
}

#[tokio::test]
async fn a_clean_native_prefix_upgrades_without_touching_remote_records() {
    let db = DbPool::new_in_memory().await.unwrap();
    sqlx::migrate!("./migrations")
        .run_to(20261114000000, db.pool())
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO requests (id, title, body, status)
         VALUES (7, 'Synthetic remote record', 'SYNTHETIC-PRIVATE-REMOTE', 'draft')",
    )
    .execute(db.pool())
    .await
    .unwrap();
    db.migrate_legacy_local_schema().await.unwrap();
    let remote: (i64, String, String, String) =
        sqlx::query_as("SELECT id, title, body, status FROM requests")
            .fetch_one(db.pool())
            .await
            .unwrap();
    assert_eq!(
        remote,
        (
            7,
            "Synthetic remote record".to_owned(),
            "SYNTHETIC-PRIVATE-REMOTE".to_owned(),
            "draft".to_owned()
        )
    );
    let counts: (i64, i64, i64, i64) = sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM tracked_requests),
                (SELECT COUNT(*) FROM local_request_events),
                (SELECT COUNT(*) FROM feed_events),
                (SELECT COUNT(*) FROM request_snapshots)",
    )
    .fetch_one(db.pool())
    .await
    .unwrap();
    assert_eq!(counts, (0, 0, 0, 0));
    let upgraded = snapshot(&db).await;
    db.migrate_legacy_local_schema().await.unwrap();
    assert_eq!(snapshot(&db).await, upgraded);
    db.pool().close().await;
}

#[tokio::test]
async fn sqlite_capacity_failure_cannot_leave_a_partial_migration_ledger() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = std::env::temp_dir().join(format!(
        "fyi-legacy-capacity-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir(&directory).unwrap();
    let db = DbPool::new(&format!(
        "sqlite://{}?mode=rwc",
        directory.join("legacy.db").display()
    ))
    .await
    .unwrap();
    sqlx::raw_sql(LEGACY_SQL).execute(db.pool()).await.unwrap();
    // DbPool::new permits four connections. Reserve three so the method must use
    // the remaining connection, because max_page_count is connection-local.
    let reserved_connections = [
        db.pool().acquire().await.unwrap(),
        db.pool().acquire().await.unwrap(),
        db.pool().acquire().await.unwrap(),
    ];
    // SQLite clamps this to the existing page count, preventing migration growth.
    // The file-backed fixture survives even if SQLx must discard a failed connection.
    let maximum: i64 = sqlx::query_scalar("PRAGMA max_page_count = 1")
        .fetch_one(db.pool())
        .await
        .unwrap();
    let pages: i64 = sqlx::query_scalar("PRAGMA page_count")
        .fetch_one(db.pool())
        .await
        .unwrap();
    assert_eq!(maximum, pages);
    assert_rejected_unchanged(&db).await;
    assert!(!table_exists(&db, "_sqlx_migrations").await);
    assert!(!table_exists(&db, "requests").await);
    assert!(!table_exists(&db, "local_request_events").await);
    drop(reserved_connections);
    db.pool().close().await;
    fs::remove_dir_all(directory).unwrap();
}
