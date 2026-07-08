use crate::api::{
    AddCorrespondencePayload, AlaveteliRequest, Authority, CorrespondenceResponse,
    CreateRequestPayload, CreateRequestResponse, UpdateRequestStatePayload,
    UpdateRequestStateResponse,
};
use crate::db::{DbPool, FieldChange, SyncStatus};
use anyhow::{anyhow, Context, Result};
use reqwest::{header, Client, Response, StatusCode, Url};
use serde_json::Value;
use std::collections::BTreeSet;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SyncHealth {
    pub network_reachable: bool,
    pub api_reachable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncRunReport {
    pub health: SyncHealth,
    pub pull: Option<PullReport>,
    pub push: Option<PushReport>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeOutcome {
    pub request: AlaveteliRequest,
    pub conflicting_fields: Vec<String>,
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
        validate_instance_url(&base_url)?;
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
        let response = self
            .http
            .get(feed_url)
            .send()
            .await
            .context("failed to fetch watched request feed")?;
        let feed = ensure_success(response, "watched request feed")
            .await?
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

        let response = self
            .http
            .get(url)
            .send()
            .await
            .context("failed to fetch updated requests")?;
        let value = ensure_success(response, "updated requests endpoint")
            .await?
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

        let response = self
            .http
            .get(url)
            .send()
            .await
            .context("failed to fetch request")?;

        ensure_success(response, "request endpoint")
            .await?
            .json::<AlaveteliRequest>()
            .await
            .context("failed to parse request JSON")
    }

    pub async fn search_requests(&self, query: &str) -> Result<Vec<AlaveteliRequest>> {
        let mut url = self
            .base_url
            .join("search.json")
            .context("failed to build search URL")?;
        if !query.trim().is_empty() {
            url.query_pairs_mut().append_pair("q", query);
        }

        let response = self
            .http
            .get(url)
            .send()
            .await
            .context("failed to search requests")?;
        let value = ensure_success(response, "search endpoint")
            .await?
            .json::<Value>()
            .await
            .context("failed to parse search results")?;

        parse_search_response(value)
    }

    pub async fn create_request(
        &self,
        payload: &CreateRequestPayload,
    ) -> Result<CreateRequestResponse> {
        let url = self
            .base_url
            .join("api/v2/request")
            .context("failed to build request submission URL")?;

        let response = self
            .http
            .post(url)
            .json(payload)
            .send()
            .await
            .context("failed to submit request")?;

        ensure_success(response, "request submission endpoint")
            .await?
            .json::<CreateRequestResponse>()
            .await
            .context("failed to parse request submission response")
    }

    pub async fn add_correspondence(
        &self,
        request_id: i64,
        payload: &AddCorrespondencePayload,
    ) -> Result<CorrespondenceResponse> {
        let url = self
            .base_url
            .join(&format!("api/v2/request/{request_id}/correspondence.json"))
            .context("failed to build correspondence URL")?;

        let response = self
            .http
            .post(url)
            .json(payload)
            .send()
            .await
            .context("failed to add correspondence")?;

        ensure_success(response, "correspondence endpoint")
            .await?
            .json::<CorrespondenceResponse>()
            .await
            .context("failed to parse correspondence response")
    }

    pub async fn update_request_state(
        &self,
        request_id: i64,
        payload: &UpdateRequestStatePayload,
    ) -> Result<UpdateRequestStateResponse> {
        let url = self
            .base_url
            .join(&format!("api/v2/request/{request_id}/state.json"))
            .context("failed to build request state URL")?;

        let response = self
            .http
            .put(url)
            .json(payload)
            .send()
            .await
            .context("failed to update request state")?;

        ensure_success(response, "request state endpoint")
            .await?
            .json::<UpdateRequestStateResponse>()
            .await
            .context("failed to parse request state response")
    }

    pub async fn list_authorities(&self) -> Result<Vec<Authority>> {
        let url = self
            .base_url
            .join("api/v2/authority.json")
            .context("failed to build authorities URL")?;

        let response = self
            .http
            .get(url)
            .send()
            .await
            .context("failed to list authorities")?;
        let value = ensure_success(response, "authorities endpoint")
            .await?
            .json::<Value>()
            .await
            .context("failed to parse authorities response")?;

        parse_authorities(value)
    }

    pub async fn get_api_version(&self) -> Result<String> {
        let url = self
            .base_url
            .join("api/v2/version.json")
            .context("failed to build API version URL")?;

        let response = self
            .http
            .get(url)
            .send()
            .await
            .context("failed to get API version")?;
        let value = ensure_success(response, "version endpoint")
            .await?
            .json::<Value>()
            .await
            .context("failed to parse API version response")?;

        match value {
            Value::String(version) => Ok(version),
            Value::Object(map) => map
                .get("version")
                .and_then(Value::as_str)
                .map(str::to_owned)
                .ok_or_else(|| anyhow!("version endpoint did not include a version string")),
            _ => Err(anyhow!("version endpoint returned an unsupported payload")),
        }
    }

    pub async fn build_prefilled_url(
        &self,
        authority_slug: &str,
        title: &str,
        body: &str,
        tags: Option<&str>,
    ) -> Result<Url> {
        let mut url = self
            .base_url
            .join(&format!("new/{authority_slug}"))
            .context("failed to build prefilled URL")?;
        url.query_pairs_mut().append_pair("title", title);
        url.query_pairs_mut().append_pair("body", body);
        if let Some(tags) = tags.filter(|value| !value.trim().is_empty()) {
            url.query_pairs_mut().append_pair("tags", tags);
        }
        Ok(url)
    }

    pub async fn health_check(&self) -> SyncHealth {
        let url = match self.base_url.join("api/v2/request.json") {
            Ok(url) => url,
            Err(_) => {
                return SyncHealth {
                    network_reachable: false,
                    api_reachable: false,
                }
            }
        };

        match self.http.get(url).send().await {
            Ok(response) => SyncHealth {
                network_reachable: true,
                api_reachable: response.status().is_success(),
            },
            Err(_) => SyncHealth {
                network_reachable: false,
                api_reachable: false,
            },
        }
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

        let response = self
            .http
            .post(url)
            .json(request)
            .send()
            .await
            .context("failed to submit request")?;

        ensure_success(response, "request submission endpoint")
            .await?
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

pub fn spawn_sync_scheduler(
    db: DbPool,
    client: SyncClient,
    config: SyncConfig,
    mut shutdown: oneshot::Receiver<()>,
) -> JoinHandle<Result<Vec<SyncRunReport>>> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(config.pull_interval);
        let mut reports = Vec::new();

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    let health = client.health_check().await;
                    if health.api_reachable {
                        let pull = client.pull_incremental(&db).await?;
                        let push = client.push_dirty_with_config(&db, &config).await?;
                        reports.push(SyncRunReport {
                            health,
                            pull: Some(pull),
                            push: Some(push),
                        });
                    } else {
                        reports.push(SyncRunReport {
                            health,
                            pull: None,
                            push: None,
                        });
                    }
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
        apply_remote_request(db, request).await?;
    }
    Ok(requests.len())
}

async fn apply_remote_request(db: &DbPool, remote: &AlaveteliRequest) -> Result<()> {
    let Some(metadata) = db.get_request_sync_metadata(remote.id).await? else {
        db.upsert_synced_request(remote, remote.updated_at.as_deref())
            .await?;
        return Ok(());
    };

    if metadata.sync_status != SyncStatus::Dirty && metadata.sync_status != SyncStatus::Pending {
        db.upsert_synced_request(remote, remote.updated_at.as_deref())
            .await?;
        return Ok(());
    }

    let Some(local) = db.get_request(remote.id).await? else {
        db.upsert_synced_request(remote, remote.updated_at.as_deref())
            .await?;
        return Ok(());
    };
    let changes = db.list_unsynced_field_changes(remote.id).await?;
    let base = reconstruct_base_request(&local, &changes);
    let outcome = three_way_merge_request(&base, &local, remote);

    if outcome.conflicting_fields.is_empty() {
        db.apply_remote_merge(&outcome.request, remote.updated_at.as_deref())
            .await?;
    } else {
        db.mark_request_conflict(remote.id).await?;
    }

    Ok(())
}

pub fn last_write_wins<'a>(
    local: &'a AlaveteliRequest,
    remote: &'a AlaveteliRequest,
) -> &'a AlaveteliRequest {
    let local_updated = local
        .updated_at
        .as_deref()
        .or(local.created_at.as_deref())
        .unwrap_or_default();
    let remote_updated = remote
        .updated_at
        .as_deref()
        .or(remote.created_at.as_deref())
        .unwrap_or_default();

    if remote_updated > local_updated {
        remote
    } else {
        local
    }
}

