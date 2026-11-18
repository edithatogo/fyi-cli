//! A fresh, single-list sync pull with immutable observed response evidence.

use super::*;
use crate::acquisition_receipts::{
    canonical_json_bytes, verify_receipt, verify_receipt_raw_bytes, verify_receipt_result_bytes,
    write_evidence_bytes_atomic, write_receipt_atomic,
};
use crate::provenance::sha256_hex;
use anyhow::{bail, ensure, Context, Result};
use serde_json::json;
use std::fs;
use std::future::Future;
use std::path::{Component, Path, PathBuf};

const MAX_RESPONSE_BYTES: u64 = 16 * 1024 * 1024;

/// Separate immutable files for a single fresh source response and its result.
pub struct SyncPullEvidencePaths<'a> {
    pub raw: &'a Path,
    pub result: &'a Path,
    pub receipt: &'a Path,
}

fn absolute_path(path: &Path) -> Result<PathBuf> {
    let path = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            component => normalized.push(component.as_os_str()),
        }
    }
    Ok(normalized)
}

fn reject_symlinks(path: &Path) -> Result<()> {
    let absolute = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()?.join(path)
    };
    for ancestor in absolute.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(metadata) => ensure!(
                !metadata.file_type().is_symlink(),
                "refusing symlink in sync evidence path"
            ),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

/// Reject collisions and existing evidence before a database or source is opened.
pub fn preflight_sync_pull_evidence(
    paths: &SyncPullEvidencePaths<'_>,
    database: Option<&Path>,
) -> Result<()> {
    let outputs = [paths.raw, paths.result, paths.receipt];
    let mut normalized = Vec::with_capacity(4);
    for path in outputs.into_iter().chain(database) {
        reject_symlinks(path)?;
        let value = absolute_path(path)?;
        ensure!(
            normalized
                .iter()
                .all(|other: &PathBuf| { !value.starts_with(other) && !other.starts_with(&value) }),
            "sync database and evidence paths must be distinct and non-overlapping"
        );
        normalized.push(value);
    }
    if database.is_some() {
        let database = normalized.last().context("database path is missing")?;
        for suffix in ["-wal", "-shm", "-journal"] {
            let mut sidecar = database.as_os_str().to_owned();
            sidecar.push(suffix);
            ensure!(
                !normalized[..3].contains(&PathBuf::from(sidecar)),
                "sync evidence path overlaps a database sidecar"
            );
        }
    }
    for path in outputs {
        ensure!(
            path.file_name().is_some(),
            "sync evidence path must name a file"
        );
        match fs::symlink_metadata(path) {
            Ok(_) => bail!("sync evidence path already exists; choose fresh files"),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(parent)?;
        }
        reject_symlinks(path)?;
    }
    Ok(())
}

/// A dedicated client whose transport does not follow redirects, decode bodies,
/// reuse a cached response or retry a request.
pub struct SyncPullEvidenceClient {
    client: SyncClient,
}

pub(super) struct SyncPullResponse {
    pub(super) status: StatusCode,
    pub(super) url: Url,
    pub(super) headers: header::HeaderMap,
    pub(super) body: Vec<u8>,
}

fn capture_http(identity: &crate::agent_runtime::ClientIdentity) -> Result<Client> {
    identity.validate()?;
    Client::builder()
        .user_agent(identity.user_agent())
        .timeout(Duration::from_secs(60))
        .redirect(reqwest::redirect::Policy::none())
        .no_gzip()
        .no_brotli()
        .no_deflate()
        .no_zstd()
        .build()
        .context("failed to build sync evidence client")
}

impl SyncPullEvidenceClient {
    pub fn new(base_url: &str) -> Result<Self> {
        let identity = crate::agent_runtime::ClientIdentity::default_identity(None)?;
        let http = capture_http(&identity)?;
        Ok(Self {
            client: SyncClient::with_http_client_and_identity(base_url, http, identity)?,
        })
    }

    /// Capture one complete response, apply parsed updates, then publish a receipt.
    /// An incomplete response has no v1 representation and produces no receipt.
    pub async fn pull(&self, db: &DbPool, paths: &SyncPullEvidencePaths<'_>) -> Result<PullReport> {
        self.pull_using(db, paths, |url| async move {
            let response = self
                .client
                .send_guarded_with_limits(
                    self.client
                        .http
                        .get(url)
                        .header(header::ACCEPT_ENCODING, "identity"),
                    Some(MAX_RESPONSE_BYTES),
                    false,
                    "sync_pull_evidence",
                )
                .await?;
            Ok(SyncPullResponse {
                status: response.source_status,
                url: response.final_url,
                headers: response.headers,
                body: response.body,
            })
        })
        .await
    }

