//! Local tracking repositories. These operations never enter the remote sync queue.
use super::DbPool;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{ConnectOptions, Row};
use std::collections::HashSet;
use std::path::Path;
use std::time::Duration;

pub const MAX_LOCAL_INPUT_BYTES: usize = 16 * 1024 * 1024;

/// Local diagnostics deliberately omit SQL parameters, source rows and request bodies.
#[derive(Debug, thiserror::Error)]
pub enum LocalError {
    #[error("{0}")]
    InvalidInput(&'static str),
    #[error("local database operation failed")]
    Database(#[from] sqlx::Error),
    #[error(
        "local database schema is unsupported or incomplete; initialize or migrate it explicitly"
    )]
    Schema,
    #[error("unable to open the existing local database")]
    Open,
}

/// Persisted local authority. Unlike the remote API DTO, keys and names are required.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LocalAuthority {
    pub slug: String,
    pub name: String,
    pub url: Option<String>,
}

impl LocalAuthority {
    pub fn validate(&self) -> Result<(), LocalError> {
        if self.slug.trim().is_empty()
            || self.slug.len() > 1024
            || self.slug.chars().any(char::is_control)
        {
            return Err(LocalError::InvalidInput("invalid authority slug"));
        }
        if self.name.trim().is_empty()
            || self.name.len() > 16 * 1024
            || self
                .name
                .chars()
                .any(|c| c.is_control() && c != '\n' && c != '\r' && c != '\t')
        {
            return Err(LocalError::InvalidInput("invalid authority name"));
        }
        if self
            .url
            .as_ref()
            .is_some_and(|url| url.len() > 16 * 1024 || url.chars().any(char::is_control))
        {
            return Err(LocalError::InvalidInput("invalid authority URL"));
        }
        Ok(())
    }
}

/// Decode the local Python catalogue aliases using the standard CSV parser.
/// Validate the entire bounded input before opening a write transaction.
pub fn parse_authorities_csv(input: &[u8]) -> Result<Vec<LocalAuthority>, LocalError> {
    if input.len() > MAX_LOCAL_INPUT_BYTES {
        return Err(LocalError::InvalidInput("authority CSV exceeds 16 MiB"));
    }
    let text = std::str::from_utf8(input)
        .map_err(|_| LocalError::InvalidInput("authority CSV must be UTF-8"))?
        .trim_start_matches('\u{feff}');
    check_csv_quoting(text.as_bytes())?;
    let mut reader = csv::ReaderBuilder::new().from_reader(text.as_bytes());
    let headers = reader
        .headers()
        .map_err(|_| LocalError::InvalidInput("invalid authority CSV headers"))?
        .clone();
    let slug_names = ["url_name", "slug", "authority_slug"];
    let name_names = ["name", "authority_name", "public_body_name"];
    let url_names = ["url", "authority_url", "request_url"];
    let unique: HashSet<_> = headers.iter().collect();
    if unique.len() != headers.len()
        || !headers.iter().any(|value| slug_names.contains(&value))
        || !headers.iter().any(|value| name_names.contains(&value))
    {
        return Err(LocalError::InvalidInput(
            "missing or duplicate authority CSV headers",
        ));
    }
    let mut authorities = Vec::new();
    for record in reader.records() {
        let record = record.map_err(|_| LocalError::InvalidInput("invalid authority CSV row"))?;
        let value = |aliases: &[&str]| {
            aliases.iter().find_map(|alias| {
                headers
                    .iter()
                    .position(|header| header == *alias)
                    .and_then(|index| record.get(index))
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_owned)
            })
        };
        let authority = LocalAuthority {
            slug: value(&slug_names).unwrap_or_default(),
            name: value(&name_names).unwrap_or_default(),
            url: value(&url_names),
        };
        authority.validate()?;
        authorities.push(authority);
    }
    Ok(authorities)
}

// The CSV crate deliberately accepts unfinished quoted fields. Reject truncation
// and ambiguous quoting before it can become a successful catalogue update.
fn check_csv_quoting(input: &[u8]) -> Result<(), LocalError> {
    #[derive(Clone, Copy)]
    enum State {
        Start,
        Plain,
        Quoted,
        Closed,
    }
    let mut state = State::Start;
    for &byte in input {
        state = match (state, byte) {
            (State::Start, b'"') | (State::Closed, b'"') => State::Quoted,
            (State::Quoted, b'"') => State::Closed,
            (State::Quoted, _) => State::Quoted,
            (_, b',' | b'\r' | b'\n') => State::Start,
            (State::Closed, _) | (State::Plain, b'"') => {
                return Err(LocalError::InvalidInput("invalid authority CSV quoting"));
            }
            _ => State::Plain,
        };
    }
    if matches!(state, State::Quoted) {
        return Err(LocalError::InvalidInput("unterminated authority CSV field"));
    }
    Ok(())
}

