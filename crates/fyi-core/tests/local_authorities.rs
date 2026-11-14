//! Offline contracts for local authorities, independent of remote acquisition.

use fyi_core::db::local::{parse_authorities_csv, LocalAuthority};
use fyi_core::db::DbPool;
use serde_json::Value;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::ConnectOptions;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const PYTHON_FIXTURES: &str =
    include_str!("../../../conformance/local-management/authorities-v1-python.json");

fn authority(slug: &str, name: &str, url: Option<&str>) -> LocalAuthority {
    LocalAuthority {
        slug: slug.to_owned(),
        name: name.to_owned(),
        url: url.map(str::to_owned),
    }
}

async fn migrated_db() -> DbPool {
    let db = DbPool::new_in_memory().await.unwrap();
    db.run_migrations().await.unwrap();
    db
}

async fn migrated_file(path: &Path) -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(path)
                .create_if_missing(true)
                .foreign_keys(true)
                .disable_statement_logging(),
        )
        .await
        .unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    pool
}

fn temporary_directory(label: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = std::env::temp_dir().join(format!(
        "fyi-local-authorities-{label}-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir(&directory).unwrap();
    directory
}

fn assert_fixture_rows(actual: &[LocalAuthority], expected: &Value, fixture: &str) {
    let expected = expected.as_array().unwrap();
    assert_eq!(actual.len(), expected.len(), "{fixture}");
    for (actual, expected) in actual.iter().zip(expected) {
        assert_eq!(actual.slug, expected["slug"].as_str().unwrap(), "{fixture}");
        assert_eq!(actual.name, expected["name"].as_str().unwrap(), "{fixture}");
        assert_eq!(actual.url.as_deref(), expected["url"].as_str(), "{fixture}");
    }
}

#[tokio::test]
async fn python_valid_fixtures_match_counts_and_persisted_projection() {
    let corpus: Value = serde_json::from_str(PYTHON_FIXTURES).unwrap();
    assert_eq!(corpus["schema_version"], 1);
    for case in corpus["valid_cases"].as_array().unwrap() {
        let fixture = case["id"].as_str().unwrap();
        let parsed = parse_authorities_csv(case["csv"].as_str().unwrap().as_bytes()).unwrap();
        let expected_count = case["imported_count"].as_u64().unwrap() as usize;
        assert_eq!(parsed.len(), expected_count, "{fixture}");
        let db = migrated_db().await;
        let imported = db.import_local_authorities(&parsed).await.unwrap();
        assert_eq!(imported, expected_count, "{fixture}");
        assert_fixture_rows(
            &db.list_local_authorities().await.unwrap(),
            &case["rows"],
            fixture,
        );
        let repeated = db.import_local_authorities(&parsed).await.unwrap();
        assert_eq!(repeated, expected_count, "{fixture}");
        assert_fixture_rows(
            &db.list_local_authorities().await.unwrap(),
            &case["rows"],
            fixture,
        );
    }
}

#[test]
fn csv_requires_slug_and_name_semantic_headers() {
    for input in [
        "",
        "slug\nalpha\n",
        "name\nAlpha Authority\n",
        "slug,url\nalpha,https://catalog.example.invalid/body/alpha\n",
        "name,url\nAlpha Authority,\n",
        "unknown,other\nalpha,Alpha Authority\n",
    ] {
        assert!(parse_authorities_csv(input.as_bytes()).is_err());
    }
}

#[test]
fn csv_rejects_duplicate_headers_including_unused_columns() {
    for input in [
        "slug,slug,name\nalpha,beta,Alpha Authority\n",
        "slug,name,name\nalpha,First,Second\n",
        "slug,name,extra,extra\nalpha,Alpha Authority,x,y\n",
    ] {
        assert!(parse_authorities_csv(input.as_bytes()).is_err());
    }
}

#[test]
fn csv_rejects_short_extra_and_unclosed_quoted_records() {
    for input in [
        "slug,name,url\nalpha,Alpha Authority\n",
        "slug,name\nalpha,Alpha Authority,unexpected\n",
        "slug,name\nalpha,\"unclosed authority\n",
        "slug,name\nalpha,Alpha Authority\nbeta\n",
    ] {
        assert!(parse_authorities_csv(input.as_bytes()).is_err());
    }
}

#[test]
fn csv_rejects_invalid_utf8_without_lossy_replacement() {
    let input = b"slug,name\nalpha,Authority \xff\n";
    assert!(parse_authorities_csv(input).is_err());
}

#[test]
fn blank_required_values_reject_the_complete_legacy_input() {
    let corpus: Value = serde_json::from_str(PYTHON_FIXTURES).unwrap();
    for case in corpus["legacy_differences"].as_array().unwrap() {
        let fixture = case["id"].as_str().unwrap();
        if fixture.starts_with("blank_required_") {
            assert!(
                parse_authorities_csv(case["csv"].as_str().unwrap().as_bytes()).is_err(),
                "{fixture}"
            );
        }
    }
}

#[test]
fn csv_rejects_control_codes_in_keys_urls_and_display_names() {
    for input in [
        "slug,name\n\"alpha\nbeta\",Alpha Authority\n",
        "slug,name\n\"alpha\tbeta\",Alpha Authority\n",
        "slug,name\nalpha,Authority\0Name\n",
        "slug,name\nalpha,Authority\u{1b}[31m\n",
        "slug,name,url\nalpha,Alpha Authority,\"https://catalog.example.invalid/\nbody\"\n",
        "slug,name,url\nalpha,Alpha Authority,\"https://catalog.example.invalid/\tbody\"\n",
    ] {
        assert!(parse_authorities_csv(input.as_bytes()).is_err());
    }
}

#[test]
fn csv_preserves_quoted_name_newlines_and_tabs() {
    let parsed = parse_authorities_csv(b"slug,name\nalpha,\"Alpha\tRecords\nOffice\"\n").unwrap();
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].name, "Alpha\tRecords\nOffice");
}

