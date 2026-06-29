use crate::api::AlaveteliRequest;
use crate::db::DbPool;
use anyhow::{anyhow, Context, Result};
use reqwest::{Client, Url};
use serde_json::Value;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PullReport {
    pub fetched: usize,
    pub applied: usize,
    pub source: String,
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
}
