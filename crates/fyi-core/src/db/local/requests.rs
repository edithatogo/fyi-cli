//! Local request tracking remains separate from remote request synchronization.

use super::{LocalError, MAX_LOCAL_INPUT_BYTES};
use crate::db::DbPool;
use chrono::{DateTime, NaiveDateTime, SecondsFormat, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use sqlx::{Connection, FromRow, Sqlite, SqliteConnection, Transaction};
use std::cmp::Ordering;

pub const MAX_LOCAL_STATUS_BYTES: usize = 128;
const MAX_LOCAL_TEXT_FIELD_BYTES: usize = 16 * 1024;

/// A new local registration. An external ID records a link, never submission intent.
#[derive(Debug, Clone)]
pub struct NewLocalRequest {
    pub authority_slug: String,
    pub title: String,
    pub body: String,
    pub tags: String,
    pub status: String,
    pub fyi_request_id: Option<i64>,
}

impl NewLocalRequest {
    fn validate(&self) -> Result<(), LocalError> {
        if self.authority_slug.trim().is_empty()
            || self.authority_slug.len() > 1024
            || self.authority_slug.chars().any(char::is_control)
        {
            return Err(LocalError::InvalidInput("invalid authority slug"));
        }
        if self.title.trim().is_empty() || self.title.len() > MAX_LOCAL_TEXT_FIELD_BYTES {
            return Err(LocalError::InvalidInput("invalid request title"));
        }
        if self.body.trim().is_empty() || self.body.len() > MAX_LOCAL_INPUT_BYTES {
            return Err(LocalError::InvalidInput("invalid request body"));
        }
        if self.tags.len() > MAX_LOCAL_TEXT_FIELD_BYTES {
            return Err(LocalError::InvalidInput("request tags exceed 16 KiB"));
        }
        if self.fyi_request_id.is_some_and(|id| id <= 0) {
            return Err(LocalError::InvalidInput("external request ID must be positive"));
        }
        validate_status(&self.status)
    }
}

/// The complete twelve-field Python local request projection, including nulls.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TrackedRequest {
    pub id: i64,
    pub authority_slug: String,
    pub title: String,
    pub body: String,
    #[serde(deserialize_with = "required_nullable")]
    pub tags: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub fyi_url: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub fyi_request_id: Option<i64>,
    #[serde(deserialize_with = "required_nullable")]
    pub status: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub created_at: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub updated_at: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub last_seen_at: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub last_event_title: Option<String>,
}

fn required_nullable<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

/// Listing deliberately omits request bodies, tags and provenance payloads.
#[derive(Debug, Clone, Serialize, FromRow, PartialEq, Eq)]
pub struct LocalRequestSummary {
    pub id: i64,
    pub authority_slug: String,
    pub title: String,
    pub status: Option<String>,
    pub fyi_request_id: Option<i64>,
}

/// Local timeline projection; nullable timestamps also support retained legacy data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TimelineItem {
    #[serde(deserialize_with = "required_nullable")]
    pub ts: Option<String>,
    pub kind: String,
    pub title: String,
    pub detail: String,
}

/// The request and its timeline are read together from one SQLite snapshot.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct LocalRequestDetail {
    #[serde(flatten)]
    pub request: TrackedRequest,
    pub timeline: Vec<TimelineItem>,
}

#[derive(FromRow)]
struct HistoryProjection {
    kind: String,
    new_status: String,
    occurred_at: String,
}

pub(crate) fn validate_status(status: &str) -> Result<(), LocalError> {
    if status.trim().is_empty()
        || status.len() > MAX_LOCAL_STATUS_BYTES
        || status.chars().any(char::is_control)
    {
        return Err(LocalError::InvalidInput(
            "status must contain 1 to 128 UTF-8 bytes and no control characters",
        ));
    }
    Ok(())
}

fn validate_request_id(id: i64) -> Result<(), LocalError> {
    if id <= 0 {
        return Err(LocalError::InvalidInput("local request ID must be positive"));
    }
    Ok(())
}

fn now_timestamp() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Micros, true)
}

async fn finish_transaction<T>(
    transaction: Transaction<'_, Sqlite>,
    result: Result<T, LocalError>,
) -> Result<T, LocalError> {
    match result {
        Ok(value) => {
            transaction.commit().await?;
            Ok(value)
        }
        Err(error) => {
            transaction.rollback().await?;
            Err(error)
        }
    }
}

