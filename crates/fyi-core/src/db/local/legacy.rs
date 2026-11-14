//! Explicit, atomic adoption of the known Python database layout.

use super::{schema, LocalError};
use crate::db::DbPool;
use sqlx::{Connection, Row, SqliteConnection};
use std::collections::HashMap;

const LEGACY_TABLES: &[&str] = &[
    "authorities",
    "tracked_requests",
    "feed_events",
    "request_snapshots",
    "run_log",
    "shared_rate_limit_state",
    "shared_rate_limit_events",
    "http_cache",
];
const NATIVE_TABLES: &[&str] = &[
    "requests",
    "correspondence",
    "sync_metadata",
    "sync_field_changes",
    "sync_outgoing_queue",
    "local_request_events",
    "_sqlx_migrations",
];
const LEGACY_INDEXES: &[(&str, &str)] = &[
    ("sqlite_autoindex_authorities_1", "authorities"),
    (
        "sqlite_autoindex_shared_rate_limit_state_1",
        "shared_rate_limit_state",
    ),
    ("sqlite_autoindex_http_cache_1", "http_cache"),
];
const NATIVE_INDEXES: &[(&str, &str)] = &[
    ("sqlite_autoindex__sqlx_migrations_1", "_sqlx_migrations"),
    ("idx_sync_metadata_status", "sync_metadata"),
    ("idx_sync_field_changes_request", "sync_field_changes"),
    ("idx_sync_outgoing_queue_status", "sync_outgoing_queue"),
    ("idx_local_request_events_request", "local_request_events"),
];

type Column = (&'static str, &'static str, i64, i64, Option<&'static str>);

const LEGACY_LAYOUTS: &[(&str, &[Column])] = &[
    (
        "run_log",
        &[
            ("id", "INTEGER", 0, 1, None),
            ("job_name", "TEXT", 1, 0, None),
            ("status", "TEXT", 1, 0, None),
            ("detail", "TEXT", 0, 0, None),
            ("ran_at", "TEXT", 0, 0, Some("CURRENT_TIMESTAMP")),
        ],
    ),
    (
        "shared_rate_limit_state",
        &[
            ("name", "TEXT", 0, 1, None),
            ("next_allowed_at", "REAL", 1, 0, Some("0")),
            ("last_acquired_at", "TEXT", 1, 0, Some("CURRENT_TIMESTAMP")),
            ("last_owner_id", "TEXT", 1, 0, Some("''")),
            ("last_sleep_seconds", "REAL", 1, 0, Some("0")),
            ("interval_seconds", "REAL", 1, 0, Some("0")),
            ("jitter_seconds", "REAL", 1, 0, Some("0")),
        ],
    ),
    (
        "shared_rate_limit_events",
        &[
            ("id", "INTEGER", 0, 1, None),
            ("name", "TEXT", 1, 0, None),
            ("kind", "TEXT", 1, 0, None),
            ("owner_id", "TEXT", 1, 0, Some("''")),
            ("status_code", "INTEGER", 0, 0, None),
            ("delay_seconds", "REAL", 1, 0, Some("0")),
            ("next_allowed_at", "REAL", 1, 0, Some("0")),
            ("observed_at", "TEXT", 1, 0, Some("CURRENT_TIMESTAMP")),
        ],
    ),
    (
        "http_cache",
        &[
            ("url", "TEXT", 0, 1, None),
            ("etag", "TEXT", 0, 0, None),
            ("last_modified", "TEXT", 0, 0, None),
            ("response_body", "TEXT", 0, 0, None),
            ("cached_at", "TEXT", 0, 0, Some("CURRENT_TIMESTAMP")),
        ],
    ),
];

const ALLOCATORS: &[(&str, &str)] = &[
    (
        "tracked_requests",
        "SELECT COALESCE(MAX(id), 0), COALESCE(MIN(id), 1) FROM tracked_requests",
    ),
    (
        "feed_events",
        "SELECT COALESCE(MAX(id), 0), COALESCE(MIN(id), 1) FROM feed_events",
    ),
    (
        "request_snapshots",
        "SELECT COALESCE(MAX(id), 0), COALESCE(MIN(id), 1) FROM request_snapshots",
    ),
    (
        "run_log",
        "SELECT COALESCE(MAX(id), 0), COALESCE(MIN(id), 1) FROM run_log",
    ),
    (
        "shared_rate_limit_events",
        "SELECT COALESCE(MAX(id), 0), COALESCE(MIN(id), 1) FROM shared_rate_limit_events",
    ),
    (
        "local_request_events",
        "SELECT COALESCE(MAX(id), 0), COALESCE(MIN(id), 1) FROM local_request_events",
    ),
    (
        "correspondence",
        "SELECT COALESCE(MAX(id), 0), COALESCE(MIN(id), 1) FROM correspondence",
    ),
    (
        "sync_field_changes",
        "SELECT COALESCE(MAX(id), 0), COALESCE(MIN(id), 1) FROM sync_field_changes",
    ),
    (
        "sync_outgoing_queue",
        "SELECT COALESCE(MAX(id), 0), COALESCE(MIN(id), 1) FROM sync_outgoing_queue",
    ),
];

impl DbPool {
    /// Adopt a complete known legacy layout or explicitly upgrade a verified Rust prefix.
    /// Existing rows, timestamps, raw payloads and allocator markers are never rewritten.
    pub async fn migrate_legacy_local_schema(&self) -> Result<(), LocalError> {
        let mut connection = self.pool().acquire().await?;
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&mut *connection)
            .await?;
        let mut transaction = connection.begin_with("BEGIN IMMEDIATE").await?;
        let result = migrate_on_connection(&mut transaction).await;
        let (result, invalidate_connection) = match result {
            Ok(()) => match transaction.commit().await {
                Ok(()) => (Ok(()), false),
                Err(error) => (Err(LocalError::from(error)), true),
            },
            Err(error) => match transaction.rollback().await {
                Ok(()) => (Err(error), false),
                Err(rollback) => (Err(LocalError::from(rollback)), true),
            },
        };
        if invalidate_connection {
            // Retain healthy connections after rollback, including the only in-memory connection.
            // Discard a connection only when its transaction state could not be established.
            let _ = connection.close().await;
        }
        result
    }
}