#[test]
fn csv_rejects_inputs_larger_than_sixteen_mebibytes() {
    let mut input = b"slug,name\nalpha,".to_vec();
    input.resize(16 * 1024 * 1024 + 1, b'x');
    assert!(parse_authorities_csv(&input).is_err());
}

#[tokio::test]
async fn late_sql_failure_rolls_back_earlier_authority_updates() {
    let db = migrated_db().await;
    db.import_local_authorities(&[authority("alpha", "Original Authority", None)])
        .await
        .unwrap();
    sqlx::query(
        "CREATE TRIGGER fail_authority_fixture BEFORE INSERT ON authorities
         WHEN NEW.slug = 'fail'
         BEGIN SELECT RAISE(ABORT, 'synthetic import failure'); END",
    )
    .execute(db.pool())
    .await
    .unwrap();

    let result = db
        .import_local_authorities(&[
            authority("alpha", "Changed Authority", None),
            authority("fail", "Failing Authority", None),
        ])
        .await;
    assert!(result.is_err());
    let rows = db.list_local_authorities().await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].slug, "alpha");
    assert_eq!(rows[0].name, "Original Authority");
}

#[tokio::test]
async fn upsert_preserves_referencing_rows_and_clears_optional_url() {
    let db = migrated_db().await;
    db.import_local_authorities(&[authority(
        "alpha",
        "Original Authority",
        Some("https://catalog.example.invalid/body/alpha"),
    )])
    .await
    .unwrap();
    sqlx::query(
        "CREATE TABLE authority_reference_fixture (
            id INTEGER PRIMARY KEY,
            authority_slug TEXT NOT NULL REFERENCES authorities(slug) ON DELETE CASCADE
         )",
    )
    .execute(db.pool())
    .await
    .unwrap();
    sqlx::query("INSERT INTO authority_reference_fixture VALUES (1, 'alpha')")
        .execute(db.pool())
        .await
        .unwrap();

    assert_eq!(
        db.import_local_authorities(&[authority("alpha", "Updated Authority", None)])
            .await
            .unwrap(),
        1
    );
    let reference: String =
        sqlx::query_scalar("SELECT authority_slug FROM authority_reference_fixture WHERE id = 1")
            .fetch_one(db.pool())
            .await
            .unwrap();
    assert_eq!(reference, "alpha");
    let rows = db.list_local_authorities().await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].name, "Updated Authority");
    assert_eq!(rows[0].url, None);
}

#[tokio::test]
async fn repository_revalidates_direct_inputs_without_partial_updates() {
    let db = migrated_db().await;
    db.import_local_authorities(&[authority("alpha", "Original Authority", None)])
        .await
        .unwrap();
    for invalid in [
        authority("", "Missing slug", None),
        authority("beta", "   ", None),
        authority("bad\0slug", "Invalid slug", None),
        authority("beta", "Invalid\u{1b}Name", None),
        authority(
            "beta",
            "Invalid URL",
            Some("https://example.invalid/\0body"),
        ),
    ] {
        let result = db
            .import_local_authorities(&[authority("alpha", "Changed Authority", None), invalid])
            .await;
        assert!(result.is_err());
        let rows = db.list_local_authorities().await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "Original Authority");
    }
}

