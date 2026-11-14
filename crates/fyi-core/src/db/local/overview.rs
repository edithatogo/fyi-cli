//! Bounded read-only metadata for local views; no bodies, URLs or history are read.
use super::requests::LocalRequestSummary;
use super::LocalError;
use crate::db::DbPool;

/// A view may request fewer rows, but cannot allocate an unbounded catalogue.
pub const MAX_LOCAL_OVERVIEW_REQUESTS: usize = 500;

/// Reuse the local summary identity/status contract and join actual authority metadata.
/// Text is bounded to 513 Unicode scalars in SQL, allowing display truncation detection.
#[derive(sqlx::FromRow)]
pub struct LocalOverviewRow {
    #[sqlx(flatten)]
    pub request: LocalRequestSummary,
    pub authority_name: Option<String>,
    pub updated_at: Option<String>,
}

/// Rows and the exact total come from one SQLite read transaction.
pub struct LocalRequestOverview {
    pub requests: Vec<LocalOverviewRow>,
    pub total: usize,
}

impl DbPool {
    /// Read the first `limit` local identities in ascending order without mutations.
    ///
    /// Callers opening files must use `open_local_existing(path, true)`, which
    /// validates the accepted local schema without initialization or migration.
    /// The projection deliberately excludes timelines: legacy timestamps alone do
    /// not establish events. No remote request/user or correspondence table is read.
    pub async fn local_request_overview(
        &self,
        limit: usize,
    ) -> Result<LocalRequestOverview, LocalError> {
        if !(1..=MAX_LOCAL_OVERVIEW_REQUESTS).contains(&limit) {
            return Err(LocalError::InvalidInput("local overview limit must be 1 to 500"));
        }
        let mut transaction = self.pool().begin().await?;
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tracked_requests")
            .fetch_one(&mut *transaction)
            .await?;
        let requests = sqlx::query_as(
            "SELECT r.id, substr(r.authority_slug, 1, 513) AS authority_slug,
                    substr(r.title, 1, 513) AS title,
                    substr(r.status, 1, 129) AS status, r.fyi_request_id,
                    substr(a.name, 1, 513) AS authority_name,
                    substr(r.updated_at, 1, 513) AS updated_at
             FROM tracked_requests r LEFT JOIN authorities a ON a.slug = r.authority_slug
             ORDER BY r.id LIMIT ?",
        )
        .bind(limit as i64)
        .fetch_all(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(LocalRequestOverview {
            requests,
            total: usize::try_from(total).map_err(|_| LocalError::Schema)?,
        })
    }
}