async fn migrate_on_connection(connection: &mut SqliteConnection) -> Result<(), LocalError> {
    validate_version_markers(connection).await?;
    let native = schema::table_exists(connection, "_sqlx_migrations").await?;
    validate_objects(connection, native).await?;
    if native {
        schema::validate_before_initialization(connection).await?;
    }
    validate_legacy_layouts(connection, !native).await?;
    validate_domain_rows(connection).await?;
    validate_allocators(connection).await?;
    sqlx::migrate!("./migrations")
        .run(&mut *connection)
        .await
        .map_err(|_| LocalError::Schema)?;
    schema::validate_current(connection).await?;
    validate_domain_rows(connection).await?;
    validate_allocators(connection).await
}

async fn validate_version_markers(connection: &mut SqliteConnection) -> Result<(), LocalError> {
    let user_version: i64 = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(&mut *connection)
        .await?;
    let application_id: i64 = sqlx::query_scalar("PRAGMA application_id")
        .fetch_one(&mut *connection)
        .await?;
    if user_version != 0 || application_id != 0 {
        return Err(LocalError::Schema);
    }
    Ok(())
}

async fn validate_objects(
    connection: &mut SqliteConnection,
    native: bool,
) -> Result<(), LocalError> {
    let objects: Vec<(String, String, String, Option<String>)> =
        sqlx::query_as("SELECT type, name, tbl_name, sql FROM sqlite_master ORDER BY type, name")
            .fetch_all(connection)
            .await?;
    for (kind, name, table, sql) in objects {
        let recognized = match kind.as_str() {
            "table" => {
                name == "sqlite_sequence"
                    || LEGACY_TABLES.contains(&name.as_str())
                    || (native && NATIVE_TABLES.contains(&name.as_str()))
            }
            "index" => {
                let identity = (name.as_str(), table.as_str());
                (LEGACY_INDEXES.contains(&identity)
                    || (native && NATIVE_INDEXES.contains(&identity)))
                    && (name.starts_with("sqlite_autoindex_") == sql.is_none())
            }
            _ => false,
        };
        if !recognized {
            return Err(LocalError::Schema);
        }
    }
    Ok(())
}

