use crate::api::{AlaveteliRequest, CreateRequestResponse};
use crate::db::DbPool;
use anyhow::{anyhow, Context, Result};
use reqwest::{Client, Url};
use serde_json::Value;
use std::collections::BTreeSet;
use std::time::Duration;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PullReport {
    pub fetched: usize,
    pub applied: usize,
    pub source: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PushReport {
    pub queued: usize,
    pub submitted: usize,
    pub failed: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncConfig {
    pub pull_interval: Duration,
    pub push_max_retries: u32,
    pub push_initial_backoff: Duration,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            pull_interval: Duration::from_secs(300),
            push_max_retries: 3,
            push_initial_backoff: Duration::from_secs(1),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SyncClient {
    base_url: Url,
    http: Client,
}

impl SyncClient {
    pub fn new(base_url: &str) -> Result<Self> {
        Self::with_http_client(base_url, Client::new())
    }

    pub fn with_http_client(base_url: &str, http: Client) -> Result<Self> {
        let base_url = Url::parse(base_url).context("invalid FYI base URL")?;
        Ok(Self { base_url, http })
    }

    pub async fn pull_incremental(&self, db: &DbPool) -> Result<PullReport> {
        let since = db.get_latest_sync_timestamp().await?;
        let requests = self.fetch_updates_since(since.as_deref()).await?;
        let applied = apply_remote_requests(db, &requests).await?;

        Ok(PullReport {
            fetched: requests.len(),
            applied,
            source: "api".to_string(),
        })
    }

    pub async fn pull_feed(&self, db: &DbPool, feed_url: &str) -> Result<PullReport> {
        let feed = self
            .http
            .get(feed_url)
            .send()
            .await
            .context("failed to fetch watched request feed")?
            .error_for_status()
            .context("watched request feed returned an error")?
            .text()
            .await
            .context("failed to read watched request feed")?;

        let request_ids = request_ids_from_feed(&feed);
        let mut requests = Vec::with_capacity(request_ids.len());
        for request_id in request_ids {
            requests.push(self.fetch_request(request_id).await?);
        }
        let applied = apply_remote_requests(db, &requests).await?;

        Ok(PullReport {
            fetched: requests.len(),
            applied,
            source: "feed".to_string(),
        })
    }

    pub async fn fetch_updates_since(&self, since: Option<&str>) -> Result<Vec<AlaveteliRequest>> {
        let mut url = self
            .base_url
            .join("api/v2/request.json")
            .context("failed to build request list URL")?;
        if let Some(since) = since.filter(|value| !value.trim().is_empty()) {
            url.query_pairs_mut().append_pair("updated_since", since);
        }

        let value = self
            .http
            .get(url)
            .send()
            .await
            .context("failed to fetch updated requests")?
            .error_for_status()
            .context("updated requests endpoint returned an error")?
            .json::<Value>()
            .await
            .context("failed to parse updated requests JSON")?;

        parse_request_list(value)
    }

    pub async fn fetch_request(&self, request_id: i64) -> Result<AlaveteliRequest> {
        let url = self
            .base_url
            .join(&format!("api/v2/request/{request_id}.json"))
            .context("failed to build request URL")?;

        self.http
            .get(url)
            .send()
            .await
            .context("failed to fetch request")?
            .error_for_status()
            .context("request endpoint returned an error")?
            .json::<AlaveteliRequest>()
            .await
            .context("failed to parse request JSON")
    }

    pub async fn push_dirty(&self, db: &DbPool) -> Result<PushReport> {
        self.push_dirty_with_config(db, &SyncConfig::default())
            .await
    }

    pub async fn push_dirty_with_config(
        &self,
        db: &DbPool,
        config: &SyncConfig,
    ) -> Result<PushReport> {
        let dirty_requests = db.list_dirty_requests(500).await?;
        let mut submitted = 0;
        let mut failed = 0;

        for request in &dirty_requests {
            let queue_id = db.enqueue_request_submission(request).await?;
            match self.submit_request_with_retries(request, config).await {
                Ok(response) => {
                    db.mark_submission_confirmed(
                        queue_id,
                        request.id,
                        response.id,
                        request.updated_at.as_deref(),
                    )
                    .await?;
                    submitted += 1;
                }
                Err(error) => {
                    let attempts = i64::from(config.push_max_retries.max(1));
                    db.mark_submission_failed(queue_id, request.id, attempts, &error.to_string())
                        .await?;
                    failed += 1;
                }
            }
        }

        Ok(PushReport {
            queued: dirty_requests.len(),
            submitted,
            failed,
        })
    }

    async fn submit_request(&self, request: &AlaveteliRequest) -> Result<CreateRequestResponse> {
        let url = self
            .base_url
            .join("api/v2/request")
            .context("failed to build request submission URL")?;

        self.http
            .post(url)
            .json(request)
            .send()
            .await
            .context("failed to submit request")?
            .error_for_status()
            .context("request submission endpoint returned an error")?
            .json::<CreateRequestResponse>()
            .await
            .context("failed to parse request submission response")
    }

    async fn submit_request_with_retries(
        &self,
        request: &AlaveteliRequest,
        config: &SyncConfig,
    ) -> Result<CreateRequestResponse> {
        let max_attempts = config.push_max_retries.max(1);
        let mut last_error = None;

        for attempt in 1..=max_attempts {
            match self.submit_request(request).await {
                Ok(response) => return Ok(response),
                Err(error) => {
                    last_error = Some(error);
                    if attempt < max_attempts {
                        let delay = config
                            .push_initial_backoff
                            .saturating_mul(1 << (attempt - 1));
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow!("request submission failed")))
    }
}

pub fn spawn_pull_scheduler(
    db: DbPool,
    client: SyncClient,
    config: SyncConfig,
    mut shutdown: oneshot::Receiver<()>,
) -> JoinHandle<Result<Vec<PullReport>>> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(config.pull_interval);
        let mut reports = Vec::new();

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    reports.push(client.pull_incremental(&db).await?);
                }
                _ = &mut shutdown => {
                    return Ok(reports);
                }
            }
        }
    })
}