pub fn three_way_merge_request(
    base: &AlaveteliRequest,
    local: &AlaveteliRequest,
    remote: &AlaveteliRequest,
) -> MergeOutcome {
    let mut merged = local.clone();
    let mut conflicts = Vec::new();

    merge_field(
        "title",
        &base.title,
        &local.title,
        &remote.title,
        &mut merged.title,
        &mut conflicts,
    );
    merge_field(
        "body",
        &base.body,
        &local.body,
        &remote.body,
        &mut merged.body,
        &mut conflicts,
    );
    merge_option_field(
        "user_name",
        &base.user_name,
        &local.user_name,
        &remote.user_name,
        &mut merged.user_name,
        &mut conflicts,
    );
    merge_option_field(
        "status",
        &base.status,
        &local.status,
        &remote.status,
        &mut merged.status,
        &mut conflicts,
    );
    merge_timestamp_field(
        "updated_at",
        &base.updated_at,
        &local.updated_at,
        &remote.updated_at,
        &mut merged.updated_at,
        &mut conflicts,
    );
    merge_option_field(
        "url",
        &base.url,
        &local.url,
        &remote.url,
        &mut merged.url,
        &mut conflicts,
    );
    merge_option_field(
        "tags",
        &base.tags,
        &local.tags,
        &remote.tags,
        &mut merged.tags,
        &mut conflicts,
    );

    MergeOutcome {
        request: merged,
        conflicting_fields: conflicts,
    }
}

