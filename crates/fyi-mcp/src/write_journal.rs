//! Bounded persistent reservations and redacted audit events for remote writes.
//!
//! A committed reservation precedes every dispatch. It is never automatically
//! removed: interruption, timeout and uncertain upstream results cannot authorize
//! another attempt after a process restart. Request bodies, result bodies, raw
//! idempotency keys and confirmation/credential tokens are not persisted.

use fyi_core::security::{generate_totp_secret, ZeroizedBytes};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::{ConnectOptions, Sqlite, SqlitePool, Transaction};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

const DATABASE_FILE: &str = "remote-write-v1.sqlite3";
const MARKER_FILE: &str = "remote-write-v1.marker";
const FORMAT_PREFIX: &str = "fyi-mcp-write-journal-v1\n";
const MAX_ATTEMPTS: i64 = 10_000;
const MAX_EVENTS: i64 = 100_000;
const MAX_DATABASE_BYTES: u64 = 64 * 1024 * 1024;
const STORAGE_UNAVAILABLE: &str =
    "remote write journal is unavailable; no new write may be dispatched";
const REPLAY_BLOCKED: &str =
    "a durable write attempt already exists; reconcile its outcome before retrying";
const CAPACITY_REACHED: &str =
    "remote write journal capacity reached; reconcile and retain existing evidence";

/// Validated selector for one instance, operation and caller idempotency key.
/// Debug output deliberately does not include its private fields.
pub struct WriteScope<'a> {
    instance: &'a str,
    operation: &'a str,
    key: &'a str,
}

impl<'a> WriteScope<'a> {
    /// Validate bounded exact identifiers without normalizing their identity.
    pub fn new(instance: &'a str, operation: &'a str, key: &'a str) -> Result<Self, String> {
        let safe = |value: &str, maximum: usize| {
            !value.is_empty()
                && value.len() <= maximum
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"-_.".contains(&byte))
        };
        if !safe(instance, 64) || !safe(key, 128) || !known_operation(operation) {
            return Err("remote write scope is invalid".into());
        }
        Ok(Self {
            instance,
            operation,
            key,
        })
    }
}

/// Private local SQLite ledger. Clones share connections and zeroized key storage.
#[derive(Clone)]
pub struct WriteJournal {
    pool: SqlitePool,
    key: Arc<ZeroizedBytes>,
    max_attempts: i64,
    max_events: i64,
}

impl std::fmt::Debug for WriteJournal {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("WriteJournal")
            .finish_non_exhaustive()
    }
}

impl WriteJournal {
    /// Close connections before an operator stops or inspects the local journal.
    pub async fn close(&self) {
        self.pool.close().await;
    }

    /// Open a private absolute state directory, creating a v1 journal once.
    /// An incomplete, missing, corrupt or unknown existing format fails closed.
    pub async fn open(directory: &Path) -> Result<Self, String> {
        Self::open_with_limits(directory, MAX_ATTEMPTS, MAX_EVENTS).await
    }

