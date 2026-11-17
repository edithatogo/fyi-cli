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
        let started = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
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
            "started_at": started, "completed_at": Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
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

#[cfg(test)]
mod lineage_tests {
    use super::*;

    struct EvidenceFixture {
        _directory: tempfile::TempDir,
        raw: PathBuf,
        result: PathBuf,
        receipt: PathBuf,
    }

    impl EvidenceFixture {
        fn new() -> Self {
            let directory =
                tempfile::tempdir_in(fs::canonicalize(std::env::temp_dir()).unwrap()).unwrap();
            Self {
                raw: directory.path().join("response.raw"),
                result: directory.path().join("result.json"),
                receipt: directory.path().join("receipt.json"),
                _directory: directory,
            }
        }

        fn paths(&self) -> SyncPullEvidencePaths<'_> {
            SyncPullEvidencePaths {
                raw: &self.raw,
                result: &self.result,
                receipt: &self.receipt,
            }
        }

        fn read(&self) -> (Value, Value) {
            let result = fs::read(&self.result).unwrap();
            let receipt: Value = serde_json::from_slice(&fs::read(&self.receipt).unwrap()).unwrap();
            verify_receipt(&receipt).unwrap();
            verify_receipt_raw_bytes(&receipt, &fs::read(&self.raw).unwrap()).unwrap();
            verify_receipt_result_bytes(&receipt, &result).unwrap();
            (receipt, serde_json::from_slice(&result).unwrap())
        }
    }

    async fn database() -> DbPool {
        let db = DbPool::new_in_memory().await.unwrap();
        db.run_migrations().await.unwrap();
        db
    }

    fn request_body(ids: &[i64]) -> Vec<u8> {
        serde_json::to_vec(&json!({
            "requests": ids.iter().map(|id| json!({
                "id": id, "title": "Synthetic request", "body": "Synthetic body",
                "updated_at": "2026-01-01T00:00:00Z",
            })).collect::<Vec<_>>()
        }))
        .unwrap()
    }

    fn boundary(value: Option<&str>) -> Value {
        json!({
            "schema": "urn:fyi-cli:sync-boundary:1",
            "updated_since_sha256": value.map(|value| sha256_hex(value.as_bytes())),
        })
    }

    fn assert_lineage(receipt: &Value, result: &Value, before: Value, after: Value) {
        assert_eq!(result["checkpoint"]["before"], before);
        assert_eq!(result["checkpoint"]["after"], after);
        assert_eq!(
            receipt["checkpoint"]["before_sha256"],
            sha256_hex(&canonical_json_bytes(&before).unwrap())
        );
        assert_eq!(
            receipt["checkpoint"]["after_sha256"],
            sha256_hex(&canonical_json_bytes(&after).unwrap())
        );
    }

    #[tokio::test]
    async fn successful_pull_binds_before_and_after_redacted_boundaries() {
        let db = database().await;
        let files = EvidenceFixture::new();
        let client = SyncPullEvidenceClient::new("https://example.test").unwrap();
        let report = client
            .pull_using(&db, &files.paths(), |url| async move {
                assert!(url.query().is_none());
                Ok(SyncPullResponse {
                    status: StatusCode::OK,
                    url,
                    headers: header::HeaderMap::new(),
                    body: request_body(&[1]),
                })
            })
            .await
            .unwrap();
        assert_eq!(report.applied, 1);
        let after = db.get_latest_sync_timestamp().await.unwrap().unwrap();
        let (receipt, result) = files.read();
        assert_lineage(&receipt, &result, boundary(None), boundary(Some(&after)));
        assert!(!result.to_string().contains(&after));
        assert!(!receipt["request_bounds"].to_string().contains(&after));
        assert_eq!(receipt["status"], "succeeded");
    }

    #[tokio::test]
    async fn empty_pull_preserves_the_observed_checkpoint() {
        let db = database().await;
        let files = EvidenceFixture::new();
        let client = SyncPullEvidenceClient::new("https://example.test").unwrap();
        client
            .pull_using(&db, &files.paths(), |url| async move {
                Ok(SyncPullResponse {
                    status: StatusCode::OK,
                    url,
                    headers: header::HeaderMap::new(),
                    body: request_body(&[]),
                })
            })
            .await
            .unwrap();
        let (receipt, result) = files.read();
        assert_lineage(&receipt, &result, boundary(None), boundary(None));
        assert_eq!(result["applied"], 0);
    }

    #[tokio::test]
    async fn failed_pull_records_rate_signals_without_header_text_or_changed_checkpoint() {
        for retry_after in ["17", "synthetic-private-header"] {
            let db = database().await;
            let existing: AlaveteliRequest = serde_json::from_value(json!({
                "id": 99, "title": "Synthetic local request", "body": "Synthetic body",
            }))
            .unwrap();
            db.upsert_synced_request(&existing, None).await.unwrap();
            sqlx::query("UPDATE sync_metadata SET last_synced_at = 'synthetic-private-boundary'")
                .execute(db.pool())
                .await
                .unwrap();
            let files = EvidenceFixture::new();
            let client = SyncPullEvidenceClient::new("https://example.test").unwrap();
            let mut headers = header::HeaderMap::new();
            headers.insert(header::RETRY_AFTER, retry_after.parse().unwrap());
            headers.insert("ratelimit-limit", "100".parse().unwrap());
            headers.insert("ratelimit-remaining", "0".parse().unwrap());
            headers.insert("ratelimit-reset", "29".parse().unwrap());
            headers.insert("x-advisory-status", "synthetic-private-header".parse().unwrap());
            headers.insert(header::SET_COOKIE, "synthetic-private-header".parse().unwrap());
            let error = client
                .pull_using(&db, &files.paths(), |url| async move {
                    Ok(SyncPullResponse {
                        status: StatusCode::TOO_MANY_REQUESTS,
                        url,
                        headers,
                        body: b"synthetic-private-body".to_vec(),
                    })
                })
                .await
                .unwrap_err();
            let (receipt, result) = files.read();
            assert_lineage(
                &receipt,
                &result,
                boundary(Some("synthetic-private-boundary")),
                boundary(Some("synthetic-private-boundary")),
            );
            assert_eq!(receipt["status"], "failed");
            assert_eq!(receipt["totals"]["retries"], 0);
            assert_eq!(receipt["responses"][0]["retry_delays_seconds"], json!([]));
            assert_eq!(
                result["rate_limit_observation"],
                json!({
                    "limit": 100, "remaining": 0, "reset_seconds": 29,
                    "retry_after_seconds": if retry_after == "17" { Some(17) } else { None },
                })
            );
            for text in [format!("{error:#}"), result.to_string(), receipt.to_string()] {
                assert!(!text.contains("synthetic-private-header"));
                assert!(!text.contains("synthetic-private-body"));
                assert!(!text.contains("synthetic-private-boundary"));
            }
        }
    }

    #[tokio::test]
    async fn partial_application_failure_records_the_observed_advanced_boundary() {
        let db = database().await;
        sqlx::query(
            "CREATE TRIGGER reject_second BEFORE INSERT ON requests WHEN NEW.id = 2
             BEGIN SELECT RAISE(ABORT, 'synthetic application failure'); END",
        )
        .execute(db.pool())
        .await
        .unwrap();
        let files = EvidenceFixture::new();
        let client = SyncPullEvidenceClient::new("https://example.test").unwrap();
        assert!(client
            .pull_using(&db, &files.paths(), |url| async move {
                Ok(SyncPullResponse {
                    status: StatusCode::OK,
                    url,
                    headers: header::HeaderMap::new(),
                    body: request_body(&[1, 2]),
                })
            })
            .await
            .is_err());
        assert!(db.get_request(1).await.unwrap().is_some());
        assert!(db.get_request(2).await.unwrap().is_none());
        let after = db.get_latest_sync_timestamp().await.unwrap().unwrap();
        let (receipt, result) = files.read();
        assert_lineage(&receipt, &result, boundary(None), boundary(Some(&after)));
        assert_eq!(receipt["status"], "failed");
        assert_eq!(result["status"], "failed");
        assert!(result.get("applied").is_none());
    }

    #[tokio::test]
    async fn unavailable_final_boundary_never_publishes_a_completed_receipt() {
        let db = database().await;
        let files = EvidenceFixture::new();
        let client = SyncPullEvidenceClient::new("https://example.test").unwrap();
        let outcome = client
            .pull_using(&db, &files.paths(), |url| {
                let db = &db;
                async move {
                    sqlx::query("DROP TABLE sync_metadata")
                        .execute(db.pool())
                        .await
                        .unwrap();
                    Ok(SyncPullResponse {
                        status: StatusCode::OK,
                        url,
                        headers: header::HeaderMap::new(),
                        body: request_body(&[]),
                    })
                }
            })
            .await;
        assert!(outcome.is_err());
        assert_eq!(fs::read(&files.raw).unwrap(), request_body(&[]));
        assert!(!files.result.exists());
        assert!(!files.receipt.exists());
    }

    #[tokio::test]
    async fn result_publication_failure_preserves_raw_and_never_completes_receipt() {
        let db = database().await;
        let files = EvidenceFixture::new();
        let client = SyncPullEvidenceClient::new("https://example.test").unwrap();
        let outcome = client
            .pull_using(&db, &files.paths(), |url| {
                fs::write(&files.result, b"preserved concurrent evidence").unwrap();
                async move {
                    Ok(SyncPullResponse {
                        status: StatusCode::OK,
                        url,
                        headers: header::HeaderMap::new(),
                        body: request_body(&[1]),
                    })
                }
            })
            .await;
        assert!(outcome.is_err());
        assert_eq!(fs::read(&files.raw).unwrap(), request_body(&[1]));
        assert_eq!(fs::read(&files.result).unwrap(), b"preserved concurrent evidence");
        assert!(!files.receipt.exists());
        assert!(db.get_request(1).await.unwrap().is_some());
    }
}
