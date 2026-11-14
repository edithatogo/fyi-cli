//! Physical schema and migration-prefix checks for retained local repositories.
use fyi_core::db::DbPool;

async fn database() -> DbPool {
    let db = DbPool::new_in_memory().await.unwrap();
    db.run_migrations().await.unwrap();
    db
}

#[tokio::test]
async fn repeated_initialization_recognizes_native_tracking_and_preserves_records() {
    let db = database().await;
    sqlx::query("INSERT INTO authorities (slug, name) VALUES ('alpha', 'Alpha')")
        .execute(db.pool())
        .await
        .unwrap();
    sqlx::query("INSERT INTO tracked_requests (authority_slug, title, body) VALUES ('alpha', 'Title', 'Body')")
        .execute(db.pool())
        .await
        .unwrap();
    db.initialize_rust_schema().await.unwrap();
    db.validate_local_schema().await.unwrap();
    let title: String = sqlx::query_scalar("SELECT title FROM tracked_requests WHERE id = 1")
        .fetch_one(db.pool())
        .await
        .unwrap();
    assert_eq!(title, "Title");
}

#[tokio::test]
async fn previous_native_schema_upgrades_only_through_explicit_initialization() {
    let db = database().await;
    sqlx::query("DROP TABLE feed_events").execute(db.pool())
        .await
        .unwrap();
    sqlx::query("DROP TABLE request_snapshots").execute(db.pool())
        .await
        .unwrap();
    sqlx::query("DROP TABLE local_request_events").execute(db.pool())
        .await
        .unwrap();
    sqlx::query("DROP TABLE tracked_requests").execute(db.pool())
        .await
        .unwrap();
    sqlx::query("DELETE FROM _sqlx_migrations WHERE version >= 20261114001000")
        .execute(db.pool())
        .await
        .unwrap();
    sqlx::query("INSERT INTO authorities (slug, name) VALUES ('alpha', 'Preserved')")
        .execute(db.pool())
        .await
        .unwrap();
    assert!(db.validate_local_schema().await.is_err());
    db.initialize_rust_schema().await.unwrap();
    db.validate_local_schema().await.unwrap();
    assert_eq!(
        db.list_local_authorities().await.unwrap()[0].name,
        "Preserved"
    );
}

#[tokio::test]
async fn genuine_ledger_does_not_authorize_malformed_local_request_tables() {
    for replacement in [
        "CREATE TABLE tracked_requests (id INTEGER PRIMARY KEY, authority_slug TEXT NOT NULL, title TEXT NOT NULL, body TEXT NOT NULL, tags TEXT DEFAULT '', fyi_url TEXT, fyi_request_id INTEGER, status TEXT DEFAULT 'draft', created_at TEXT DEFAULT CURRENT_TIMESTAMP, updated_at TEXT DEFAULT CURRENT_TIMESTAMP, last_seen_at TEXT, last_event_title TEXT, FOREIGN KEY(authority_slug) REFERENCES authorities(slug))",
        "CREATE TABLE tracked_requests (id INTEGER PRIMARY KEY AUTOINCREMENT, authority_slug TEXT NOT NULL, title TEXT NOT NULL, body TEXT NOT NULL, tags TEXT DEFAULT '', fyi_url TEXT, fyi_request_id INTEGER, status TEXT DEFAULT 'draft', created_at TEXT DEFAULT CURRENT_TIMESTAMP, updated_at TEXT DEFAULT CURRENT_TIMESTAMP, last_seen_at TEXT, last_event_title TEXT)",
        "CREATE TABLE tracked_requests (id INTEGER PRIMARY KEY AUTOINCREMENT, authority_slug TEXT NOT NULL, title TEXT NOT NULL, body TEXT NOT NULL)",
    ] {
        let db = database().await;
        sqlx::query("DROP TABLE local_request_events").execute(db.pool())
        .await
        .unwrap();
        sqlx::query("DROP TABLE tracked_requests").execute(db.pool())
        .await
        .unwrap();
        sqlx::query(replacement).execute(db.pool())
        .await
        .unwrap();
        let before: Vec<(String, Option<String>)> = sqlx::query_as("SELECT name, sql FROM sqlite_master ORDER BY name")
            .fetch_all(db.pool()).await.unwrap();
        assert!(db.validate_local_schema().await.is_err());
        assert!(db.initialize_rust_schema().await.is_err());
        let after: Vec<(String, Option<String>)> = sqlx::query_as("SELECT name, sql FROM sqlite_master ORDER BY name")
            .fetch_all(db.pool()).await.unwrap();
        assert_eq!(before, after);
    }
}

#[tokio::test]
async fn unexpected_tracking_table_is_not_blessed_by_an_older_valid_ledger() {
    let db = database().await;
    sqlx::query("DELETE FROM _sqlx_migrations WHERE version = 20261114001000")
        .execute(db.pool())
        .await
        .unwrap();
    assert!(db.initialize_rust_schema().await.is_err());
    let applied: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations WHERE version = 20261114001000")
            .fetch_one(db.pool())
            .await
            .unwrap();
    assert_eq!(applied, 0);
}

#[tokio::test]
async fn corrupt_or_future_ledgers_are_not_repaired_by_initialization() {
    for corruption in [
        "UPDATE _sqlx_migrations SET success = 0 WHERE version = 20261114001000",
        "UPDATE _sqlx_migrations SET checksum = X'00' WHERE version = 20261114001000",
        "UPDATE _sqlx_migrations SET version = 99999999999999 WHERE version = 20261114001000",
        "DELETE FROM _sqlx_migrations WHERE version = 20260630000000",
    ] {
        let db = database().await;
        sqlx::query(corruption).execute(db.pool())
        .await
        .unwrap();
        let before: Vec<(i64, bool, Vec<u8>)> = sqlx::query_as(
            "SELECT version, success, checksum FROM _sqlx_migrations ORDER BY version",
        )
        .fetch_all(db.pool())
        .await
        .unwrap();
        assert!(db.initialize_rust_schema().await.is_err());
        let after: Vec<(i64, bool, Vec<u8>)> = sqlx::query_as(
            "SELECT version, success, checksum FROM _sqlx_migrations ORDER BY version",
        )
        .fetch_all(db.pool())
        .await
        .unwrap();
        assert_eq!(before, after);
    }
}