async fn validate_legacy_layouts(
    connection: &mut SqliteConnection,
    required: bool,
) -> Result<(), LocalError> {
    schema::validate_authorities(connection, required).await?;
    if schema::table_exists(connection, "tracked_requests").await? {
        schema::validate_tracked_requests(connection).await?;
    } else if required {
        return Err(LocalError::Schema);
    }
    schema::validate_provenance(connection, required).await?;
    for &(table, columns) in LEGACY_LAYOUTS {
        schema::validate_columns(connection, table, columns, required).await?;
    }
    for &table in LEGACY_TABLES {
        if !schema::table_exists(connection, table).await? {
            continue;
        }
        validate_no_schema_extensions(connection, table).await?;
        if table != "tracked_requests" {
            let foreign_keys: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM pragma_foreign_key_list(?)")
                    .bind(table)
                    .fetch_one(&mut *connection)
                    .await?;
            if foreign_keys != 0 {
                return Err(LocalError::Schema);
            }
        }
    }
    Ok(())
}

async fn validate_no_schema_extensions(
    connection: &mut SqliteConnection,
    table: &str,
) -> Result<(), LocalError> {
    let hidden: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM pragma_table_xinfo(?) WHERE hidden != 0")
            .bind(table)
            .fetch_one(&mut *connection)
            .await?;
    let (without_rowid, strict): (i64, i64) = sqlx::query_as(
        "SELECT wr, strict FROM pragma_table_list WHERE schema = 'main' AND name = ?",
    )
    .bind(table)
    .fetch_one(&mut *connection)
    .await?;
    let definition: String =
        sqlx::query_scalar("SELECT sql FROM sqlite_master WHERE type = 'table' AND name = ?")
            .bind(table)
            .fetch_one(&mut *connection)
            .await?;
    let unsupported_constraint = definition
        .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
        .any(|token| {
            ["CHECK", "UNIQUE", "COLLATE", "DEFERRABLE", "GENERATED"]
                .iter()
                .any(|keyword| token.eq_ignore_ascii_case(keyword))
        });
    if hidden != 0 || without_rowid != 0 || strict != 0 || unsupported_constraint {
        return Err(LocalError::Schema);
    }
    Ok(())
}