#[tokio::test]
async fn listing_uses_slug_to_break_equal_name_ties() {
    let corpus: Value = serde_json::from_str(PYTHON_FIXTURES).unwrap();
    let case = corpus["legacy_differences"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["id"] == "equal_names_follow_legacy_insertion_order")
        .unwrap();
    let parsed = parse_authorities_csv(case["csv"].as_str().unwrap().as_bytes()).unwrap();
    let db = migrated_db().await;
    db.import_local_authorities(&parsed).await.unwrap();
    assert_fixture_rows(
        &db.list_local_authorities().await.unwrap(),
        &case["rows"],
        "equal-name ordering",
    );
}

#[tokio::test]
async fn authority_operations_leave_remote_requests_and_sync_tables_empty() {
    let db = migrated_db().await;
    db.import_local_authorities(&[authority("alpha", "Alpha Authority", None)])
        .await
        .unwrap();
    assert_eq!(db.list_local_authorities().await.unwrap().len(), 1);
    for table in [
        "requests",
        "correspondence",
        "sync_metadata",
        "sync_field_changes",
        "sync_outgoing_queue",
    ] {
        let count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
            .fetch_one(db.pool())
            .await
            .unwrap();
        assert_eq!(count, 0, "local authority operation changed {table}");
    }
}

#[tokio::test]
async fn existing_read_only_pool_lists_but_cannot_mutate() {
    let directory = temporary_directory("read-only");
    let path = directory.join("authorities.db");
    let fixture = migrated_file(&path).await;
    sqlx::query(
        "INSERT INTO authorities (slug, name, url) VALUES ('alpha', 'Alpha Authority', NULL)",
    )
    .execute(&fixture)
    .await
    .unwrap();
    fixture.close().await;

    let before = fs::read(&path).unwrap();
    let reader = DbPool::open_local_existing(&path, true).await.unwrap();
    let rows = reader.list_local_authorities().await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].slug, "alpha");
    let foreign_keys: i64 = sqlx::query_scalar("PRAGMA foreign_keys")
        .fetch_one(reader.pool())
        .await
        .unwrap();
    assert_eq!(foreign_keys, 1);
    assert!(reader
        .import_local_authorities(&[authority("beta", "Beta Authority", None)])
        .await
        .is_err());
    assert!(sqlx::query("DELETE FROM authorities")
        .execute(reader.pool())
        .await
        .is_err());
    reader.pool().close().await;
    assert_eq!(fs::read(&path).unwrap(), before);
    fs::remove_dir_all(directory).unwrap();
}

#[tokio::test]
async fn opening_missing_existing_database_never_creates_files_or_parents() {
    let directory = temporary_directory("missing");
    for read_only in [true, false] {
        let path = directory.join("missing.db");
        assert!(DbPool::open_local_existing(&path, read_only).await.is_err());
        assert!(!path.exists());
        let nested = directory.join("absent").join("missing.db");
        assert!(DbPool::open_local_existing(&nested, read_only)
            .await
            .is_err());
        assert!(!directory.join("absent").exists());
    }
    assert_eq!(fs::read_dir(&directory).unwrap().count(), 0);
    fs::remove_dir_all(directory).unwrap();
}

#[tokio::test]
async fn existing_write_pool_enforces_foreign_keys_and_preserves_readback() {
    let directory = temporary_directory("write-existing");
    let path = directory.join("authorities with spaces.db");
    let fixture = migrated_file(&path).await;
    fixture.close().await;

    let writer = DbPool::open_local_existing(&path, false).await.unwrap();
    writer
        .import_local_authorities(&[authority("alpha", "Alpha Authority", None)])
        .await
        .unwrap();
    let mut first = writer.pool().acquire().await.unwrap();
    let mut second = writer.pool().acquire().await.unwrap();
    for connection in [&mut first, &mut second] {
        let foreign_keys: i64 = sqlx::query_scalar("PRAGMA foreign_keys")
            .fetch_one(&mut **connection)
            .await
            .unwrap();
        assert_eq!(foreign_keys, 1);
    }
    drop(first);
    drop(second);
    writer.pool().close().await;
    let reader = DbPool::open_local_existing(&path, true).await.unwrap();
    assert_eq!(
        reader.list_local_authorities().await.unwrap()[0].slug,
        "alpha"
    );
    reader.pool().close().await;
    fs::remove_dir_all(directory).unwrap();
}

#[tokio::test]
async fn opening_existing_database_rejects_unverified_migration_ledgers_without_repair() {
    let directory = temporary_directory("schema-ledger");
    let corruptions = [
        "UPDATE _sqlx_migrations SET checksum = x'00'
         WHERE version = (SELECT MIN(version) FROM _sqlx_migrations)",
        "INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time)
         VALUES (99999999999999, 'unknown future fixture', 1, x'00', 0)",
        "UPDATE _sqlx_migrations SET success = 0
         WHERE version = (SELECT MAX(version) FROM _sqlx_migrations)",
        "DELETE FROM _sqlx_migrations
         WHERE version = (SELECT MAX(version) FROM _sqlx_migrations)",
    ];
    for (index, corruption) in corruptions.iter().enumerate() {
        let path = directory.join(format!("invalid-{index}.db"));
        let fixture = migrated_file(&path).await;
        sqlx::query(corruption).execute(&fixture).await.unwrap();
        fixture.close().await;
        let before = fs::read(&path).unwrap();
        for read_only in [true, false] {
            assert!(DbPool::open_local_existing(&path, read_only).await.is_err());
            assert_eq!(fs::read(&path).unwrap(), before);
        }
    }
    fs::remove_dir_all(directory).unwrap();
}