impl DbPool {
    /// Register a local request and its creation event atomically, without sync state.
    pub async fn register_local_request(
        &self,
        request: &NewLocalRequest,
    ) -> Result<i64, LocalError> {
        request.validate()?;
        let mut connection = self.pool().acquire().await?;
        let mut transaction = connection.begin_with("BEGIN IMMEDIATE").await?;
        let result = insert_request_in_connection(&mut transaction, request).await;
        finish_transaction(transaction, result).await
    }

    /// List local identities in ascending ID order without exposing request bodies.
    pub async fn list_local_requests(&self) -> Result<Vec<LocalRequestSummary>, LocalError> {
        Ok(sqlx::query_as(
            "SELECT id, authority_slug, title, status, fyi_request_id
             FROM tracked_requests ORDER BY id",
        )
        .fetch_all(self.pool())
        .await?)
    }

    /// Read one complete local record without changing nullable legacy values.
    pub async fn get_local_request(&self, id: i64) -> Result<Option<TrackedRequest>, LocalError> {
        validate_request_id(id)?;
        let mut connection = self.pool().acquire().await?;
        get_request_in_connection(&mut connection, id).await
    }

    /// Record a status transition. Repeating the current status leaves no new event.
    pub async fn set_local_request_status(
        &self,
        id: i64,
        status: &str,
    ) -> Result<bool, LocalError> {
        validate_request_id(id)?;
        validate_status(status)?;
        let mut connection = self.pool().acquire().await?;
        let mut transaction = connection.begin_with("BEGIN IMMEDIATE").await?;
        let result = set_status_in_connection(&mut transaction, id, status).await;
        finish_transaction(transaction, result).await
    }

    /// Read the record and its local history in the same snapshot.
    pub async fn local_request_timeline(&self, id: i64) -> Result<Vec<TimelineItem>, LocalError> {
        validate_request_id(id)?;
        let mut transaction = self.pool().begin().await?;
        let result = async {
            let request = get_request_in_connection(&mut transaction, id)
                .await?
                .ok_or(LocalError::InvalidInput("local request not found"))?;
            timeline_in_connection(&mut transaction, &request).await
        }
        .await;
        finish_transaction(transaction, result).await
    }

    /// Read complete local detail without claiming derived reports or remote evidence.
    pub async fn local_request_detail(&self, id: i64) -> Result<LocalRequestDetail, LocalError> {
        validate_request_id(id)?;
        let mut transaction = self.pool().begin().await?;
        let result = async {
            let request = get_request_in_connection(&mut transaction, id)
                .await?
                .ok_or(LocalError::InvalidInput("local request not found"))?;
            let timeline = timeline_in_connection(&mut transaction, &request).await?;
            Ok(LocalRequestDetail { request, timeline })
        }
        .await;
        finish_transaction(transaction, result).await
    }
}

async fn insert_request_in_connection(
    connection: &mut SqliteConnection,
    request: &NewLocalRequest,
) -> Result<i64, LocalError> {
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM authorities WHERE slug = ?)")
        .bind(&request.authority_slug)
        .fetch_one(&mut *connection)
        .await?;
    if !exists {
        return Err(LocalError::InvalidInput("local authority not found"));
    }
    let timestamp = now_timestamp();
    let result = sqlx::query(
        "INSERT INTO tracked_requests (
            authority_slug, title, body, tags, status, fyi_request_id, created_at, updated_at
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&request.authority_slug)
    .bind(&request.title)
    .bind(&request.body)
    .bind(&request.tags)
    .bind(&request.status)
    .bind(request.fyi_request_id)
    .bind(&timestamp)
    .bind(&timestamp)
    .execute(&mut *connection)
    .await?;
    let id = result.last_insert_rowid();
    sqlx::query(
        "INSERT INTO local_request_events (
            tracked_request_id, kind, old_status, new_status, occurred_at
         ) VALUES (?, 'created', NULL, ?, ?)",
    )
    .bind(id)
    .bind(&request.status)
    .bind(&timestamp)
    .execute(&mut *connection)
    .await?;
    Ok(id)
}

