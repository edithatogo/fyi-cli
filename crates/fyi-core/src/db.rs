use crate::api::{AlaveteliCorrespondence, AlaveteliRequest, CorrespondenceDirection};
use chrono::Utc;
use serde::Serialize;
use sqlx::{sqlite::SqlitePool, sqlite::SqlitePoolOptions, Row};
use std::time::Duration;

/// Request-level sync status persisted in `sync_metadata`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncStatus {
    Clean,
    Dirty,
    Pending,
    Conflict,
}

impl SyncStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Clean => "clean",
            Self::Dirty => "dirty",
            Self::Pending => "pending",
            Self::Conflict => "conflict",
        }
    }

    fn from_str(value: &str) -> Self {
        match value {
            "clean" => Self::Clean,
            "pending" => Self::Pending,
            "conflict" => Self::Conflict,
            _ => Self::Dirty,
        }
    }
}

/// Stored sync metadata for one request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestSyncMetadata {
    pub request_id: i64,
    pub remote_request_id: Option<i64>,
    pub last_synced_at: Option<String>,
    pub remote_updated_at: Option<String>,
    pub local_updated_at: String,
    pub sync_status: SyncStatus,
    pub conflict_version: i64,
}

/// Pending field-level local changes for conflict-aware sync.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldChange {
    pub request_id: i64,
    pub field_name: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub changed_at: String,
    pub synced_at: Option<String>,
}

/// Aggregate sync status for dashboard and API consumers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GlobalSyncStatus {
    pub total: i64,
    pub clean: i64,
    pub dirty: i64,
    pub pending: i64,
    pub conflict: i64,
}

/// Durable queued submission waiting to be pushed upstream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutgoingQueueItem {
    pub id: i64,
    pub request_id: i64,
    pub operation: String,
    pub payload: String,
    pub status: String,
    pub remote_request_id: Option<i64>,
    pub attempts: i64,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OutgoingQueueDepth {
    pub pending: i64,
    pub submitted: i64,
    pub failed: i64,
}

/// Database wrapper managing the connection pool and CRUD operations.
#[derive(Debug, Clone)]
pub struct DbPool {
    pool: SqlitePool,
}