#[tokio::test]
async fn initialization_rejects_malformed_authority_tables_before_recording_migrations() {
    let malformed_schemas = [
        "CREATE TABLE authorities (slug TEXT, name TEXT NOT NULL, url TEXT)",
        "CREATE TABLE authorities (slug TEXT PRIMARY KEY, name TEXT, url TEXT)",
        "CREATE TABLE authorities (slug TEXT PRIMARY KEY, name TEXT NOT NULL)",
        "CREATE TABLE authorities (
            slug TEXT PRIMARY KEY, name TEXT NOT NULL, url TEXT, unexpected TEXT
         )",
        "CREATE TABLE authorities (slug BLOB PRIMARY KEY, name TEXT NOT NULL, url TEXT)",
        "CREATE TABLE authorities (slug TEXT PRIMARY KEY, name INTEGER NOT NULL, url TEXT)",
        "CREATE TABLE authorities (
            slug TEXT NOT NULL PRIMARY KEY, name TEXT NOT NULL, url TEXT
         )",
    ];
    for schema in malformed_schemas {
        let db = DbPool::new_in_memory().await.unwrap();
        sqlx::query(schema).execute(db.pool()).await.unwrap();
        sqlx::query(
            "INSERT INTO authorities (slug, name) VALUES ('preserved', 'Preserved Authority')",
        )
        .execute(db.pool())
        .await
        .unwrap();
        let before: Vec<(String, String, Option<String>)> =
            sqlx::query_as("SELECT name, type, sql FROM sqlite_master ORDER BY name")
                .fetch_all(db.pool())
                .await
                .unwrap();

        assert!(db.initialize_rust_schema().await.is_err(), "{schema}");

        let after: Vec<(String, String, Option<String>)> =
            sqlx::query_as("SELECT name, type, sql FROM sqlite_master ORDER BY name")
                .fetch_all(db.pool())
                .await
                .unwrap();
        assert_eq!(after, before, "initialization changed an unsupported schema");
        let row: (String, String) = sqlx::query_as("SELECT slug, name FROM authorities")
            .fetch_one(db.pool())
            .await
            .unwrap();
        assert_eq!(row, ("preserved".to_owned(), "Preserved Authority".to_owned()));
        let ledger_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE name = '_sqlx_migrations'",
        )
        .fetch_one(db.pool())
        .await
        .unwrap();
        assert_eq!(ledger_count, 0);
    }
}

#[tokio::test]
async fn genuine_migration_ledger_does_not_bless_missing_or_replaced_authority_tables() {
    for replacement in [
        None,
        Some("CREATE TABLE authorities (slug TEXT, name TEXT NOT NULL, url TEXT)"),
        Some("CREATE TABLE authorities (slug TEXT PRIMARY KEY, name TEXT, url TEXT)"),
    ] {
        let db = migrated_db().await;
        let ledger_before: Vec<(i64, bool, Vec<u8>)> = sqlx::query_as(
            "SELECT version, success, checksum FROM _sqlx_migrations ORDER BY version",
        )
        .fetch_all(db.pool())
        .await
        .unwrap();
        sqlx::query("DROP TABLE authorities")
            .execute(db.pool())
            .await
            .unwrap();
        if let Some(schema) = replacement {
            sqlx::query(schema).execute(db.pool()).await.unwrap();
        }
        let schema_before: Vec<(String, String, Option<String>)> =
            sqlx::query_as("SELECT name, type, sql FROM sqlite_master ORDER BY name")
                .fetch_all(db.pool())
                .await
                .unwrap();

        assert!(db.validate_local_schema().await.is_err());

        let ledger_after: Vec<(i64, bool, Vec<u8>)> = sqlx::query_as(
            "SELECT version, success, checksum FROM _sqlx_migrations ORDER BY version",
        )
        .fetch_all(db.pool())
        .await
        .unwrap();
        assert_eq!(ledger_after, ledger_before);
        let schema_after: Vec<(String, String, Option<String>)> =
            sqlx::query_as("SELECT name, type, sql FROM sqlite_master ORDER BY name")
                .fetch_all(db.pool())
                .await
                .unwrap();
        assert_eq!(schema_after, schema_before);
    }
}