    async fn open_with_limits(
        directory: &Path,
        max_attempts: i64,
        max_events: i64,
    ) -> Result<Self, String> {
        if !directory.is_absolute() || max_attempts < 1 || max_events < 1 {
            return Err(STORAGE_UNAVAILABLE.into());
        }
        ensure_private_directory(directory)?;
        let database = directory.join(DATABASE_FILE);
        let marker = directory.join(MARKER_FILE);
        let database_exists = checked_exists(&database)?;
        let marker_exists = checked_exists(&marker)?;
        if database_exists != marker_exists {
            return Err(STORAGE_UNAVAILABLE.into());
        }
        if database_exists {
            check_private_file(&database, MAX_DATABASE_BYTES)?;
            check_private_file(&marker, 128)?;
        } else {
            create_private_file(&database)?
                .sync_all()
                .map_err(unavailable)?;
        }
        let options = SqliteConnectOptions::new()
            .filename(&database)
            .create_if_missing(false)
            .journal_mode(SqliteJournalMode::Delete)
            .synchronous(SqliteSynchronous::Full)
            .busy_timeout(Duration::from_secs(5))
            .pragma("max_page_count", "16384")
            .pragma("page_size", "4096")
            .pragma("cache_size", "-2048")
            .disable_statement_logging();
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .map_err(unavailable)?;
        if !database_exists {
            let mut transaction = pool
                .begin_with("BEGIN IMMEDIATE")
                .await
                .map_err(unavailable)?;
            sqlx::query("CREATE TABLE mcp_write_meta (version INTEGER PRIMARY KEY CHECK(version = 1), mac_key BLOB NOT NULL CHECK(length(mac_key) = 32))")
                .execute(&mut *transaction).await.map_err(unavailable)?;
            sqlx::query("CREATE TABLE mcp_write_attempts (scope BLOB PRIMARY KEY CHECK(length(scope) = 32), operation TEXT NOT NULL CHECK(operation IN ('remote_create_request','remote_add_correspondence','remote_update_state')), outcome TEXT NOT NULL CHECK(outcome IN ('reserved','succeeded','uncertain')))")
                .execute(&mut *transaction).await.map_err(unavailable)?;
            sqlx::query("CREATE TABLE mcp_write_audit (sequence INTEGER PRIMARY KEY AUTOINCREMENT, at_ms INTEGER NOT NULL, operation TEXT NOT NULL CHECK(operation IN ('remote_create_request','remote_add_correspondence','remote_update_state')), event TEXT NOT NULL CHECK(event IN ('prepare','commit','reject','replay','success','failure')), scope BLOB CHECK(scope IS NULL OR length(scope) = 32))")
                .execute(&mut *transaction).await.map_err(unavailable)?;
            let secret = generate_totp_secret().map_err(unavailable)?;
            sqlx::query("INSERT INTO mcp_write_meta(version, mac_key) VALUES (1, ?)")
                .bind(secret.as_str().as_bytes())
                .execute(&mut *transaction)
                .await
                .map_err(unavailable)?;
            sqlx::query("PRAGMA user_version = 1")
                .execute(&mut *transaction)
                .await
                .map_err(unavailable)?;
            transaction.commit().await.map_err(unavailable)?;
        }
        let version: i64 = sqlx::query_scalar("PRAGMA user_version")
            .fetch_one(&pool)
            .await
            .map_err(unavailable)?;
        let page_size: i64 = sqlx::query_scalar("PRAGMA page_size")
            .fetch_one(&pool)
            .await
            .map_err(unavailable)?;
        let integrity: String = sqlx::query_scalar("PRAGMA quick_check")
            .fetch_one(&pool)
            .await
            .map_err(unavailable)?;
        if version != 1 || page_size != 4096 || integrity != "ok" {
            pool.close().await;
            return Err(STORAGE_UNAVAILABLE.into());
        }
        let key: Vec<u8> =
            sqlx::query_scalar("SELECT mac_key FROM mcp_write_meta WHERE version = 1")
                .fetch_one(&pool)
                .await
                .map_err(unavailable)?;
        if key.len() != 32 {
            pool.close().await;
            return Err(STORAGE_UNAVAILABLE.into());
        }
        let fingerprint = format!("{FORMAT_PREFIX}{:x}\n", Sha256::digest(&key));
        if database_exists {
            if fs::read_to_string(&marker).map_err(unavailable)? != fingerprint {
                pool.close().await;
                return Err(STORAGE_UNAVAILABLE.into());
            }
        } else {
            let mut file = create_private_file(&marker)?;
            file.write_all(fingerprint.as_bytes())
                .map_err(unavailable)?;
            file.sync_all().map_err(unavailable)?;
            #[cfg(unix)]
            {
                File::open(directory)
                    .and_then(|directory| directory.sync_all())
                    .map_err(unavailable)?;
                File::open(directory.parent().ok_or(STORAGE_UNAVAILABLE)?)
                    .and_then(|parent| parent.sync_all())
                    .map_err(unavailable)?;
            }
        }
        Ok(Self {
            pool,
            key: Arc::new(ZeroizedBytes::new(key)),
            max_attempts,
            max_events,
        })
    }

    fn tag(&self, scope: &WriteScope<'_>) -> [u8; 32] {
        // Fixed field boundaries and domain separation prevent concatenation
        // aliases. The journal-local secret prevents public dictionary hashes.
        let mut mac = Hmac::<Sha256>::new_from_slice(self.key.as_slice())
            .expect("HMAC accepts a key of any length");
        mac.update(b"fyi-mcp-write-scope-v1\0");
        for value in [scope.instance, scope.operation, scope.key] {
            mac.update(&(value.len() as u64).to_be_bytes());
            mac.update(value.as_bytes());
        }
        mac.finalize().into_bytes().into()
    }

