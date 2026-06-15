use crate::api::{AlaveteliCorrespondence, AlaveteliRequest, CorrespondenceDirection};
use sqlx::{sqlite::SqlitePool, sqlite::SqlitePoolOptions, Row};
use std::time::Duration;

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

    /// Inserts or replaces an `AlaveteliRequest` in the database.
    pub async fn insert_request(&self, request: &AlaveteliRequest) -> Result<(), sqlx::Error> {
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
             VALUES (?, ?, ?, ?, ?, ?)"
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
}