fn merge_field<T: Clone + Eq>(
    name: &str,
    base: &T,
    local: &T,
    remote: &T,
    merged: &mut T,
    conflicts: &mut Vec<String>,
) {
    if local == remote || remote == base {
        *merged = local.clone();
    } else if local == base {
        *merged = remote.clone();
    } else {
        conflicts.push(name.to_string());
    }
}

fn merge_option_field<T: Clone + Eq>(
    name: &str,
    base: &Option<T>,
    local: &Option<T>,
    remote: &Option<T>,
    merged: &mut Option<T>,
    conflicts: &mut Vec<String>,
) {
    merge_field(name, base, local, remote, merged, conflicts);
}

fn merge_timestamp_field(
    name: &str,
    base: &Option<String>,
    local: &Option<String>,
    remote: &Option<String>,
    merged: &mut Option<String>,
    conflicts: &mut Vec<String>,
) {
    if local == remote || remote == base {
        *merged = local.clone();
    } else if local == base {
        *merged = remote.clone();
    } else if local > remote {
        *merged = local.clone();
    } else if remote > local {
        *merged = remote.clone();
    } else {
        conflicts.push(name.to_string());
    }
}

fn reconstruct_base_request(local: &AlaveteliRequest, changes: &[FieldChange]) -> AlaveteliRequest {
    let mut base = local.clone();
    for change in changes {
        apply_old_field_value(&mut base, change);
    }
    base
}

fn apply_old_field_value(base: &mut AlaveteliRequest, change: &FieldChange) {
    match change.field_name.as_str() {
        "title" => {
            if let Some(value) = parse_json_value::<String>(&change.old_value) {
                base.title = value;
            }
        }
        "body" => {
            if let Some(value) = parse_json_value::<String>(&change.old_value) {
                base.body = value;
            }
        }
        "user_name" => base.user_name = parse_json_value(&change.old_value),
        "status" => base.status = parse_json_value(&change.old_value),
        "created_at" => base.created_at = parse_json_value(&change.old_value),
        "updated_at" => base.updated_at = parse_json_value(&change.old_value),
        "url" => base.url = parse_json_value(&change.old_value),
        "tags" => base.tags = parse_json_value(&change.old_value),
        _ => {}
    }
}