impl DbPool {
    /// Create a new connection pool for the given SQLite database URL.
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = SqlitePoolOptions::new()
            .max_connections(4) // Optimized for SQLite concurrent reads and serialized writes
            .acquire_timeout(Duration::from_secs(3)) // Fail fast if DB is locked
            .idle_timeout(Duration::from_secs(10))
            .connect(database_url)
            .await?;
        Ok(Self { pool })
    }

    /// Create an in-memory SQLite connection pool for testing or transient usage.
    pub async fn new_in_memory() -> Result<Self, sqlx::Error> {
        Self::new("sqlite::memory:").await
    }

    /// Runs all embedded migrations on the database pool.
    pub async fn run_migrations(&self) -> Result<(), sqlx::migrate::MigrateError> {
        sqlx::migrate!("./migrations").run(&self.pool).await
    }

    /// Returns a reference to the underlying `SqlitePool`.
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Inserts or replaces a locally edited `AlaveteliRequest` and marks it dirty.
    pub async fn insert_request(&self, request: &AlaveteliRequest) -> Result<(), sqlx::Error> {
        let previous = self.get_request(request.id).await?;
        self.upsert_request_row(request).await?;
        self.record_local_request_change(request.id, previous.as_ref(), request)
            .await?;
        Ok(())
    }

    /// Inserts or replaces a remotely sourced request and marks it clean.
    pub async fn upsert_synced_request(
        &self,
        request: &AlaveteliRequest,
        remote_updated_at: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        self.upsert_request_row(request).await?;
        self.mark_request_clean(
            request.id,
            remote_updated_at.or(request.updated_at.as_deref()),
        )
        .await
    }

    async fn upsert_request_row(&self, request: &AlaveteliRequest) -> Result<(), sqlx::Error> {
        let tags_json = request
            .tags
            .as_ref()
            .map(|t| serde_json::to_string(t).unwrap_or_default());

        sqlx::query(
            "INSERT OR REPLACE INTO requests (id, title, body, user_name, status, created_at, updated_at, url, tags)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(request.id)
        .bind(&request.title)
        .bind(&request.body)
        .bind(&request.user_name)
        .bind(&request.status)
        .bind(&request.created_at)
        .bind(&request.updated_at)
        .bind(&request.url)
        .bind(tags_json)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Retrieves an `AlaveteliRequest` by ID.
    pub async fn get_request(&self, id: i64) -> Result<Option<AlaveteliRequest>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT id, title, body, user_name, status, created_at, updated_at, url, tags FROM requests WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let tags_str: Option<String> = row.try_get("tags")?;
            let tags = tags_str.and_then(|s| serde_json::from_str(&s).ok());
            Ok(Some(AlaveteliRequest {
                id: row.try_get("id")?,
                title: row.try_get("title")?,
                body: row.try_get("body")?,
                user_name: row.try_get("user_name")?,
                status: row.try_get("status")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
                url: row.try_get("url")?,
                tags,
            }))
        } else {
            Ok(None)
        }
    }

    /// Lists requests from newest to oldest, bounded by the supplied limit.
    pub async fn list_requests(&self, limit: i64) -> Result<Vec<AlaveteliRequest>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT id, title, body, user_name, status, created_at, updated_at, url, tags
             FROM requests
             ORDER BY COALESCE(updated_at, created_at, '') DESC, id DESC
             LIMIT ?",
        )
        .bind(limit.clamp(1, 500))
        .fetch_all(&self.pool)
        .await?;

        let mut requests = Vec::with_capacity(rows.len());
        for row in rows {
            let tags_str: Option<String> = row.try_get("tags")?;
            let tags = tags_str.and_then(|s| serde_json::from_str(&s).ok());
            requests.push(AlaveteliRequest {
                id: row.try_get("id")?,
                title: row.try_get("title")?,
                body: row.try_get("body")?,
                user_name: row.try_get("user_name")?,
                status: row.try_get("status")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
                url: row.try_get("url")?,
                tags,
            });
        }

        Ok(requests)
    }

    /// Updates a locally edited request, marks it dirty, and returns whether a row was changed.
    pub async fn update_request(&self, request: &AlaveteliRequest) -> Result<bool, sqlx::Error> {
        let previous = self.get_request(request.id).await?;
        let tags_json = request
            .tags
            .as_ref()
            .map(|t| serde_json::to_string(t).unwrap_or_default());

        let result = sqlx::query(
            "UPDATE requests
             SET title = ?, body = ?, user_name = ?, status = ?, updated_at = ?, url = ?, tags = ?
             WHERE id = ?",
        )
        .bind(&request.title)
        .bind(&request.body)
        .bind(&request.user_name)
        .bind(&request.status)
        .bind(&request.updated_at)
        .bind(&request.url)
        .bind(tags_json)
        .bind(request.id)
        .execute(&self.pool)
        .await?;

        let changed = result.rows_affected() > 0;
        if changed {
            self.record_local_request_change(request.id, previous.as_ref(), request)
                .await?;
        }

        Ok(changed)
    }

    /// Deletes a request by ID. Correspondence rows are removed by SQLite cascade.
    pub async fn delete_request(&self, id: i64) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM requests WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Inserts a new `AlaveteliCorrespondence` associated with a request ID.
    pub async fn insert_correspondence(
        &self,
        request_id: i64,
        correspondence: &AlaveteliCorrespondence,
    ) -> Result<(), sqlx::Error> {
        let direction_str = match correspondence.direction {
            CorrespondenceDirection::Request => "request",
            CorrespondenceDirection::Response => "response",
        };
        let attachments_json = correspondence
            .attachments
            .as_ref()
            .map(|a| serde_json::to_string(a).unwrap_or_default());

        sqlx::query(
            "INSERT INTO correspondence (request_id, direction, body, sent_at, state, attachments)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(request_id)
        .bind(direction_str)
        .bind(&correspondence.body)
        .bind(&correspondence.sent_at)
        .bind(&correspondence.state)
        .bind(attachments_json)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Retrieves all correspondences associated with a request ID, sorted chronologically.
    pub async fn get_correspondence_for_request(
        &self,
        request_id: i64,
    ) -> Result<Vec<AlaveteliCorrespondence>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT direction, body, sent_at, state, attachments FROM correspondence WHERE request_id = ? ORDER BY sent_at ASC"
        )
        .bind(request_id)
        .fetch_all(&self.pool)
        .await?;

        let mut list = Vec::new();
        for row in rows {
            let direction_str: String = row.try_get("direction")?;
            let direction = match direction_str.as_str() {
                "response" => CorrespondenceDirection::Response,
                _ => CorrespondenceDirection::Request,
            };
            let attachments_str: Option<String> = row.try_get("attachments")?;
            let attachments = attachments_str.and_then(|s| serde_json::from_str(&s).ok());
            list.push(AlaveteliCorrespondence {
                direction,
                body: row.try_get("body")?,
                sent_at: row.try_get("sent_at")?,
                state: row.try_get("state")?,
                attachments,
            });
        }
        Ok(list)
    }

    /// Returns sync metadata for a request, if the request has been tracked.
    pub async fn get_request_sync_metadata(
        &self,
        request_id: i64,
    ) -> Result<Option<RequestSyncMetadata>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT request_id, remote_request_id, last_synced_at, remote_updated_at, local_updated_at, sync_status, conflict_version
             FROM sync_metadata
             WHERE request_id = ?",
        )
        .bind(request_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(metadata_from_row).transpose()
    }

    /// Returns dirty local requests ready to be pushed upstream.
    pub async fn list_dirty_requests(
        &self,
        limit: i64,
    ) -> Result<Vec<AlaveteliRequest>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT r.id, r.title, r.body, r.user_name, r.status, r.created_at, r.updated_at, r.url, r.tags
             FROM requests r
             INNER JOIN sync_metadata sm ON sm.request_id = r.id
             WHERE sm.sync_status = 'dirty'
             ORDER BY sm.local_updated_at ASC, r.id ASC
             LIMIT ?",
        )
        .bind(limit.clamp(1, 500))
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(request_from_row).collect()
    }

    /// Returns requests currently marked as sync conflicts.
    pub async fn list_conflicted_requests(
        &self,
        limit: i64,
    ) -> Result<Vec<AlaveteliRequest>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT r.id, r.title, r.body, r.user_name, r.status, r.created_at, r.updated_at, r.url, r.tags
             FROM requests r
             INNER JOIN sync_metadata sm ON sm.request_id = r.id
             WHERE sm.sync_status = 'conflict'
             ORDER BY sm.conflict_version DESC, sm.local_updated_at DESC, r.id ASC
             LIMIT ?",
        )
        .bind(limit.clamp(1, 500))
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(request_from_row).collect()
    }

    /// Enqueues a local request submission for durable push processing.
    pub async fn enqueue_request_submission(
        &self,
        request: &AlaveteliRequest,
    ) -> Result<i64, sqlx::Error> {
        let payload = serde_json::to_string(request).unwrap_or_default();
        let now = now_timestamp();
        let result = sqlx::query(
            "INSERT INTO sync_outgoing_queue (
                request_id, operation, payload, status, updated_at
             )
             VALUES (?, 'upsert_request', ?, 'pending', ?)",
        )
        .bind(request.id)
        .bind(payload)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "UPDATE sync_metadata
             SET sync_status = 'pending', local_updated_at = ?
             WHERE request_id = ? AND sync_status != 'conflict'",
        )
        .bind(&now)
        .bind(request.id)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// Returns pending outgoing queue items.
    pub async fn list_pending_outgoing_queue(
        &self,
        limit: i64,
    ) -> Result<Vec<OutgoingQueueItem>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT id, request_id, operation, payload, status, remote_request_id, attempts, last_error
             FROM sync_outgoing_queue
             WHERE status = 'pending'
             ORDER BY created_at ASC, id ASC
             LIMIT ?",
        )
        .bind(limit.clamp(1, 500))
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(outgoing_queue_item_from_row).collect()
    }

    /// Returns queue depth by status for monitoring.
    pub async fn get_outgoing_queue_depth(&self) -> Result<OutgoingQueueDepth, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT status, COUNT(*) AS count
             FROM sync_outgoing_queue
             GROUP BY status",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut depth = OutgoingQueueDepth::default();
        for row in rows {
            let status: String = row.try_get("status")?;
            let count: i64 = row.try_get("count")?;
            match status.as_str() {
                "submitted" => depth.submitted = count,
                "failed" => depth.failed = count,
                _ => depth.pending = count,
            }
        }
        Ok(depth)
    }

    /// Marks an outgoing queue item as submitted and stores the FYI-issued request ID.
    pub async fn mark_submission_confirmed(
        &self,
        queue_id: i64,
        request_id: i64,
        remote_request_id: i64,
        remote_updated_at: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        let now = now_timestamp();
        sqlx::query(
            "UPDATE sync_outgoing_queue
             SET status = 'submitted', remote_request_id = ?, updated_at = ?
             WHERE id = ?",
        )
        .bind(remote_request_id)
        .bind(&now)
        .bind(queue_id)
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "UPDATE sync_metadata
             SET remote_request_id = ?
             WHERE request_id = ?",
        )
        .bind(remote_request_id)
        .bind(request_id)
        .execute(&self.pool)
        .await?;

        self.mark_request_clean(request_id, remote_updated_at).await
    }

    /// Marks an outgoing queue item as failed after retries are exhausted.
    pub async fn mark_submission_failed(
        &self,
        queue_id: i64,
        request_id: i64,
        attempts: i64,
        error: &str,
    ) -> Result<(), sqlx::Error> {
        let now = now_timestamp();
        sqlx::query(
            "UPDATE sync_outgoing_queue
             SET status = 'failed', attempts = ?, last_error = ?, updated_at = ?
             WHERE id = ?",
        )
        .bind(attempts)
        .bind(error)
        .bind(&now)
        .bind(queue_id)
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "UPDATE sync_metadata
             SET sync_status = 'dirty', local_updated_at = ?
             WHERE request_id = ? AND sync_status != 'conflict'",
        )
        .bind(&now)
        .bind(request_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Marks a request as conflicted and increments its conflict version.
    pub async fn mark_request_conflict(&self, request_id: i64) -> Result<(), sqlx::Error> {
        let now = now_timestamp();
        sqlx::query(
            "UPDATE sync_metadata
             SET sync_status = 'conflict',
                 conflict_version = conflict_version + 1,
                 remote_updated_at = COALESCE(remote_updated_at, ?),
                 local_updated_at = ?
             WHERE request_id = ?",
        )
        .bind(&now)
        .bind(&now)
        .bind(request_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Resolves a conflict by moving the request back to clean or dirty sync state.
    pub async fn resolve_request_conflict(
        &self,
        request_id: i64,
        as_clean: bool,
    ) -> Result<bool, sqlx::Error> {
        let now = now_timestamp();
        let status = if as_clean { "clean" } else { "dirty" };
        let result = sqlx::query(
            "UPDATE sync_metadata
             SET sync_status = ?, local_updated_at = ?
             WHERE request_id = ? AND sync_status = 'conflict'",
        )
        .bind(status)
        .bind(&now)
        .bind(request_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Returns aggregate sync counts across all tracked requests.
    pub async fn get_global_sync_status(&self) -> Result<GlobalSyncStatus, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT sync_status, COUNT(*) AS count
             FROM sync_metadata
             GROUP BY sync_status",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut status = GlobalSyncStatus::default();
        for row in rows {
            let sync_status: String = row.try_get("sync_status")?;
            let count: i64 = row.try_get("count")?;
            status.total += count;
            match SyncStatus::from_str(&sync_status) {
                SyncStatus::Clean => status.clean = count,
                SyncStatus::Dirty => status.dirty = count,
                SyncStatus::Pending => status.pending = count,
                SyncStatus::Conflict => status.conflict = count,
            }
        }

        Ok(status)
    }

    /// Returns the latest successful sync timestamp across tracked requests.
    pub async fn get_latest_sync_timestamp(&self) -> Result<Option<String>, sqlx::Error> {
        sqlx::query_scalar(
            "SELECT MAX(last_synced_at)
             FROM sync_metadata
             WHERE last_synced_at IS NOT NULL",
        )
        .fetch_one(&self.pool)
        .await
    }

    /// Returns unsynced field-level changes for a request.
    pub async fn list_unsynced_field_changes(
        &self,
        request_id: i64,
    ) -> Result<Vec<FieldChange>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT request_id, field_name, old_value, new_value, changed_at, synced_at
             FROM sync_field_changes
             WHERE request_id = ? AND synced_at IS NULL
             ORDER BY changed_at ASC, id ASC",
        )
        .bind(request_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(field_change_from_row).collect()
    }

    /// Marks a request clean after a confirmed remote sync.
    pub async fn mark_request_clean(
        &self,
        request_id: i64,
        remote_updated_at: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        let now = now_timestamp();
        sqlx::query(
            "INSERT INTO sync_metadata (
                request_id, last_synced_at, remote_updated_at, local_updated_at, sync_status, conflict_version
             )
             VALUES (?, ?, ?, ?, 'clean', 0)
             ON CONFLICT(request_id) DO UPDATE SET
                last_synced_at = excluded.last_synced_at,
                remote_updated_at = excluded.remote_updated_at,
                sync_status = 'clean'",
        )
        .bind(request_id)
        .bind(&now)
        .bind(remote_updated_at)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "UPDATE sync_field_changes
             SET synced_at = ?
             WHERE request_id = ? AND synced_at IS NULL",
        )
        .bind(&now)
        .bind(request_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn record_local_request_change(
        &self,
        request_id: i64,
        previous: Option<&AlaveteliRequest>,
        current: &AlaveteliRequest,
    ) -> Result<(), sqlx::Error> {
        let now = now_timestamp();
        sqlx::query(
            "INSERT INTO sync_metadata (
                request_id, local_updated_at, sync_status, conflict_version
             )
             VALUES (?, ?, 'dirty', 0)
             ON CONFLICT(request_id) DO UPDATE SET
                local_updated_at = excluded.local_updated_at,
                sync_status = CASE
                    WHEN sync_metadata.sync_status = 'conflict' THEN 'conflict'
                    ELSE 'dirty'
                END",
        )
        .bind(request_id)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        for (field_name, old_value, new_value) in diff_request_fields(previous, current) {
            sqlx::query(
                "INSERT INTO sync_field_changes (
                    request_id, field_name, old_value, new_value, changed_at
                 )
                 VALUES (?, ?, ?, ?, ?)",
            )
            .bind(request_id)
            .bind(field_name)
            .bind(old_value)
            .bind(new_value)
            .bind(&now)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }
}

fn metadata_from_row(row: sqlx::sqlite::SqliteRow) -> Result<RequestSyncMetadata, sqlx::Error> {
    let sync_status: String = row.try_get("sync_status")?;
    Ok(RequestSyncMetadata {
        request_id: row.try_get("request_id")?,
        remote_request_id: row.try_get("remote_request_id")?,
        last_synced_at: row.try_get("last_synced_at")?,
        remote_updated_at: row.try_get("remote_updated_at")?,
        local_updated_at: row.try_get("local_updated_at")?,
        sync_status: SyncStatus::from_str(&sync_status),
        conflict_version: row.try_get("conflict_version")?,
    })
}

fn request_from_row(row: sqlx::sqlite::SqliteRow) -> Result<AlaveteliRequest, sqlx::Error> {
    let tags_str: Option<String> = row.try_get("tags")?;
    let tags = tags_str.and_then(|s| serde_json::from_str(&s).ok());
    Ok(AlaveteliRequest {
        id: row.try_get("id")?,
        title: row.try_get("title")?,
        body: row.try_get("body")?,
        user_name: row.try_get("user_name")?,
        status: row.try_get("status")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
        url: row.try_get("url")?,
        tags,
    })
}

fn outgoing_queue_item_from_row(
    row: sqlx::sqlite::SqliteRow,
) -> Result<OutgoingQueueItem, sqlx::Error> {
    Ok(OutgoingQueueItem {
        id: row.try_get("id")?,
        request_id: row.try_get("request_id")?,
        operation: row.try_get("operation")?,
        payload: row.try_get("payload")?,
        status: row.try_get("status")?,
        remote_request_id: row.try_get("remote_request_id")?,
        attempts: row.try_get("attempts")?,
        last_error: row.try_get("last_error")?,
    })
}

fn field_change_from_row(row: sqlx::sqlite::SqliteRow) -> Result<FieldChange, sqlx::Error> {
    Ok(FieldChange {
        request_id: row.try_get("request_id")?,
        field_name: row.try_get("field_name")?,
        old_value: row.try_get("old_value")?,
        new_value: row.try_get("new_value")?,
        changed_at: row.try_get("changed_at")?,
        synced_at: row.try_get("synced_at")?,
    })
}

fn diff_request_fields(
    previous: Option<&AlaveteliRequest>,
    current: &AlaveteliRequest,
) -> Vec<(&'static str, Option<String>, Option<String>)> {
    let old = previous;
    let fields = [
        (
            "title",
            old.map(|request| json_value(&request.title)),
            Some(json_value(&current.title)),
        ),
        (
            "body",
            old.map(|request| json_value(&request.body)),
            Some(json_value(&current.body)),
        ),
        (
            "user_name",
            old.and_then(|request| request.user_name.as_ref().map(json_value)),
            current.user_name.as_ref().map(json_value),
        ),
        (
            "status",
            old.and_then(|request| request.status.as_ref().map(json_value)),
            current.status.as_ref().map(json_value),
        ),
        (
            "created_at",
            old.and_then(|request| request.created_at.as_ref().map(json_value)),
            current.created_at.as_ref().map(json_value),
        ),
        (
            "updated_at",
            old.and_then(|request| request.updated_at.as_ref().map(json_value)),
            current.updated_at.as_ref().map(json_value),
        ),
        (
            "url",
            old.and_then(|request| request.url.as_ref().map(json_value)),
            current.url.as_ref().map(json_value),
        ),
        (
            "tags",
            old.and_then(|request| request.tags.as_ref().map(json_value)),
            current.tags.as_ref().map(json_value),
        ),
    ];

    fields
        .into_iter()
        .filter(|(_, old_value, new_value)| old_value != new_value)
        .collect()
}

fn json_value<T: Serialize>(value: T) -> String {
    serde_json::to_string(&value).unwrap_or_else(|_| "null".to_string())
}

fn now_timestamp() -> String {
    Utc::now().to_rfc3339()
}