impl DbPool {
    /// Open an existing file by filename, never by interpolating a SQLite URI.
    /// Reads do not create files or run migrations. Statement logging is disabled
    /// because local tracking can contain sensitive request text.
    pub async fn open_local_existing(path: &Path, read_only: bool) -> Result<Self, LocalError> {
        let metadata = std::fs::symlink_metadata(path).map_err(|_| LocalError::Open)?;
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err(LocalError::Open);
        }
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(false)
            .read_only(read_only)
            .foreign_keys(true)
            .busy_timeout(Duration::from_secs(3))
            .disable_statement_logging();
        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .acquire_timeout(Duration::from_secs(3))
            .connect_with(options)
            .await
            .map_err(|_| LocalError::Open)?;
        let db = Self { pool };
        if let Err(error) = db.validate_local_schema().await {
            db.pool.close().await;
            return Err(error);
        }
        Ok(db)
    }

    /// Check version and checksum evidence without changing the migration ledger.
    pub async fn validate_local_schema(&self) -> Result<(), LocalError> {
        let applied: Vec<(i64, bool, Vec<u8>)> = sqlx::query_as(
            "SELECT version, success, checksum FROM _sqlx_migrations ORDER BY version",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|_| LocalError::Schema)?;
        let migrations = sqlx::migrate!("./migrations");
        if applied.len() != migrations.iter().count()
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
        self.validate_local_authorities_schema(true).await?;
        Ok(())
    }

    /// Refuse an incompatible pre-existing table before adopting a migration ledger.
    pub(crate) async fn validate_local_authorities_schema(
        &self,
        required: bool,
    ) -> Result<(), LocalError> {
        let rows = sqlx::query("PRAGMA table_info(authorities)")
            .fetch_all(&self.pool)
            .await?;
        if rows.is_empty() && !required {
            return Ok(());
        }
        let expected = [("slug", 0_i64, 1_i64), ("name", 1, 0), ("url", 0, 0)];
        if rows.len() != expected.len() {
            return Err(LocalError::Schema);
        }
        for (row, (name, not_null, primary_key)) in rows.iter().zip(expected) {
            if row.try_get::<String, _>("name")? != name
                || !row.try_get::<String, _>("type")?.eq_ignore_ascii_case("TEXT")
                || row.try_get::<i64, _>("notnull")? != not_null
                || row.try_get::<i64, _>("pk")? != primary_key
            {
                return Err(LocalError::Schema);
            }
        }
        Ok(())
    }

    /// Upsert a complete catalogue atomically; keep existing dependent references.
    pub async fn import_local_authorities(
        &self,
        authorities: &[LocalAuthority],
    ) -> Result<usize, LocalError> {
        for authority in authorities {
            authority.validate()?;
        }
        let mut transaction = self.pool.begin().await?;
        for authority in authorities {
            sqlx::query(
                "INSERT INTO authorities (slug, name, url) VALUES (?, ?, ?)
                 ON CONFLICT(slug) DO UPDATE SET name = excluded.name, url = excluded.url
                 WHERE authorities.name IS NOT excluded.name OR authorities.url IS NOT excluded.url",
            )
            .bind(&authority.slug)
            .bind(&authority.name)
            .bind(&authority.url)
            .execute(&mut *transaction)
            .await?;
        }
        transaction.commit().await?;
        Ok(authorities.len())
    }

    /// Preserve Python name ordering and add an explicit deterministic slug tie-break.
    pub async fn list_local_authorities(&self) -> Result<Vec<LocalAuthority>, LocalError> {
        let rows = sqlx::query("SELECT slug, name, url FROM authorities ORDER BY name, slug")
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter()
            .map(|row| {
                Ok(LocalAuthority {
                    slug: row.try_get("slug")?,
                    name: row.try_get("name")?,
                    url: row.try_get("url")?,
                })
            })
            .collect()
    }
}
