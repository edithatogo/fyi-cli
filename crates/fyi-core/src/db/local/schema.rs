//! Validate local layouts on the same connection as an explicit migration.
use super::LocalError;
use sqlx::{Row, SqliteConnection};

pub(crate) const AUTHORITIES_VERSION: i64 = 20261114000000;
pub(crate) const REQUESTS_VERSION: i64 = 20261114001000;

pub(crate) async fn table_exists(
    connection: &mut SqliteConnection,
    table: &str,
) -> Result<bool, LocalError> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?)",
    )
    .bind(table)
    .fetch_one(connection)
    .await?)
}

/// Return the last known applied migration; reject gaps, failures and drift.
pub(crate) async fn validate_ledger(
    connection: &mut SqliteConnection,
    require_current: bool,
) -> Result<i64, LocalError> {
    let applied: Vec<(i64, bool, Vec<u8>)> =
        sqlx::query_as("SELECT version, success, checksum FROM _sqlx_migrations ORDER BY version")
            .fetch_all(connection)
            .await
            .map_err(|_| LocalError::Schema)?;
    let migrations = sqlx::migrate!("./migrations");
    let known_count = migrations.iter().count();
    if applied.is_empty()
        || applied.len() > known_count
        || (require_current && applied.len() != known_count)
        || applied
            .iter()
            .zip(migrations.iter())
            .any(|(applied, known)| {
                applied.0 != known.version
                    || !applied.1
                    || applied.2.as_slice() != known.checksum.as_ref()
            })
    {
        return Err(LocalError::Schema);
    }
    Ok(applied.last().ok_or(LocalError::Schema)?.0)
}

type Column<'a> = (&'a str, &'a str, i64, i64, Option<&'a str>);

pub(crate) async fn validate_columns(
    connection: &mut SqliteConnection,
    table: &str,
    columns: &[Column<'_>],
    required: bool,
) -> Result<(), LocalError> {
    let rows = sqlx::query(
        "SELECT name, type, \"notnull\", pk, dflt_value FROM pragma_table_info(?) ORDER BY cid",
    )
    .bind(table)
    .fetch_all(connection)
    .await?;
    if rows.is_empty() && !required {
        return Ok(());
    }
    if rows.len() != columns.len() {
        return Err(LocalError::Schema);
    }
    for (row, &(name, data_type, not_null, primary_key, default)) in rows.iter().zip(columns) {
        let stored_default: Option<String> = row.try_get("dflt_value")?;
        let default_matches = match (stored_default.as_deref(), default) {
            (Some(stored), Some(expected)) => stored.eq_ignore_ascii_case(expected),
            (None, None) => true,
            _ => false,
        };
        if row.try_get::<String, _>("name")? != name
            || !row
                .try_get::<String, _>("type")?
                .eq_ignore_ascii_case(data_type)
            || row.try_get::<i64, _>("notnull")? != not_null
            || row.try_get::<i64, _>("pk")? != primary_key
            || !default_matches
        {
            return Err(LocalError::Schema);
        }
    }
    Ok(())
}

pub(crate) async fn validate_authorities(
    connection: &mut SqliteConnection,
    required: bool,
) -> Result<(), LocalError> {
    validate_columns(
        connection,
        "authorities",
        &[
            ("slug", "TEXT", 0, 1, None),
            ("name", "TEXT", 1, 0, None),
            ("url", "TEXT", 0, 0, None),
        ],
        required,
    )
    .await
}

pub(crate) async fn validate_foreign_key(
    connection: &mut SqliteConnection,
    table: &str,
    expected: (&str, &str, &str, &str),
) -> Result<(), LocalError> {
    let keys: Vec<(String, String, String, String, String)> = sqlx::query_as(
        "SELECT \"from\", \"table\", \"to\", on_delete, on_update FROM pragma_foreign_key_list(?)",
    )
    .bind(table)
    .fetch_all(connection)
    .await?;
    if keys.len() != 1
        || keys[0].0 != expected.0
        || keys[0].1 != expected.1
        || keys[0].2 != expected.2
        || keys[0].3 != expected.3
        || keys[0].4 != "NO ACTION"
    {
        return Err(LocalError::Schema);
    }
    Ok(())
}

pub(crate) async fn validate_autoincrement(
    connection: &mut SqliteConnection,
    table: &str,
) -> Result<(), LocalError> {
    let sql: String =
        sqlx::query_scalar("SELECT sql FROM sqlite_master WHERE type = 'table' AND name = ?")
            .bind(table)
            .fetch_one(connection)
            .await?;
    if !sql.to_ascii_uppercase().contains("AUTOINCREMENT") {
        return Err(LocalError::Schema);
    }
    Ok(())
}

pub(crate) async fn validate_tracked_requests(
    connection: &mut SqliteConnection,
) -> Result<(), LocalError> {
    validate_columns(
        connection,
        "tracked_requests",
        &[
            ("id", "INTEGER", 0, 1, None),
            ("authority_slug", "TEXT", 1, 0, None),
            ("title", "TEXT", 1, 0, None),
            ("body", "TEXT", 1, 0, None),
            ("tags", "TEXT", 0, 0, Some("''")),
            ("fyi_url", "TEXT", 0, 0, None),
            ("fyi_request_id", "INTEGER", 0, 0, None),
            ("status", "TEXT", 0, 0, Some("'draft'")),
            ("created_at", "TEXT", 0, 0, Some("CURRENT_TIMESTAMP")),
            ("updated_at", "TEXT", 0, 0, Some("CURRENT_TIMESTAMP")),
            ("last_seen_at", "TEXT", 0, 0, None),
            ("last_event_title", "TEXT", 0, 0, None),
        ],
        true,
    )
    .await?;
    validate_foreign_key(
        connection,
        "tracked_requests",
        ("authority_slug", "authorities", "slug", "NO ACTION"),
    )
    .await?;
    validate_autoincrement(connection, "tracked_requests").await
}

pub(crate) async fn validate_request_events(
    connection: &mut SqliteConnection,
) -> Result<(), LocalError> {
    validate_columns(
        connection,
        "local_request_events",
        &[
            ("id", "INTEGER", 0, 1, None),
            ("tracked_request_id", "INTEGER", 1, 0, None),
            ("kind", "TEXT", 1, 0, None),
            ("old_status", "TEXT", 0, 0, None),
            ("new_status", "TEXT", 1, 0, None),
            ("occurred_at", "TEXT", 1, 0, None),
        ],
        true,
    )
    .await?;
    validate_foreign_key(
        connection,
        "local_request_events",
        ("tracked_request_id", "tracked_requests", "id", "CASCADE"),
    )
    .await?;
    validate_autoincrement(connection, "local_request_events").await
}

pub(crate) async fn validate_current(connection: &mut SqliteConnection) -> Result<(), LocalError> {
    validate_ledger(connection, true).await?;
    validate_authorities(connection, true).await?;
    validate_tracked_requests(connection).await?;
    validate_request_events(connection).await
}

pub(crate) async fn validate_before_initialization(
    connection: &mut SqliteConnection,
) -> Result<(), LocalError> {
    let version = if table_exists(connection, "_sqlx_migrations").await? {
        validate_ledger(connection, false).await?
    } else {
        0
    };
    validate_authorities(connection, version >= AUTHORITIES_VERSION).await?;
    if version >= REQUESTS_VERSION {
        validate_tracked_requests(connection).await?;
        validate_request_events(connection).await?;
    } else if table_exists(connection, "tracked_requests").await?
        || table_exists(connection, "local_request_events").await?
    {
        return Err(LocalError::Schema);
    }
    Ok(())
}