fn parse_json_value<T: serde::de::DeserializeOwned>(value: &Option<String>) -> Option<T> {
    value
        .as_deref()
        .and_then(|value| serde_json::from_str(value).ok())
}

fn validate_instance_url(url: &Url) -> Result<()> {
    let Some(host) = url.host_str() else {
        return Err(anyhow!("instance URL must include a host"));
    };

    if url.username().is_empty() && url.password().is_none() {
    } else {
        return Err(anyhow!("instance URL must not include credentials"));
    }

    if url.query().is_some_and(|query| !query.is_empty()) || url.fragment().is_some() {
        return Err(anyhow!(
            "instance URL must not contain query strings or fragments"
        ));
    }

    if url.scheme() != "https" && !is_loopback_host(host) {
        return Err(anyhow!("instance URLs must use https for public hosts"));
    }

    if host.eq_ignore_ascii_case("localhost") || host.ends_with(".local") {
        return Err(anyhow!(
            "instance URL must not target localhost or .local hosts"
        ));
    }

    if let Ok(ip) = host.parse::<IpAddr>() {
        if !is_public_ip(ip) {
            return Err(anyhow!(
                "instance URL must not target private or loopback addresses"
            ));
        }
    }

    Ok(())
}

fn is_loopback_host(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost")
        || host.eq_ignore_ascii_case("::1")
        || host == "127.0.0.1"
        || host.starts_with("127.")
}

fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => is_public_ipv4(ip),
        IpAddr::V6(ip) => is_public_ipv6(ip),
    }
}

fn is_public_ipv4(ip: Ipv4Addr) -> bool {
    let [a, b, c, _d] = ip.octets();

    if a == 0 || a == 10 || a == 127 || a == 169 && b == 254 {
        return false;
    }

    if a == 100 && (64..=127).contains(&b) {
        return false;
    }

    if a == 172 && (16..=31).contains(&b) {
        return false;
    }

    if a == 192 && b == 168 {
        return false;
    }

    if a == 198 && (18..=19).contains(&b) {
        return false;
    }

    if a == 198 && b == 51 && c == 100 {
        return false;
    }

    if a == 203 && b == 0 && c == 113 {
        return false;
    }

    if a == 224 || a >= 240 {
        return false;
    }

    true
}

