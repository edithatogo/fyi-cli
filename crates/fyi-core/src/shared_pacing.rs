//! SQLite pacing shared with the retained Python source adapters.

use anyhow::{ensure, Context, Result};
use chrono::{DateTime, SecondsFormat, Utc};
use serde::Serialize;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    Row, SqlitePool,
};
use std::path::Path;

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS shared_rate_limit_state (
    name TEXT PRIMARY KEY,
    next_allowed_at REAL NOT NULL DEFAULT 0,
    last_acquired_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_owner_id TEXT NOT NULL DEFAULT '',
    last_sleep_seconds REAL NOT NULL DEFAULT 0,
    interval_seconds REAL NOT NULL DEFAULT 0,
    jitter_seconds REAL NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS shared_rate_limit_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    kind TEXT NOT NULL,
    owner_id TEXT NOT NULL DEFAULT '',
    status_code INTEGER,
    delay_seconds REAL NOT NULL DEFAULT 0,
    next_allowed_at REAL NOT NULL DEFAULT 0,
    observed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);";

/// Inputs for one reservation. A status code records a backoff reservation.
/// Explicit time and jitter inputs allow deterministic replay of scheduling.
pub struct ReservationInput<'a> {
    pub name: &'a str,
    pub owner_id: &'a str,
    pub now: f64,
    pub interval_seconds: f64,
    pub jitter_seconds: f64,
    pub jitter_fraction: f64,
    pub status_code: Option<u16>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Reservation {
    pub name: String,
    pub owner_id: String,
    pub sleep_seconds: f64,
    pub next_allowed_at: f64,
    pub previous_next_allowed_at: f64,
    pub interval_seconds: f64,
    pub jitter_seconds: f64,
    pub kind: String,
    pub status_code: Option<u16>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RecentEvent {
    pub kind: String,
    pub owner_id: String,
    pub status_code: Option<i64>,
    pub delay_seconds: f64,
    pub next_allowed_at: f64,
    pub observed_at: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SharedRateLimitStatus {
    pub name: String,
    pub next_allowed_at: f64,
    pub last_acquired_at: String,
    pub last_owner_id: String,
    pub last_sleep_seconds: f64,
    pub interval_seconds: f64,
    pub jitter_seconds: f64,
    pub recent_events: Vec<RecentEvent>,
}

fn valid_identifier(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 256 && !value.chars().any(char::is_control)
}

fn nonnegative_finite(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}

/// Atomically reserve a slot and its event in the Python-compatible tables.
/// SQLite's immediate transaction serializes independent processes and pools.
/// The caller sleeps for the returned delay before sending its request.
pub async fn reserve_slot(pool: &SqlitePool, input: &ReservationInput<'_>) -> Result<Reservation> {
    ensure!(valid_identifier(input.name), "invalid shared pacing name");
    ensure!(
        valid_identifier(input.owner_id),
        "invalid shared pacing owner"
    );
    ensure!(
        [input.now, input.interval_seconds, input.jitter_seconds]
            .into_iter()
            .all(nonnegative_finite),
        "shared pacing time and intervals must be finite and nonnegative"
    );
    ensure!(
        input.jitter_fraction.is_finite() && (0.0..=1.0).contains(&input.jitter_fraction),
        "shared pacing jitter fraction must be between zero and one"
    );
    ensure!(
        input
            .status_code
            .is_none_or(|status| (100..=599).contains(&status)),
        "invalid shared pacing status code"
    );
    let observed = DateTime::<Utc>::from_timestamp(
        input.now.trunc() as i64,
        (input.now.fract() * 1_000_000_000.0) as u32,
    )
    .context("shared pacing timestamp is outside the supported range")?
    .to_rfc3339_opts(SecondsFormat::Micros, false);

    let mut transaction = pool.begin_with("BEGIN IMMEDIATE").await?;
    sqlx::raw_sql(SCHEMA).execute(&mut *transaction).await?;
    let previous: Option<f64> =
        sqlx::query_scalar("SELECT next_allowed_at FROM shared_rate_limit_state WHERE name = ?")
            .bind(input.name)
            .fetch_optional(&mut *transaction)
            .await?;
    let previous = previous.unwrap_or(0.0);
    ensure!(
        nonnegative_finite(previous),
        "invalid stored pacing timestamp"
    );
    let scheduled = input.now.max(previous);
    let sleep_seconds = scheduled - input.now;
    let jitter = if input.interval_seconds > 0.0 {
        input.jitter_seconds * input.jitter_fraction
    } else {
        0.0
    };
    let next_allowed_at = scheduled + input.interval_seconds + jitter;
    ensure!(
        next_allowed_at.is_finite(),
        "shared pacing timestamp overflow"
    );
    let kind = if input.status_code.is_some() {
        "backoff"
    } else {
        "acquired"
    };

    sqlx::query(
        "INSERT INTO shared_rate_limit_state
         (name, next_allowed_at, last_acquired_at, last_owner_id, last_sleep_seconds, interval_seconds, jitter_seconds)
         VALUES (?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(name) DO UPDATE SET
         next_allowed_at=excluded.next_allowed_at,
         last_acquired_at=excluded.last_acquired_at,
         last_owner_id=excluded.last_owner_id,
         last_sleep_seconds=excluded.last_sleep_seconds,
         interval_seconds=excluded.interval_seconds,
         jitter_seconds=excluded.jitter_seconds",
    )
    .bind(input.name)
    .bind(next_allowed_at)
    .bind(&observed)
    .bind(input.owner_id)
    .bind(sleep_seconds)
    .bind(input.interval_seconds)
    .bind(input.jitter_seconds)
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        "INSERT INTO shared_rate_limit_events
         (name, kind, owner_id, status_code, delay_seconds, next_allowed_at, observed_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(input.name)
    .bind(kind)
    .bind(input.owner_id)
    .bind(input.status_code.map(i64::from))
    .bind(if input.status_code.is_some() {
        input.interval_seconds
    } else {
        sleep_seconds
    })
    .bind(next_allowed_at)
    .bind(&observed)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;

    Ok(Reservation {
        name: input.name.to_owned(),
        owner_id: input.owner_id.to_owned(),
        sleep_seconds,
        next_allowed_at,
        previous_next_allowed_at: previous,
        interval_seconds: input.interval_seconds,
        jitter_seconds: input.jitter_seconds,
        kind: kind.to_owned(),
        status_code: input.status_code,
    })
}

/// Read the existing Python-compatible snapshot and ten latest events.
/// An empty database has no snapshot. This operation creates no schema or state.
pub async fn read_status(pool: &SqlitePool, name: &str) -> Result<Option<SharedRateLimitStatus>> {
    ensure!(valid_identifier(name), "invalid shared pacing name");
    let mut transaction = pool.begin().await?;
    let tables: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table'
         AND name IN ('shared_rate_limit_state', 'shared_rate_limit_events')",
    )
    .fetch_one(&mut *transaction)
    .await?;
    if tables == 0 {
        return Ok(None);
    }
    ensure!(tables == 2, "incomplete shared pacing schema");
    let row = sqlx::query(
        "SELECT name, next_allowed_at, last_acquired_at, last_owner_id,
         last_sleep_seconds, interval_seconds, jitter_seconds
         FROM shared_rate_limit_state WHERE name = ?",
    )
    .bind(name)
    .fetch_optional(&mut *transaction)
    .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let rows = sqlx::query(
        "SELECT kind, owner_id, status_code, delay_seconds, next_allowed_at, observed_at
         FROM shared_rate_limit_events WHERE name = ? ORDER BY id DESC LIMIT 10",
    )
    .bind(name)
    .fetch_all(&mut *transaction)
    .await?;
    let mut events = Vec::with_capacity(rows.len());
    for row in rows {
        events.push(RecentEvent {
            kind: row.try_get("kind")?,
            owner_id: row.try_get("owner_id")?,
            status_code: row.try_get("status_code")?,
            delay_seconds: row.try_get("delay_seconds")?,
            next_allowed_at: row.try_get("next_allowed_at")?,
            observed_at: row.try_get("observed_at")?,
        });
    }
    let status = SharedRateLimitStatus {
        name: row.try_get("name")?,
        next_allowed_at: row.try_get("next_allowed_at")?,
        last_acquired_at: row.try_get("last_acquired_at")?,
        last_owner_id: row.try_get("last_owner_id")?,
        last_sleep_seconds: row.try_get("last_sleep_seconds")?,
        interval_seconds: row.try_get("interval_seconds")?,
        jitter_seconds: row.try_get("jitter_seconds")?,
        recent_events: events,
    };
    ensure!(
        [
            status.next_allowed_at,
            status.last_sleep_seconds,
            status.interval_seconds,
            status.jitter_seconds
        ]
        .into_iter()
        .all(nonnegative_finite)
            && status
                .recent_events
                .iter()
                .all(|event| nonnegative_finite(event.delay_seconds)
                    && nonnegative_finite(event.next_allowed_at)),
        "invalid stored shared pacing values"
    );
    transaction.commit().await?;
    Ok(Some(status))
}

/// Open an existing pacing database in read-only mode and return its snapshot.
pub async fn read_status_file(path: &Path, name: &str) -> Result<Option<SharedRateLimitStatus>> {
    let options = SqliteConnectOptions::new().filename(path).read_only(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;
    let result = read_status(&pool, name).await;
    pool.close().await;
    result
}
