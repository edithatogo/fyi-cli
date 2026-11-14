//! Read-only projections of locally retained provenance, with no source acquisition.

use super::requests::{TimelineItem, TrackedRequest};
use super::{required_nullable, LocalError};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, SqliteConnection};

/// The complete Python feed-event row. Raw payloads remain separate from projections.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FeedEvent {
    pub id: i64,
    pub feed_url: String,
    #[serde(deserialize_with = "required_nullable")]
    pub event_id: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub title: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub link: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub published: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub summary: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub request_id_guess: Option<i64>,
    #[serde(deserialize_with = "required_nullable")]
    pub tracked_request_id: Option<i64>,
    #[serde(deserialize_with = "required_nullable")]
    pub raw_json: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub seen_at: Option<String>,
}

/// A locally stored snapshot associated with the external request identifier.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RequestSnapshot {
    pub id: i64,
    pub fyi_request_id: i64,
    pub source_url: String,
    pub raw_json: String,
    #[serde(deserialize_with = "required_nullable")]
    pub fetched_at: Option<String>,
}

#[derive(FromRow)]
struct FeedTimelineRow {
    title: Option<String>,
    link: Option<String>,
    published: Option<String>,
    summary: Option<String>,
    seen_at: Option<String>,
}

#[derive(FromRow)]
struct SnapshotTimelineRow {
    raw_json: String,
    fetched_at: Option<String>,
}

/// Append projections using the caller's read transaction. Link feed rows only by
/// explicit local identity, and snapshots only by the stored external identity.
/// Descending IDs provide deterministic ties within each source group; the caller
/// performs the final stable chronological sort across all timeline items.
pub(crate) async fn append_timeline_in_connection(
    connection: &mut SqliteConnection,
    request: &TrackedRequest,
    items: &mut Vec<TimelineItem>,
) -> Result<(), LocalError> {
    let feeds: Vec<FeedTimelineRow> = sqlx::query_as(
        "SELECT title, link, published, summary, seen_at
         FROM feed_events WHERE tracked_request_id = ? ORDER BY id DESC",
    )
    .bind(request.id)
    .fetch_all(&mut *connection)
    .await?;
    for feed in feeds {
        items.push(TimelineItem {
            ts: nonempty(feed.published).or(feed.seen_at),
            kind: "feed_event".to_owned(),
            title: nonempty(feed.title).unwrap_or_else(|| "Feed event".to_owned()),
            detail: nonempty(feed.summary).or(feed.link).unwrap_or_default(),
        });
    }

    if let Some(remote_id) = request.fyi_request_id {
        let snapshots: Vec<SnapshotTimelineRow> = sqlx::query_as(
            "SELECT raw_json, fetched_at
             FROM request_snapshots WHERE fyi_request_id = ? ORDER BY id DESC",
        )
        .bind(remote_id)
        .fetch_all(&mut *connection)
        .await?;
        for snapshot in snapshots {
            let payload = serde_json::from_str::<Value>(&snapshot.raw_json).ok();
            let title = snapshot_field(payload.as_ref(), "title")
                .unwrap_or("Request snapshot")
                .to_owned();
            let detail = snapshot_field(payload.as_ref(), "described_state")
                .unwrap_or_default()
                .to_owned();
            items.push(TimelineItem {
                ts: snapshot.fetched_at,
                kind: "request_snapshot".to_owned(),
                title,
                detail,
            });
        }
    }
    Ok(())
}

fn nonempty(value: Option<String>) -> Option<String> {
    value.filter(|text| !text.is_empty())
}

/// Malformed JSON, non-object documents and non-string fields have safe defaults.
/// Never stringify an arbitrary value: it could contain the full private payload.
fn snapshot_field<'a>(payload: Option<&'a Value>, field: &str) -> Option<&'a str> {
    let object = payload?.as_object()?;
    let text = |value: &'a Value| value.as_str().filter(|text| !text.is_empty());
    object.get(field).and_then(text).or_else(|| {
        object
            .get("info_request")?
            .as_object()?
            .get(field)
            .and_then(text)
    })
}
