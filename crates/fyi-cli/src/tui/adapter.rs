//! One-time local snapshot acquisition, outside rendering and input handling.
use super::contracts::{
    PresentationError, PresentationSnapshot, RequestItem, RequestStatus, SnapshotScope,
    TimelineState, MAX_REQUESTS,
};
use fyi_core::db::DbPool;
use std::path::Path;

/// Open only existing, validated local data read-only, then close it before display.
/// No path, SQL, raw status, body, URL or backend diagnostic is returned on error.
pub async fn load_snapshot(path: &Path) -> Result<PresentationSnapshot, PresentationError> {
    let db = DbPool::open_local_existing(path, true)
        .await
        .map_err(|_| PresentationError::Unavailable)?;
    let result = db.local_request_overview(MAX_REQUESTS).await;
    db.pool().close().await;
    let overview = result.map_err(|_| PresentationError::Unavailable)?;
    let scope = if overview.requests.len() == overview.total {
        SnapshotScope::Complete
    } else {
        SnapshotScope::Partial {
            total: Some(overview.total),
        }
    };
    PresentationSnapshot {
        requests: overview
            .requests
            .into_iter()
            .map(|row| RequestItem {
                id: row.request.id,
                title: row.request.title,
                authority: row.authority_name,
                status: RequestStatus::from_backend(row.request.status.as_deref()),
                needs_attention: None,
                updated_at: row.updated_at,
                timeline: TimelineState::Unavailable,
            })
            .collect(),
        scope,
    }
    .validate()
}

#[cfg(test)]
#[path = "adapter_tests.rs"]
mod tests;