    pub(super) async fn pull_using<F, Fut>(
        &self,
        db: &DbPool,
        paths: &SyncPullEvidencePaths<'_>,
        fetch: F,
    ) -> Result<PullReport>
    where
        F: FnOnce(Url) -> Fut,
        Fut: Future<Output = Result<SyncPullResponse>>,
    {
        preflight_sync_pull_evidence(paths, None)?;
        let since = db.get_latest_sync_timestamp().await?;
        ensure!(
            since.as_ref().is_none_or(|value| value.len() <= 4096),
            "stored sync boundary exceeds the byte limit"
        );
        let since_digest = since
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(|value| sha256_hex(value.as_bytes()));
        let mut url = self.client.base_url.join("api/v2/request.json")?;
        if let Some(since) = since.as_deref().filter(|value| !value.trim().is_empty()) {
            url.query_pairs_mut().append_pair("updated_since", since);
        }
        let started = now_rfc3339_millis_z();
        let response = fetch(url.clone())
            .await
            .map_err(|_| anyhow!("sync source exchange incomplete; no receipt was published"))?;
        ensure!(
            response.url == url,
            "sync response URL differs from its request; no receipt was published"
        );
        ensure!(
            response.body.len() as u64 <= MAX_RESPONSE_BYTES,
            "sync response exceeds its limit; no receipt was published"
        );
        let status = response.status;
        let mut response_url = response.url.clone();
        if response_url.query().is_some() {
            response_url.set_query(Some("REDACTED"));
        }
        response_url.set_fragment(None);
        let mut source_url = self.client.base_url.clone();
        if source_url.query().is_some() {
            source_url.set_query(Some("REDACTED"));
        }
        source_url.set_fragment(None);
        let raw = response.body;
        write_evidence_bytes_atomic(paths.raw, &raw)
            .context("sync response could not be saved; no receipt was published")?;

        let outcome: Result<PullReport> = async {
            ensure!(
                status.is_success(),
                "sync source returned a non-success status"
            );
            for value in response.headers.get_all(header::CONTENT_ENCODING) {
                let encoding = value.to_str().context("invalid source content encoding")?;
                ensure!(
                    encoding
                        .split(',')
                        .all(|part| part.trim().eq_ignore_ascii_case("identity")),
                    "unsupported sync content encoding"
                );
            }
            let value: Value = serde_json::from_slice(&raw).context("invalid sync source JSON")?;
            let requests = parse_request_list(value)?;
            let applied = apply_remote_requests(db, &requests).await?;
            Ok(PullReport {
                fetched: requests.len(),
                applied,
                source: "api".to_string(),
            })
        }
        .await;

        let projection = match &outcome {
            Ok(report) => {
                json!({"schema": "urn:fyi-cli:sync-pull-result:1", "fetched": report.fetched, "applied": report.applied, "source": report.source})
            }
            Err(_) => json!({"schema": "urn:fyi-cli:sync-pull-result:1", "status": "failed"}),
        };
        let result_bytes = canonical_json_bytes(&projection)?;
        let mut receipt = json!({
            "schema": "urn:fyi-cli:acquisition-receipt", "schema_version": "1.0.0",
            "command": "sync-pull", "adapter": {"id": "alaveteli-sync", "version": env!("CARGO_PKG_VERSION")},
            "source": {"url": source_url.as_str()},
            "request_bounds": {"updated_since_sha256": since_digest, "max_response_bytes": MAX_RESPONSE_BYTES, "redirects": 0, "retries": 0},
            "started_at": started, "completed_at": now_rfc3339_millis_z(),
            "status": if outcome.is_ok() { "succeeded" } else { "failed" },
            "responses": [{"url": response_url.as_str(), "status": status.as_u16(), "bytes": raw.len(), "sha256": sha256_hex(&raw), "attempts": 1, "retry_delays_seconds": []}],
            "totals": {"requests": 1, "bytes": raw.len(), "retries": 0},
            "rate_limit": {"name": null, "minimum_interval_seconds": null},
            "checkpoint": {"before_sha256": null, "after_sha256": null},
            "result": {"representation": "canonical_result_projection", "media_type": "application/json", "bytes": result_bytes.len(), "sha256": sha256_hex(&result_bytes)},
        });
        if outcome.is_err() {
            receipt["failure"] = json!({"type": "SyncPullFailed"});
        }
        receipt["receipt_sha256"] = json!(sha256_hex(&canonical_json_bytes(&receipt)?));
        verify_receipt(&receipt)?;
        verify_receipt_raw_bytes(&receipt, &raw)?;
        verify_receipt_result_bytes(&receipt, &result_bytes)?;
        write_evidence_bytes_atomic(paths.result, &result_bytes)?;
        write_receipt_atomic(paths.receipt, &receipt)?;
        outcome
    }
}