async fn validate_domain_rows(connection: &mut SqliteConnection) -> Result<(), LocalError> {
    if schema::table_exists(connection, "authorities").await? {
        reject_matching_rows(
            connection,
            "SELECT EXISTS(SELECT 1 FROM authorities
             WHERE typeof(slug) != 'text' OR typeof(name) != 'text'
             OR typeof(url) NOT IN ('null', 'text'))",
        )
        .await?;
    }
    if schema::table_exists(connection, "tracked_requests").await? {
        reject_matching_rows(
            connection,
            "SELECT EXISTS(SELECT 1 FROM tracked_requests
             WHERE id <= 0 OR typeof(authority_slug) != 'text'
             OR typeof(title) != 'text' OR typeof(body) != 'text'
             OR typeof(tags) NOT IN ('null', 'text')
             OR typeof(fyi_url) NOT IN ('null', 'text')
             OR typeof(status) NOT IN ('null', 'text')
             OR typeof(created_at) NOT IN ('null', 'text')
             OR typeof(updated_at) NOT IN ('null', 'text')
             OR typeof(last_seen_at) NOT IN ('null', 'text')
             OR typeof(last_event_title) NOT IN ('null', 'text')
             OR (fyi_request_id IS NOT NULL
                 AND (typeof(fyi_request_id) != 'integer' OR fyi_request_id <= 0)))",
        )
        .await?;
    }
    if schema::table_exists(connection, "feed_events").await? {
        reject_matching_rows(
            connection,
            "SELECT EXISTS(SELECT 1 FROM feed_events
             WHERE id <= 0 OR typeof(feed_url) != 'text'
             OR typeof(event_id) NOT IN ('null', 'text')
             OR typeof(title) NOT IN ('null', 'text')
             OR typeof(link) NOT IN ('null', 'text')
             OR typeof(published) NOT IN ('null', 'text')
             OR typeof(summary) NOT IN ('null', 'text')
             OR typeof(raw_json) NOT IN ('null', 'text')
             OR typeof(seen_at) NOT IN ('null', 'text')
             OR (request_id_guess IS NOT NULL
                 AND (typeof(request_id_guess) != 'integer' OR request_id_guess <= 0))
             OR (tracked_request_id IS NOT NULL
                 AND (typeof(tracked_request_id) != 'integer' OR tracked_request_id <= 0)))",
        )
        .await?;
    }
    if schema::table_exists(connection, "request_snapshots").await? {
        reject_matching_rows(
            connection,
            "SELECT EXISTS(SELECT 1 FROM request_snapshots
             WHERE id <= 0 OR typeof(fyi_request_id) != 'integer' OR fyi_request_id <= 0
             OR typeof(source_url) != 'text' OR typeof(raw_json) != 'text'
             OR typeof(fetched_at) NOT IN ('null', 'text'))",
        )
        .await?;
    }
    if schema::table_exists(connection, "feed_events").await? {
        schema::validate_local_references(connection).await?;
    } else if sqlx::query("PRAGMA foreign_key_check")
        .fetch_optional(&mut *connection)
        .await?
        .is_some()
    {
        return Err(LocalError::Schema);
    }
    Ok(())
}

async fn reject_matching_rows(
    connection: &mut SqliteConnection,
    query: &'static str,
) -> Result<(), LocalError> {
    let invalid: bool = sqlx::query_scalar(query).fetch_one(connection).await?;
    if invalid {
        return Err(LocalError::Schema);
    }
    Ok(())
}

async fn validate_allocators(connection: &mut SqliteConnection) -> Result<(), LocalError> {
    schema::validate_columns(
        connection,
        "sqlite_sequence",
        &[("name", "", 0, 0, None), ("seq", "", 0, 0, None)],
        true,
    )
    .await?;
    let rows = sqlx::query("SELECT name, seq, typeof(seq) AS storage_type FROM sqlite_sequence")
        .fetch_all(&mut *connection)
        .await?;
    let mut markers = HashMap::new();
    for row in rows {
        let name: String = row.try_get("name")?;
        let storage_type: String = row.try_get("storage_type")?;
        if storage_type != "integer" || !ALLOCATORS.iter().any(|entry| entry.0 == name) {
            return Err(LocalError::Schema);
        }
        let sequence: i64 = row.try_get("seq")?;
        if sequence < 0 || markers.insert(name, sequence).is_some() {
            return Err(LocalError::Schema);
        }
    }
    for &(table, query) in ALLOCATORS {
        if !schema::table_exists(connection, table).await? {
            if markers.contains_key(table) {
                return Err(LocalError::Schema);
            }
            continue;
        }
        schema::validate_autoincrement(connection, table).await?;
        let (maximum, minimum): (i64, i64) =
            sqlx::query_as(query).fetch_one(&mut *connection).await?;
        if minimum <= 0
            || markers
                .get(table)
                .is_some_and(|sequence| *sequence < maximum)
            || (maximum > 0 && !markers.contains_key(table))
        {
            return Err(LocalError::Schema);
        }
    }
    Ok(())
}