async fn apply_remote_requests(db: &DbPool, requests: &[AlaveteliRequest]) -> Result<usize> {
    for request in requests {
        db.upsert_synced_request(request, request.updated_at.as_deref())
            .await?;
    }
    Ok(requests.len())
}

fn parse_request_list(value: Value) -> Result<Vec<AlaveteliRequest>> {
    if value.is_array() {
        return serde_json::from_value(value).context("invalid request array");
    }

    if let Some(requests) = value.get("requests").cloned() {
        return serde_json::from_value(requests).context("invalid requests field");
    }

    Err(anyhow!(
        "updated requests response must be an array or requests object"
    ))
}

fn request_ids_from_feed(feed: &str) -> Vec<i64> {
    let mut ids = BTreeSet::new();
    for marker in ["/request/", "/requests/"] {
        let mut remaining = feed;
        while let Some(index) = remaining.find(marker) {
            let after_marker = &remaining[index + marker.len()..];
            let digits = after_marker
                .chars()
                .take_while(|ch| ch.is_ascii_digit())
                .collect::<String>();
            if let Ok(id) = digits.parse::<i64>() {
                ids.insert(id);
            }
            remaining = after_marker;
        }
    }
    ids.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn request(id: i64, title: &str, updated_at: &str) -> AlaveteliRequest {
        AlaveteliRequest {
            id,
            title: title.to_string(),
            body: "Body".to_string(),
            user_name: Some("FYI user".to_string()),
            status: Some("waiting_response".to_string()),
            created_at: Some("2026-06-30T00:00:00Z".to_string()),
            updated_at: Some(updated_at.to_string()),
            url: Some(format!("https://fyi.org.nz/request/{id}")),
            tags: None,
        }
    }

    #[tokio::test]
    async fn pull_incremental_fetches_and_applies_remote_updates() {
        let server = MockServer::start().await;
        let db = DbPool::new_in_memory().await.unwrap();
        db.run_migrations().await.unwrap();
        let existing = request(1, "Existing request", "2026-06-30T00:00:00Z");
        db.upsert_synced_request(&existing, existing.updated_at.as_deref())
            .await
            .unwrap();

        Mock::given(method("GET"))
            .and(path("/api/v2/request.json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "requests": [request(2, "Remote update", "2026-06-30T01:00:00Z")]
            })))
            .mount(&server)
            .await;

        let client = SyncClient::new(&server.uri()).unwrap();
        let report = client.pull_incremental(&db).await.unwrap();
        let saved = db.get_request(2).await.unwrap().unwrap();
        let metadata = db.get_request_sync_metadata(2).await.unwrap().unwrap();

        assert_eq!(report.fetched, 1);
        assert_eq!(report.applied, 1);
        assert_eq!(saved.title, "Remote update");
        assert_eq!(metadata.sync_status.as_str(), "clean");
    }

    #[tokio::test]
    async fn pull_feed_fetches_watched_request_records() {
        let server = MockServer::start().await;
        let db = DbPool::new_in_memory().await.unwrap();
        db.run_migrations().await.unwrap();

        Mock::given(method("GET"))
            .and(path("/feed.atom"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                r#"<feed><entry><link href="https://fyi.org.nz/request/33/example"/></entry></feed>"#,
            ))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/v2/request/33.json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(request(
                33,
                "Feed request",
                "2026-06-30T02:00:00Z",
            )))
            .mount(&server)
            .await;

        let client = SyncClient::new(&server.uri()).unwrap();
        let report = client
            .pull_feed(&db, &format!("{}/feed.atom", server.uri()))
            .await
            .unwrap();
        let saved = db.get_request(33).await.unwrap().unwrap();

        assert_eq!(report.source, "feed");
        assert_eq!(report.fetched, 1);
        assert_eq!(saved.title, "Feed request");
    }

    #[test]
    fn extracts_request_ids_from_rss_and_atom_links() {
        let ids = request_ids_from_feed(
            r#"
            <rss><channel><item><link>https://fyi.org.nz/request/4/foo</link></item></channel></rss>
            <feed><entry><id>https://fyi.org.nz/requests/9</id></entry></feed>
            <a href="/request/4/foo">duplicate</a>
            "#,
        );

        assert_eq!(ids, vec![4, 9]);
    }

    #[tokio::test]
    async fn pull_scheduler_runs_until_shutdown() {
        let server = MockServer::start().await;
        let db = DbPool::new_in_memory().await.unwrap();
        db.run_migrations().await.unwrap();

        Mock::given(method("GET"))
            .and(path("/api/v2/request.json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "requests": [request(51, "Scheduled request", "2026-06-30T03:00:00Z")]
            })))
            .mount(&server)
            .await;

        let client = SyncClient::new(&server.uri()).unwrap();
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let handle = spawn_pull_scheduler(
            db.clone(),
            client,
            SyncConfig {
                pull_interval: Duration::from_millis(10),
                ..SyncConfig::default()
            },
            shutdown_rx,
        );

        tokio::time::sleep(Duration::from_millis(25)).await;
        let _ = shutdown_tx.send(());
        let reports = handle.await.unwrap().unwrap();
        let saved = db.get_request(51).await.unwrap().unwrap();

        assert!(!reports.is_empty());
        assert_eq!(saved.title, "Scheduled request");
    }

    #[tokio::test]
    async fn push_dirty_enqueues_submits_and_records_remote_id() {
        let server = MockServer::start().await;
        let db = DbPool::new_in_memory().await.unwrap();
        db.run_migrations().await.unwrap();
        let local = request(-1, "Local draft", "2026-06-30T04:00:00Z");
        db.insert_request(&local).await.unwrap();

        Mock::given(method("POST"))
            .and(path("/api/v2/request"))
            .respond_with(
                ResponseTemplate::new(201).set_body_json(CreateRequestResponse {
                    id: 4100,
                    url: "https://fyi.org.nz/request/4100".to_string(),
                }),
            )
            .mount(&server)
            .await;

        let client = SyncClient::new(&server.uri()).unwrap();
        let report = client.push_dirty(&db).await.unwrap();
        let metadata = db.get_request_sync_metadata(-1).await.unwrap().unwrap();
        let pending = db.list_pending_outgoing_queue(10).await.unwrap();

        assert_eq!(report.queued, 1);
        assert_eq!(report.submitted, 1);
        assert_eq!(report.failed, 0);
        assert_eq!(metadata.remote_request_id, Some(4100));
        assert_eq!(metadata.sync_status.as_str(), "clean");
        assert!(pending.is_empty());
    }

    #[tokio::test]
    async fn push_dirty_retries_and_marks_failed_after_exhaustion() {
        let server = MockServer::start().await;
        let db = DbPool::new_in_memory().await.unwrap();
        db.run_migrations().await.unwrap();
        let local = request(-2, "Retry draft", "2026-06-30T05:00:00Z");
        db.insert_request(&local).await.unwrap();

        Mock::given(method("POST"))
            .and(path("/api/v2/request"))
            .respond_with(ResponseTemplate::new(503).set_body_string("try later"))
            .mount(&server)
            .await;

        let client = SyncClient::new(&server.uri()).unwrap();
        let report = client
            .push_dirty_with_config(
                &db,
                &SyncConfig {
                    push_max_retries: 2,
                    push_initial_backoff: Duration::from_millis(1),
                    ..SyncConfig::default()
                },
            )
            .await
            .unwrap();
        let depth = db.get_outgoing_queue_depth().await.unwrap();
        let metadata = db.get_request_sync_metadata(-2).await.unwrap().unwrap();

        assert_eq!(report.queued, 1);
        assert_eq!(report.submitted, 0);
        assert_eq!(report.failed, 1);
        assert_eq!(depth.failed, 1);
        assert_eq!(metadata.sync_status.as_str(), "dirty");
    }
}