async fn set_status_in_connection(
    connection: &mut SqliteConnection,
    id: i64,
    status: &str,
) -> Result<bool, LocalError> {
    let previous: Option<Option<String>> =
        sqlx::query_scalar("SELECT status FROM tracked_requests WHERE id = ?")
            .bind(id)
            .fetch_optional(&mut *connection)
            .await?;
    let previous = previous.ok_or(LocalError::InvalidInput("local request not found"))?;
    if previous.as_deref() == Some(status) {
        return Ok(false);
    }
    let timestamp = now_timestamp();
    sqlx::query("UPDATE tracked_requests SET status = ?, updated_at = ? WHERE id = ?")
        .bind(status)
        .bind(&timestamp)
        .bind(id)
        .execute(&mut *connection)
        .await?;
    sqlx::query(
        "INSERT INTO local_request_events (
            tracked_request_id, kind, old_status, new_status, occurred_at
         ) VALUES (?, 'status_changed', ?, ?, ?)",
    )
    .bind(id)
    .bind(previous)
    .bind(status)
    .bind(&timestamp)
    .execute(&mut *connection)
    .await?;
    Ok(true)
}

pub(crate) async fn get_request_in_connection(
    connection: &mut SqliteConnection,
    id: i64,
) -> Result<Option<TrackedRequest>, LocalError> {
    Ok(sqlx::query_as(
        "SELECT id, authority_slug, title, body, tags, fyi_url, fyi_request_id,
                status, created_at, updated_at, last_seen_at, last_event_title
         FROM tracked_requests WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(connection)
    .await?)
}

pub(crate) async fn timeline_in_connection(
    connection: &mut SqliteConnection,
    request: &TrackedRequest,
) -> Result<Vec<TimelineItem>, LocalError> {
    let history: Vec<HistoryProjection> = sqlx::query_as(
        "SELECT kind, new_status, occurred_at FROM local_request_events
         WHERE tracked_request_id = ? ORDER BY id DESC",
    )
    .bind(request.id)
    .fetch_all(connection)
    .await?;
    let mut items = Vec::with_capacity(history.len() + 2);
    for event in &history {
        let (title, detail) = match event.kind.as_str() {
            "created" => ("Tracked request created", request.title.clone()),
            "status_changed" => (
                "Tracked request updated",
                format!("Status: {}", event.new_status),
            ),
            _ => return Err(LocalError::Schema),
        };
        items.push(TimelineItem {
            ts: Some(event.occurred_at.clone()),
            kind: "tracked_request".to_owned(),
            title: title.to_owned(),
            detail,
        });
    }
    // An imported legacy row has no invented native creation event. Preserve its
    // known creation timestamp even after a later native status change is recorded.
    if !history.iter().any(|event| event.kind == "created") {
        if let Some(timestamp) = request.created_at.as_ref().filter(|value| !value.is_empty()) {
            items.push(TimelineItem {
                ts: Some(timestamp.clone()),
                kind: "tracked_request".to_owned(),
                title: "Tracked request created".to_owned(),
                detail: request.title.clone(),
            });
        }
    }
    if history.is_empty() {
        if let Some(timestamp) = request.updated_at.as_ref().filter(|value| {
            !value.is_empty() && Some(value.as_str()) != request.created_at.as_deref()
        }) {
            items.push(TimelineItem {
                ts: Some(timestamp.clone()),
                kind: "tracked_request".to_owned(),
                title: "Tracked request updated".to_owned(),
                detail: format!("Status: {}", request.status.as_deref().unwrap_or("None")),
            });
        }
    }
    sort_timeline_items(&mut items);
    Ok(items)
}

/// Stable sorting preserves the caller's deterministic source-ID order for ties.
/// Native events are supplied in descending event-ID order. Original timestamp
/// text is retained; valid times sort before unparseable or missing legacy times.
pub(crate) fn sort_timeline_items(items: &mut [TimelineItem]) {
    items.sort_by(|left, right| match (
        parsed_timestamp(left.ts.as_deref()),
        parsed_timestamp(right.ts.as_deref()),
    ) {
        (Some(left), Some(right)) => right.cmp(&left),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => right.ts.cmp(&left.ts),
    });
}

fn parsed_timestamp(value: Option<&str>) -> Option<DateTime<Utc>> {
    let value = value?;
    if let Ok(timestamp) = DateTime::parse_from_rfc3339(value) {
        return Some(timestamp.with_timezone(&Utc));
    }
    NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S%.f")
        .or_else(|_| NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S%.f"))
        .ok()
        .map(|timestamp| timestamp.and_utc())
}