fn is_public_ipv6(ip: Ipv6Addr) -> bool {
    let octets = ip.octets();
    let [a, b, ..] = octets;

    if octets == [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1] {
        return false;
    }

    if octets.iter().all(|octet| *octet == 0) {
        return false;
    }

    if a == 0xff {
        return false;
    }

    if a == 0xfe && (b & 0xC0) == 0x80 {
        return false;
    }

    if a == 0xfc || a == 0xfd {
        return false;
    }

    true
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

fn parse_search_response(value: Value) -> Result<Vec<AlaveteliRequest>> {
    if value.is_array() {
        return serde_json::from_value(value).context("invalid search results array");
    }

    if let Some(requests) = value.get("results").cloned() {
        return serde_json::from_value(requests).context("invalid results field");
    }

    if let Some(requests) = value.get("requests").cloned() {
        return serde_json::from_value(requests).context("invalid requests field");
    }

    Err(anyhow!(
        "search response must be an array or object with results/requests"
    ))
}

fn parse_authorities(value: Value) -> Result<Vec<Authority>> {
    if value.is_array() {
        return serde_json::from_value(value).context("invalid authority array");
    }

    if let Some(authorities) = value.get("authorities").cloned() {
        return serde_json::from_value(authorities).context("invalid authorities field");
    }

    Err(anyhow!(
        "authority response must be an array or object with an authorities field"
    ))
}

async fn ensure_success(response: Response, endpoint: &str) -> Result<Response> {
    if response.status().is_success() {
        return Ok(response);
    }

    let status = response.status();
    let retry_after = response
        .headers()
        .get(header::RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);

    Err(anyhow!(api_status_error(
        endpoint,
        status,
        retry_after.as_deref()
    )))
}

fn api_status_error(endpoint: &str, status: StatusCode, retry_after: Option<&str>) -> String {
    let reason = match status {
        StatusCode::UNAUTHORIZED => "authentication failed; check the FYI API key",
        StatusCode::FORBIDDEN => "permission denied by the FYI API",
        StatusCode::NOT_FOUND => "the requested FYI API resource was not found",
        StatusCode::TOO_MANY_REQUESTS => "rate limited by the FYI API",
        status if status.is_server_error() => "the FYI API returned a server error",
        status if status.is_client_error() => "the FYI API rejected the request",
        _ => "the FYI API returned an unexpected status",
    };

    let retry_hint = retry_after
        .filter(|value| !value.trim().is_empty())
        .map(|value| format!("; retry after {value} seconds"))
        .unwrap_or_default();

    format!("{endpoint} returned HTTP {status}: {reason}{retry_hint}")
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

    #[test]
    fn validate_instance_url_rejects_private_targets_and_credentials() {
        assert!(SyncClient::new("https://www.fyi.org.nz").is_ok());
        assert!(SyncClient::new("https://127.0.0.1").is_err());
        assert!(SyncClient::new("https://example.local").is_err());
        assert!(SyncClient::new("https://example.com").is_ok());
        assert!(SyncClient::new("https://user:pass@example.com").is_err());
        assert!(SyncClient::new("https://example.com?x=1").is_err());
        assert!(SyncClient::new("http://127.0.0.1").is_ok());
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
    async fn search_requests_parses_results_payload() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/search.json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "results": [{
                    "id": 77,
                    "title": "Search result",
                    "body": "Body"
                }]
            })))
            .mount(&server)
            .await;

        let client = SyncClient::new(&server.uri()).unwrap();
        let requests = client.search_requests("transparency").await.unwrap();

        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].id, 77);
        assert_eq!(requests[0].title, "Search result");
    }

    #[tokio::test]
    async fn build_prefilled_url_includes_title_and_body() {
        let server = MockServer::start().await;
        let client = SyncClient::new(&server.uri()).unwrap();

        let url = client
            .build_prefilled_url("ministry", "My title", "My body", Some("foo"))
            .await
            .unwrap();

        assert!(url.to_string().contains("/new/ministry"));
        assert!(url.to_string().contains("title=My+title"));
        assert!(url.to_string().contains("body=My+body"));
        assert!(url.to_string().contains("tags=foo"));
    }

    #[test]
    fn rejects_non_https_or_private_targets() {
        assert!(SyncClient::new("http://example.com").is_err());
        assert!(SyncClient::new("https://localhost").is_err());
        assert!(SyncClient::new("https://127.0.0.1").is_err());
    }

    #[test]
    fn contract_fixtures_match_sync_parser_expectations() {
        let success =
            include_str!("../../../tests/fixtures/api_contracts/request-list-success.json");
        let missing_required = include_str!(
            "../../../tests/fixtures/api_contracts/request-list-missing-required.json"
        );
        let create_success =
            include_str!("../../../tests/fixtures/api_contracts/create-request-success.json");

        let requests =
            parse_request_list(serde_json::from_str(success).expect("valid success fixture"))
                .expect("success fixture parses");
        let create_response: CreateRequestResponse =
            serde_json::from_str(create_success).expect("valid create response fixture");
        let missing_error = parse_request_list(
            serde_json::from_str(missing_required).expect("valid error fixture"),
        )
        .unwrap_err()
        .to_string();

        assert_eq!(requests[0].id, 1001);
        assert_eq!(create_response.id, 2001);
        assert!(missing_error.contains("invalid requests field"));
    }

    #[tokio::test]
    async fn pull_incremental_accepts_unexpected_optional_fields() {
        let server = MockServer::start().await;
        let db = DbPool::new_in_memory().await.unwrap();
        db.run_migrations().await.unwrap();

        Mock::given(method("GET"))
            .and(path("/api/v2/request.json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "requests": [{
                    "id": 71,
                    "title": "Forward compatible request",
                    "body": "Body",
                    "updated_at": "2026-06-30T08:00:00Z",
                    "future_optional_field": {
                        "ignored": true
                    }
                }]
            })))
            .mount(&server)
            .await;

        let client = SyncClient::new(&server.uri()).unwrap();
        let report = client.pull_incremental(&db).await.unwrap();
        let saved = db.get_request(71).await.unwrap().unwrap();

        assert_eq!(report.fetched, 1);
        assert_eq!(saved.title, "Forward compatible request");
    }

    #[tokio::test]
    async fn pull_incremental_rejects_malformed_json_without_local_changes() {
        let server = MockServer::start().await;
        let db = DbPool::new_in_memory().await.unwrap();
        db.run_migrations().await.unwrap();
        let local = request(72, "Local preserved", "2026-06-30T08:00:00Z");
        db.insert_request(&local).await.unwrap();

        Mock::given(method("GET"))
            .and(path("/api/v2/request.json"))
            .respond_with(ResponseTemplate::new(200).set_body_string("{not valid json"))
            .mount(&server)
            .await;

        let client = SyncClient::new(&server.uri()).unwrap();
        let error = client.pull_incremental(&db).await.unwrap_err().to_string();
        let saved = db.get_request(72).await.unwrap().unwrap();

        assert!(error.contains("failed to parse updated requests JSON"));
        assert_eq!(saved.title, "Local preserved");
    }

    #[tokio::test]
    async fn pull_incremental_rejects_missing_required_fields_without_local_changes() {
        let server = MockServer::start().await;
        let db = DbPool::new_in_memory().await.unwrap();
        db.run_migrations().await.unwrap();
        let local = request(73, "Local required fields", "2026-06-30T08:00:00Z");
        db.insert_request(&local).await.unwrap();

        Mock::given(method("GET"))
            .and(path("/api/v2/request.json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "requests": [{
                    "id": 73,
                    "body": "Missing title"
                }]
            })))
            .mount(&server)
            .await;

        let client = SyncClient::new(&server.uri()).unwrap();
        let error = client.pull_incremental(&db).await.unwrap_err().to_string();
        let saved = db.get_request(73).await.unwrap().unwrap();

        assert!(error.contains("invalid requests field"));
        assert_eq!(saved.title, "Local required fields");
    }

    #[tokio::test]
    async fn pull_incremental_returns_non_secret_rate_limit_context() {
        let server = MockServer::start().await;
        let db = DbPool::new_in_memory().await.unwrap();
        db.run_migrations().await.unwrap();

        Mock::given(method("GET"))
            .and(path("/api/v2/request.json"))
            .respond_with(
                ResponseTemplate::new(429)
                    .insert_header("Retry-After", "60")
                    .set_body_string("api_key=secret-token"),
            )
            .mount(&server)
            .await;

        let client = SyncClient::new(&server.uri()).unwrap();
        let error = client.pull_incremental(&db).await.unwrap_err().to_string();

        assert!(error.contains("updated requests endpoint returned HTTP 429"));
        assert!(error.contains("rate limited"));
        assert!(error.contains("retry after 60 seconds"));
        assert!(!error.contains("secret-token"));
    }

    #[tokio::test]
    async fn fetch_request_returns_auth_and_not_found_context() {
        for (status, expected) in [
            (StatusCode::UNAUTHORIZED, "authentication failed"),
            (StatusCode::FORBIDDEN, "permission denied"),
            (StatusCode::NOT_FOUND, "not found"),
        ] {
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .and(path("/api/v2/request/74.json"))
                .respond_with(ResponseTemplate::new(status.as_u16()))
                .mount(&server)
                .await;

            let client = SyncClient::new(&server.uri()).unwrap();
            let error = client.fetch_request(74).await.unwrap_err().to_string();

            assert!(error.contains(&format!("HTTP {status}")));
            assert!(error.contains(expected));
        }
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
    async fn sync_scheduler_runs_health_pull_push_and_shutdown() {
        let server = MockServer::start().await;
        let db = DbPool::new_in_memory().await.unwrap();
        db.run_migrations().await.unwrap();
        let dirty = request(-10, "Scheduler push", "2026-06-30T06:00:00Z");
        db.insert_request(&dirty).await.unwrap();

        Mock::given(method("GET"))
            .and(path("/api/v2/request.json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "requests": [request(61, "Scheduler pull", "2026-06-30T07:00:00Z")]
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/api/v2/request"))
            .respond_with(
                ResponseTemplate::new(201).set_body_json(CreateRequestResponse {
                    id: 5100,
                    url: "https://fyi.org.nz/request/5100".to_string(),
                }),
            )
            .mount(&server)
            .await;

        let client = SyncClient::new(&server.uri()).unwrap();
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let handle = spawn_sync_scheduler(
            db.clone(),
            client,
            SyncConfig {
                pull_interval: Duration::from_millis(10),
                push_initial_backoff: Duration::from_millis(1),
                ..SyncConfig::default()
            },
            shutdown_rx,
        );

        tokio::time::sleep(Duration::from_millis(25)).await;
        let _ = shutdown_tx.send(());
        let reports = handle.await.unwrap().unwrap();
        let pulled = db.get_request(61).await.unwrap().unwrap();
        let pushed_metadata = db.get_request_sync_metadata(-10).await.unwrap().unwrap();

        assert!(reports.iter().any(|report| report.health.api_reachable));
        assert_eq!(pulled.title, "Scheduler pull");
        assert_eq!(pushed_metadata.remote_request_id, Some(5100));
    }

    #[tokio::test]
    async fn health_check_reports_unreachable_api() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v2/request.json"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&server)
            .await;

        let client = SyncClient::new(&server.uri()).unwrap();
        let health = client.health_check().await;

        assert!(health.network_reachable);
        assert!(!health.api_reachable);
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

    #[tokio::test]
    async fn push_dirty_records_nonretryable_auth_failure_without_secret_body() {
        let server = MockServer::start().await;
        let db = DbPool::new_in_memory().await.unwrap();
        db.run_migrations().await.unwrap();
        let local = request(-3, "Auth failure draft", "2026-06-30T08:00:00Z");
        db.insert_request(&local).await.unwrap();

        Mock::given(method("POST"))
            .and(path("/api/v2/request"))
            .respond_with(ResponseTemplate::new(401).set_body_string("token=very-secret"))
            .mount(&server)
            .await;

        let client = SyncClient::new(&server.uri()).unwrap();
        let report = client
            .push_dirty_with_config(
                &db,
                &SyncConfig {
                    push_max_retries: 1,
                    push_initial_backoff: Duration::from_millis(1),
                    ..SyncConfig::default()
                },
            )
            .await
            .unwrap();
        let metadata = db.get_request_sync_metadata(-3).await.unwrap().unwrap();
        let depth = db.get_outgoing_queue_depth().await.unwrap();

        assert_eq!(report.failed, 1);
        assert_eq!(metadata.sync_status.as_str(), "dirty");
        assert_eq!(depth.failed, 1);
    }

    #[tokio::test]
    async fn push_dirty_keeps_dirty_request_when_server_error_retries_exhaust() {
        let server = MockServer::start().await;
        let db = DbPool::new_in_memory().await.unwrap();
        db.run_migrations().await.unwrap();
        let local = request(-4, "Server failure draft", "2026-06-30T08:00:00Z");
        db.insert_request(&local).await.unwrap();

        Mock::given(method("POST"))
            .and(path("/api/v2/request"))
            .respond_with(ResponseTemplate::new(500).set_body_string("temporary"))
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
        let saved = db.get_request(-4).await.unwrap().unwrap();
        let metadata = db.get_request_sync_metadata(-4).await.unwrap().unwrap();

        assert_eq!(report.failed, 1);
        assert_eq!(saved.title, "Server failure draft");
        assert_eq!(metadata.sync_status.as_str(), "dirty");
    }

    #[test]
    fn api_status_error_classifies_contract_failures_without_body_content() {
        let cases = [
            (StatusCode::UNAUTHORIZED, "authentication failed"),
            (StatusCode::FORBIDDEN, "permission denied"),
            (StatusCode::NOT_FOUND, "not found"),
            (StatusCode::TOO_MANY_REQUESTS, "rate limited"),
            (StatusCode::INTERNAL_SERVER_ERROR, "server error"),
        ];

        for (status, expected) in cases {
            let message = api_status_error("request endpoint", status, Some("30"));

            assert!(message.contains(&format!("HTTP {status}")));
            assert!(message.contains(expected));
            assert!(message.contains("retry after 30 seconds"));
            assert!(!message.contains("api_key"));
            assert!(!message.contains("token"));
        }
    }

    #[test]
    fn last_write_wins_chooses_newer_updated_at() {
        let local = request(5, "Local", "2026-06-30T01:00:00Z");
        let remote = request(5, "Remote", "2026-06-30T02:00:00Z");

        assert_eq!(last_write_wins(&local, &remote).title, "Remote");
    }

    #[test]
    fn three_way_merge_applies_non_conflicting_remote_fields() {
        let base = request(6, "Base title", "2026-06-30T01:00:00Z");
        let mut local = base.clone();
        local.body = "Local body".to_string();
        let mut remote = base.clone();
        remote.title = "Remote title".to_string();

        let outcome = three_way_merge_request(&base, &local, &remote);

        assert!(outcome.conflicting_fields.is_empty());
        assert_eq!(outcome.request.title, "Remote title");
        assert_eq!(outcome.request.body, "Local body");
    }

    #[tokio::test]
    async fn pull_marks_request_conflicted_when_local_and_remote_change_same_field() {
        let server = MockServer::start().await;
        let db = DbPool::new_in_memory().await.unwrap();
        db.run_migrations().await.unwrap();
        let base = request(7, "Base title", "2026-06-30T01:00:00Z");
        db.upsert_synced_request(&base, base.updated_at.as_deref())
            .await
            .unwrap();
        let mut local = base.clone();
        local.title = "Local title".to_string();
        local.updated_at = Some("2026-06-30T02:00:00Z".to_string());
        db.update_request(&local).await.unwrap();

        Mock::given(method("GET"))
            .and(path("/api/v2/request.json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "requests": [request(7, "Remote title", "2026-06-30T03:00:00Z")]
            })))
            .mount(&server)
            .await;

        let client = SyncClient::new(&server.uri()).unwrap();
        let report = client.pull_incremental(&db).await.unwrap();
        let metadata = db.get_request_sync_metadata(7).await.unwrap().unwrap();

        assert_eq!(report.applied, 1);
        assert_eq!(metadata.sync_status.as_str(), "conflict");
        assert_eq!(metadata.conflict_version, 1);
    }

    #[tokio::test]
    async fn pull_merge_does_not_record_remote_fields_as_local_changes() {
        let server = MockServer::start().await;
        let db = DbPool::new_in_memory().await.unwrap();
        db.run_migrations().await.unwrap();
        let base = request(8, "Base title", "2026-06-30T01:00:00Z");
        db.upsert_synced_request(&base, base.updated_at.as_deref())
            .await
            .unwrap();

        let mut local = base.clone();
        local.body = "Local body".to_string();
        local.updated_at = Some("2026-06-30T02:00:00Z".to_string());
        db.update_request(&local).await.unwrap();

        let mut remote = base.clone();
        remote.title = "Remote title".to_string();
        remote.updated_at = Some("2026-06-30T03:00:00Z".to_string());
        Mock::given(method("GET"))
            .and(path("/api/v2/request.json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "requests": [remote]
            })))
            .mount(&server)
            .await;

        let client = SyncClient::new(&server.uri()).unwrap();
        client.pull_incremental(&db).await.unwrap();

        let saved = db.get_request(8).await.unwrap().unwrap();
        let metadata = db.get_request_sync_metadata(8).await.unwrap().unwrap();
        let changes = db.list_unsynced_field_changes(8).await.unwrap();
        let fields = changes
            .iter()
            .map(|change| change.field_name.as_str())
            .collect::<Vec<_>>();

        assert_eq!(saved.title, "Remote title");
        assert_eq!(saved.body, "Local body");
        assert_eq!(metadata.sync_status.as_str(), "dirty");
        assert!(fields.contains(&"body"));
        assert!(fields.contains(&"updated_at"));
        assert!(!fields.contains(&"title"));
    }
}