    async fn transaction(&self) -> Result<Transaction<'static, Sqlite>, String> {
        let mut transaction = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(unavailable)?;
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM mcp_write_audit")
            .fetch_one(&mut *transaction)
            .await
            .map_err(unavailable)?;
        if count >= self.max_events {
            return Err(CAPACITY_REACHED.into());
        }
        Ok(transaction)
    }

    /// Check restart reservations and atomically audit preparation or cached replay.
    /// `cached` is true only for a matching in-process completed result.
    pub async fn check_prepare(&self, scope: &WriteScope<'_>, cached: bool) -> Result<(), String> {
        let tag = self.tag(scope);
        let mut transaction = self.transaction().await?;
        let outcome: Option<String> =
            sqlx::query_scalar("SELECT outcome FROM mcp_write_attempts WHERE scope = ?")
                .bind(tag.as_slice())
                .fetch_optional(&mut *transaction)
                .await
                .map_err(unavailable)?;
        let allowed = if cached {
            outcome.as_deref() == Some("succeeded")
        } else {
            outcome.is_none()
        };
        let event = if cached || outcome.is_some() {
            "replay"
        } else {
            "prepare"
        };
        audit(&mut transaction, scope.operation, event, Some(&tag)).await?;
        transaction.commit().await.map_err(unavailable)?;
        if allowed {
            Ok(())
        } else {
            Err(REPLAY_BLOCKED.into())
        }
    }

    /// Persist and audit one reservation before callers may perform network I/O.
    pub async fn reserve(&self, scope: &WriteScope<'_>) -> Result<(), String> {
        let tag = self.tag(scope);
        let mut transaction = self.transaction().await?;
        let exists: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM mcp_write_attempts WHERE scope = ?")
                .bind(tag.as_slice())
                .fetch_one(&mut *transaction)
                .await
                .map_err(unavailable)?;
        if exists != 0 {
            audit(&mut transaction, scope.operation, "replay", Some(&tag)).await?;
            transaction.commit().await.map_err(unavailable)?;
            return Err(REPLAY_BLOCKED.into());
        }
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM mcp_write_attempts")
            .fetch_one(&mut *transaction)
            .await
            .map_err(unavailable)?;
        if count >= self.max_attempts {
            audit(&mut transaction, scope.operation, "reject", Some(&tag)).await?;
            transaction.commit().await.map_err(unavailable)?;
            return Err(CAPACITY_REACHED.into());
        }
        sqlx::query(
            "INSERT INTO mcp_write_attempts(scope, operation, outcome) VALUES (?, ?, 'reserved')",
        )
        .bind(tag.as_slice())
        .bind(scope.operation)
        .execute(&mut *transaction)
        .await
        .map_err(unavailable)?;
        audit(&mut transaction, scope.operation, "commit", Some(&tag)).await?;
        transaction.commit().await.map_err(unavailable)
    }

    /// Record success or an uncertain failure without ever releasing the key.
    pub async fn complete(&self, scope: &WriteScope<'_>, succeeded: bool) -> Result<(), String> {
        let tag = self.tag(scope);
        let mut transaction = self.transaction().await?;
        let changed = sqlx::query(
            "UPDATE mcp_write_attempts SET outcome = ? WHERE scope = ? AND outcome = 'reserved'",
        )
        .bind(if succeeded { "succeeded" } else { "uncertain" })
        .bind(tag.as_slice())
        .execute(&mut *transaction)
        .await
        .map_err(unavailable)?
        .rows_affected();
        if changed != 1 {
            audit(&mut transaction, scope.operation, "reject", Some(&tag)).await?;
            transaction.commit().await.map_err(unavailable)?;
            return Err(STORAGE_UNAVAILABLE.into());
        }
        audit(
            &mut transaction,
            scope.operation,
            if succeeded { "success" } else { "failure" },
            Some(&tag),
        )
        .await?;
        transaction.commit().await.map_err(unavailable)
    }

    /// Rejections contain only a fixed operation label, never rejected input.
    pub async fn record_rejection(&self, operation: &str) -> Result<(), String> {
        if !known_operation(operation) {
            return Err("remote write scope is invalid".into());
        }
        let mut transaction = self.transaction().await?;
        audit(&mut transaction, operation, "reject", None).await?;
        transaction.commit().await.map_err(unavailable)
    }
}

fn known_operation(operation: &str) -> bool {
    matches!(
        operation,
        "remote_create_request" | "remote_add_correspondence" | "remote_update_state"
    )
}

async fn audit(
    transaction: &mut Transaction<'_, Sqlite>,
    operation: &str,
    event: &str,
    scope: Option<&[u8; 32]>,
) -> Result<(), String> {
    sqlx::query("INSERT INTO mcp_write_audit(at_ms, operation, event, scope) VALUES (?, ?, ?, ?)")
        .bind(chrono::Utc::now().timestamp_millis())
        .bind(operation)
        .bind(event)
        .bind(scope.map(|scope| scope.as_slice()))
        .execute(&mut **transaction)
        .await
        .map_err(unavailable)?;
    Ok(())
}

fn unavailable(_: impl std::fmt::Display) -> String {
    STORAGE_UNAVAILABLE.into()
}

fn checked_exists(path: &Path) -> Result<bool, String> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(unavailable(error)),
    }
}

fn ensure_private_directory(path: &Path) -> Result<(), String> {
    if !checked_exists(path)? {
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(path).map_err(unavailable)?;
    }
    let metadata = fs::symlink_metadata(path).map_err(unavailable)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(STORAGE_UNAVAILABLE.into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(STORAGE_UNAVAILABLE.into());
        }
    }
    Ok(())
}

fn check_private_file(path: &Path, maximum: u64) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path).map_err(unavailable)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > maximum {
        return Err(STORAGE_UNAVAILABLE.into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(STORAGE_UNAVAILABLE.into());
        }
    }
    Ok(())
}

fn create_private_file(path: &Path) -> Result<File, String> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path).map_err(unavailable)
}

#[cfg(test)]
#[path = "write_journal_tests.rs"]
mod tests;
