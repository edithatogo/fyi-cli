//! A single public request acquisition with complete response evidence.

use super::{
    preflight_sync_pull_evidence, AlaveteliRequest, DbPool, FetchedRequestEvidence,
    SourceHttpError, SyncClient, SyncPullEvidencePaths,
};
use crate::acquisition_receipts::{
    canonical_json_bytes, verify_receipt, verify_receipt_raw_bytes, verify_receipt_result_bytes,
    write_evidence_bytes_atomic, write_receipt_atomic,
};
use crate::provenance::sha256_hex;
use anyhow::{anyhow, ensure, Context, Result};
use chrono::{SecondsFormat, Utc};
use reqwest::{header, Client, Url};
use serde_json::{json, Value};
use std::future::Future;
use std::time::Duration;

const MAX_RESPONSE_BYTES: u64 = 16 * 1024 * 1024;

/// Read-only public request transport with no redirects, decoding, cache or retries.
///
/// Complete responses can be retained even when their status or representation
/// is unusable. An incomplete exchange has no receipt-v1 representation and
/// fails without publishing a receipt.
pub struct RequestEvidenceClient {
    client: SyncClient,
}

struct RequestObservation {
    outcome: Result<AlaveteliRequest>,
    raw: Vec<u8>,
    result: Vec<u8>,
    receipt: Value,
}

pub(super) fn capture_http(identity: &crate::agent_runtime::ClientIdentity) -> Result<Client> {
    identity.validate()?;
    Client::builder()
        .user_agent(identity.user_agent())
        .timeout(Duration::from_secs(60))
        .redirect(reqwest::redirect::Policy::none())
        // Protocol-level retries would otherwise be hidden from receipt totals.
        .retry(reqwest::retry::never())
        .no_gzip()
        .no_brotli()
        .no_deflate()
        .no_zstd()
        .build()
        .context("failed to build request evidence client")
}

impl RequestEvidenceClient {
    /// Build a fresh guarded client for a validated public source origin.
    pub fn new(base_url: &str) -> Result<Self> {
        let identity = crate::agent_runtime::ClientIdentity::default_identity(None)?;
        let http = capture_http(&identity)?;
        Ok(Self {
            client: SyncClient::with_http_client_and_identity(base_url, http, identity)?,
        })
    }

    /// Reuse the caller's source policy and limiter with a fresh evidence transport.
    ///
    /// Only a built-in client's paired transport has known identity/proxy/TLS
    /// defaults. An opaque custom client must never have those policies silently
    /// replaced. Preserve the built-in client's guardrails and shared pacing.
    pub(super) fn from_sync_client(source: &SyncClient) -> Result<Self> {
        let http = source.evidence_http.as_ref().context(
            "custom HTTP client cannot establish request evidence; use its ordinary request API or explicitly construct RequestEvidenceClient",
        )?;
        let mut client = source.clone();
        client.http = http.clone();
        Ok(Self { client })
    }

    /// Coordinate the existing source pacing database before each acquisition.
    pub fn with_shared_pacing(
        mut self,
        db: DbPool,
        name: impl Into<String>,
        minimum_interval: Duration,
    ) -> Self {
        self.client = self.client.with_shared_pacing(db, name, minimum_interval);
        self
    }

    /// Persist complete raw bytes, a canonical result and the receipt in that order.
    ///
    /// HTTP, parsing, identifier and encoding failures preserve failed evidence
    /// and still return an error. Publication failures never report success.
    pub async fn fetch(
        &self,
        request_id: i64,
        paths: &SyncPullEvidencePaths<'_>,
    ) -> Result<AlaveteliRequest> {
        self.fetch_using(request_id, paths, |request| {
            self.client.execute_guarded_request(request)
        })
        .await
    }

    async fn fetch_using<F, Fut>(
        &self,
        request_id: i64,
        paths: &SyncPullEvidencePaths<'_>,
        execute: F,
    ) -> Result<AlaveteliRequest>
    where
        F: FnOnce(reqwest::Request) -> Fut,
        Fut: Future<Output = Result<(reqwest::Response, Url)>>,
    {
        ensure!(request_id > 0, "request ID must be positive");
        preflight_sync_pull_evidence(paths, None)?;
        let observed = self.observe_using(request_id, execute).await?;
        // A source wait must not let a replaced parent redirect evidence into
        // another directory. These checks and no-clobber publication assume
        // trusted operator directories; they are not directory-handle race protection.
        preflight_sync_pull_evidence(paths, None)
            .context("request evidence changed during acquisition; no receipt was published")?;
        write_evidence_bytes_atomic(paths.raw, &observed.raw)
            .context("request response could not be saved; no receipt was published")?;
        write_evidence_bytes_atomic(paths.result, &observed.result)
            .context("request result could not be saved; no receipt was published")?;
        write_receipt_atomic(paths.receipt, &observed.receipt)?;
        observed.outcome
    }

    /// Return successful response and projection evidence without writing files.
    ///
    /// Complete source failures return errors. Call [`Self::fetch`] with explicit
    /// output paths when those failed observations must also be retained.
    pub async fn fetch_evidence(&self, request_id: i64) -> Result<FetchedRequestEvidence> {
        let observed = self
            .observe_using(request_id, |request| {
                self.client.execute_guarded_request(request)
            })
            .await?;
        Ok(FetchedRequestEvidence {
            request: observed.outcome?,
            raw_body: observed.raw,
            result_projection: observed.result,
            receipt: observed.receipt,
        })
    }

    async fn observe_using<F, Fut>(&self, request_id: i64, execute: F) -> Result<RequestObservation>
    where
        F: FnOnce(reqwest::Request) -> Fut,
        Fut: Future<Output = Result<(reqwest::Response, Url)>>,
    {
        ensure!(request_id > 0, "request ID must be positive");
        if let Some(shared) = &self.client.shared_pacing {
            ensure!(
                !shared.name.trim().is_empty(),
                "rate limit name must not be blank"
            );
        }
        let url = self
            .client
            .base_url
            .join(&format!("api/v2/request/{request_id}.json"))?;
        let started = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        let response = self
            .client
            .send_guarded_with_limits_using(
                self.client
                    .http
                    .get(url.clone())
                    .header(header::ACCEPT_ENCODING, "identity"),
                Some(MAX_RESPONSE_BYTES),
                false,
                "request_evidence",
                execute,
            )
            .await
            .map_err(|_| anyhow!("request source exchange incomplete; no receipt was published"))?;
        ensure!(
            response.final_url == url,
            "request response URL differs from its request; no receipt was published"
        );
        let status = response.source_status;
        let outcome = (|| -> Result<AlaveteliRequest> {
            if !status.is_success() {
                return Err(SourceHttpError::new(status.as_u16()).into());
            }
            for value in response.headers.get_all(header::CONTENT_ENCODING) {
                let encoding = value
                    .to_str()
                    .map_err(|_| anyhow!("invalid source content encoding"))?;
                ensure!(
                    encoding
                        .split(',')
                        .all(|part| part.trim().eq_ignore_ascii_case("identity")),
                    "unsupported request content encoding"
                );
            }
            let request: AlaveteliRequest = serde_json::from_slice(&response.body)
                .map_err(|_| anyhow!("invalid public request JSON"))?;
            ensure!(
                request.id == request_id,
                "upstream request identifier does not match requested ID"
            );
            Ok(request)
        })();
        let projection = match &outcome {
            Ok(request) => serde_json::to_value(request)?,
            Err(_) => {
                json!({"schema": "urn:fyi-cli:request-acquisition-failure:1", "status": "failed"})
            }
        };
        let result = canonical_json_bytes(&projection)?;
        let mut safe_url = url.clone();
        if safe_url.query().is_some() {
            safe_url.set_query(Some("REDACTED"));
        }
        safe_url.set_fragment(None);
        let raw = response.body;
        let limiter = self.client.shared_pacing.as_ref();
        let mut receipt = json!({
            "schema": "urn:fyi-cli:acquisition-receipt", "schema_version": "1.0.0",
            "command": "fetch-request", "adapter": {"id": "alaveteli-request-json", "version": env!("CARGO_PKG_VERSION")},
            "source": {"url": safe_url.as_str()},
            "request_bounds": {"request_id": request_id, "max_response_bytes": MAX_RESPONSE_BYTES, "redirects": 0, "retries": 0},
            "started_at": started, "completed_at": Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
            "status": if outcome.is_ok() { "succeeded" } else { "failed" },
            "responses": [{"url": safe_url.as_str(), "status": status.as_u16(), "bytes": raw.len(), "sha256": sha256_hex(&raw), "attempts": 1, "retry_delays_seconds": []}],
            "totals": {"requests": 1, "bytes": raw.len(), "retries": 0},
            "rate_limit": {"name": limiter.map(|shared| &shared.name), "minimum_interval_seconds": limiter.map(|shared| shared.minimum_interval.as_secs_f64())},
            "checkpoint": {"before_sha256": null, "after_sha256": null},
            "result": {"representation": "canonical_result_projection", "media_type": "application/json", "bytes": result.len(), "sha256": sha256_hex(&result)},
        });
        if outcome.is_err() {
            receipt["failure"] = json!({"type": "RequestFetchFailed"});
        }
        receipt["receipt_sha256"] = json!(sha256_hex(&canonical_json_bytes(&receipt)?));
        verify_receipt(&receipt)?;
        verify_receipt_raw_bytes(&receipt, &raw)?;
        verify_receipt_result_bytes(&receipt, &result)?;
        Ok(RequestObservation {
            outcome,
            raw,
            result,
            receipt,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    const VALID: &[u8] = b"{\"id\":42,\"title\":\"Water records\",\"body\":\"Synthetic request\"}";

    struct Harness {
        _temp: tempfile::TempDir,
        raw: PathBuf,
        result: PathBuf,
        receipt: PathBuf,
    }

    impl Harness {
        fn new() -> Self {
            let parent = fs::canonicalize(std::env::temp_dir()).unwrap();
            let temp = tempfile::tempdir_in(parent).unwrap();
            Self {
                raw: temp.path().join("response.raw"),
                result: temp.path().join("result.json"),
                receipt: temp.path().join("receipt.json"),
                _temp: temp,
            }
        }

        fn paths(&self) -> SyncPullEvidencePaths<'_> {
            SyncPullEvidencePaths {
                raw: &self.raw,
                result: &self.result,
                receipt: &self.receipt,
            }
        }

        fn verified_receipt(&self) -> Value {
            let receipt =
                crate::receipt_wire::parse_receipt_bytes(&fs::read(&self.receipt).unwrap())
                    .unwrap();
            verify_receipt_raw_bytes(&receipt, &fs::read(&self.raw).unwrap()).unwrap();
            verify_receipt_result_bytes(&receipt, &fs::read(&self.result).unwrap()).unwrap();
            receipt
        }

        fn assert_absent(&self) {
            assert!(!self.raw.exists() && !self.result.exists() && !self.receipt.exists());
        }
    }

    async fn client() -> RequestEvidenceClient {
        let client = RequestEvidenceClient::new("https://source.example.test").unwrap();
        client.client.middleware.lock().await.pacing.current_delay = Duration::ZERO;
        client
    }

    fn response(
        request: reqwest::Request,
        status: u16,
        body: &[u8],
        encoding: Option<&str>,
    ) -> Result<(reqwest::Response, Url)> {
        assert_eq!(request.method(), reqwest::Method::GET);
        assert_eq!(request.url().path(), "/api/v2/request/42.json");
        assert_eq!(request.headers()[header::ACCEPT_ENCODING], "identity");
        assert!(!request.headers().contains_key(header::IF_NONE_MATCH));
        assert!(!request.headers().contains_key(header::IF_MODIFIED_SINCE));
        let url = request.url().clone();
        let mut builder = http::Response::builder().status(status);
        if let Some(encoding) = encoding {
            builder = builder.header(header::CONTENT_ENCODING, encoding);
        }
        Ok((
            builder.body(reqwest::Body::from(body.to_vec()))?.into(),
            url,
        ))
    }

    #[tokio::test]
    async fn complete_success_retains_exact_response_and_canonical_request() {
        let harness = Harness::new();
        let calls = AtomicUsize::new(0);
        let request = client()
            .await
            .fetch_using(42, &harness.paths(), |request| {
                calls.fetch_add(1, Ordering::SeqCst);
                async move { response(request, 200, VALID, Some("identity")) }
            })
            .await
            .unwrap();
        assert_eq!(request.id, 42);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(fs::read(&harness.raw).unwrap(), VALID);
        let receipt = harness.verified_receipt();
        assert_eq!(receipt["status"], "succeeded");
        assert_eq!(receipt["request_bounds"]["redirects"], 0);
        assert_eq!(receipt["totals"]["requests"], 1);
        assert_eq!(receipt["totals"]["retries"], 0);
        assert_eq!(
            fs::read(&harness.result).unwrap(),
            canonical_json_bytes(&serde_json::to_value(request).unwrap()).unwrap()
        );
    }

    #[tokio::test]
    async fn complete_http_failures_preserve_failed_receipts_and_return_errors() {
        for status in [302, 304, 403, 404, 429, 503] {
            let harness = Harness::new();
            let calls = AtomicUsize::new(0);
            let error = client()
                .await
                .fetch_using(42, &harness.paths(), |request| {
                    calls.fetch_add(1, Ordering::SeqCst);
                    async move { response(request, status, b"synthetic-private-response", None) }
                })
                .await
                .unwrap_err();
            assert_eq!(calls.load(Ordering::SeqCst), 1);
            assert!(!format!("{error:#}").contains("synthetic-private-response"));
            let receipt = harness.verified_receipt();
            assert_eq!(receipt["status"], "failed");
            assert_eq!(receipt["responses"][0]["status"], status);
            assert_eq!(receipt["failure"]["type"], "RequestFetchFailed");
        }
    }

    #[tokio::test]
    async fn complete_invalid_representations_preserve_failure_bytes() {
        for (body, encoding) in [
            (b"synthetic-private-invalid-json".as_slice(), None),
            (
                b"{\"id\":41,\"title\":\"Other\",\"body\":\"Synthetic\"}".as_slice(),
                None,
            ),
            (VALID, Some("gzip")),
        ] {
            let harness = Harness::new();
            let error = client()
                .await
                .fetch_using(42, &harness.paths(), |request| async move {
                    response(request, 200, body, encoding)
                })
                .await
                .unwrap_err();
            assert!(!format!("{error:#}").contains("synthetic-private-invalid-json"));
            assert_eq!(fs::read(&harness.raw).unwrap(), body);
            assert_eq!(harness.verified_receipt()["status"], "failed");
        }
    }

    #[tokio::test]
    async fn incomplete_exchange_never_fabricates_response_evidence() {
        let harness = Harness::new();
        let error = client()
            .await
            .fetch_using(42, &harness.paths(), |_request| async {
                Err(anyhow!("synthetic-private-transport-error"))
            })
            .await
            .unwrap_err();
        assert!(error.to_string().contains("no receipt was published"));
        assert!(!format!("{error:#}").contains("synthetic-private-transport-error"));
        harness.assert_absent();
    }

    #[tokio::test]
    async fn interrupted_body_never_publishes_a_partial_digest_as_complete() {
        let harness = Harness::new();
        let error = client()
            .await
            .fetch_using(42, &harness.paths(), |request| async move {
                let body = futures_util::stream::iter(vec![
                    Ok(b"partial response".to_vec()),
                    Err(std::io::Error::other("synthetic body interruption")),
                ]);
                let response = http::Response::builder()
                    .status(200)
                    .body(reqwest::Body::wrap_stream(body))?;
                Ok((response.into(), request.url().clone()))
            })
            .await
            .unwrap_err();
        assert!(error.to_string().contains("no receipt was published"));
        harness.assert_absent();
    }

    #[tokio::test]
    async fn configured_shared_pacing_is_applied_and_bound_into_receipt() {
        let db = DbPool::new_in_memory().await.unwrap();
        let client =
            client()
                .await
                .with_shared_pacing(db.clone(), "request-evidence-test", Duration::ZERO);
        let harness = Harness::new();
        client
            .fetch_using(42, &harness.paths(), |request| async move {
                response(request, 200, VALID, None)
            })
            .await
            .unwrap();
        let receipt = harness.verified_receipt();
        assert_eq!(receipt["rate_limit"]["name"], "request-evidence-test");
        assert_eq!(receipt["rate_limit"]["minimum_interval_seconds"], 0.0);
        let state = crate::shared_pacing::read_status(db.pool(), "request-evidence-test")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(state.recent_events.len(), 1);
        assert_eq!(state.recent_events[0].kind, "acquired");
    }

    #[tokio::test]
    async fn invalid_limiter_blocks_source_before_a_request_is_sent() {
        let db = DbPool::new_in_memory().await.unwrap();
        let client = client().await.with_shared_pacing(db, " ", Duration::ZERO);
        let harness = Harness::new();
        let error = client
            .fetch_using(42, &harness.paths(), |_request| async {
                panic!("invalid limiter must not send a request")
            })
            .await
            .unwrap_err();
        assert!(error.to_string().contains("rate limit name"));
        harness.assert_absent();
    }

    #[tokio::test]
    async fn invalid_id_or_existing_evidence_blocks_the_transport() {
        let harness = Harness::new();
        let client = client().await;
        for id in [0, -1] {
            assert!(client
                .fetch_using(id, &harness.paths(), |_request| async {
                    panic!("preflight must precede transport")
                })
                .await
                .is_err());
        }
        harness.assert_absent();
        fs::write(&harness.receipt, b"preserved evidence").unwrap();
        assert!(client
            .fetch_using(42, &harness.paths(), |_request| async {
                panic!("existing evidence must block transport")
            })
            .await
            .is_err());
        assert_eq!(fs::read(&harness.receipt).unwrap(), b"preserved evidence");
        assert!(!harness.raw.exists() && !harness.result.exists());
    }

    #[tokio::test]
    async fn unexpected_final_url_cannot_be_published_as_one_request() {
        let harness = Harness::new();
        let error = client()
            .await
            .fetch_using(42, &harness.paths(), |request| async move {
                let (response, _) = response(request, 200, VALID, None)?;
                Ok((
                    response,
                    Url::parse("https://source.example.test/redirected")?,
                ))
            })
            .await
            .unwrap_err();
        assert!(error.to_string().contains("URL differs"));
        harness.assert_absent();
    }

    #[tokio::test]
    async fn oversized_response_never_produces_truncated_success() {
        let harness = Harness::new();
        let error = client()
            .await
            .fetch_using(42, &harness.paths(), |request| async move {
                response(
                    request,
                    200,
                    &vec![b'x'; MAX_RESPONSE_BYTES as usize + 1],
                    None,
                )
            })
            .await
            .unwrap_err();
        assert!(error.to_string().contains("no receipt was published"));
        harness.assert_absent();
    }

    #[tokio::test]
    async fn evidence_created_during_acquisition_is_preserved_without_publishing_success() {
        let harness = Harness::new();
        let result = harness.result.clone();
        let error = client()
            .await
            .fetch_using(42, &harness.paths(), |request| async move {
                fs::write(result, b"racing evidence").unwrap();
                response(request, 200, VALID, None)
            })
            .await
            .unwrap_err();
        assert!(error
            .to_string()
            .contains("evidence changed during acquisition"));
        assert!(format!("{error:#}").contains("already exists"));
        assert!(!harness.raw.exists());
        assert_eq!(fs::read(&harness.result).unwrap(), b"racing evidence");
        assert!(!harness.receipt.exists());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn parent_replaced_with_symlink_during_response_cannot_redirect_evidence() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir_in(fs::canonicalize(std::env::temp_dir()).unwrap()).unwrap();
        let parent = root.path().join("evidence");
        let preserved = root.path().join("original-evidence");
        let substituted = root.path().join("substituted");
        fs::create_dir(&parent).unwrap();
        fs::create_dir(&substituted).unwrap();
        let raw = parent.join("response.raw");
        let result = parent.join("result.json");
        let receipt = parent.join("receipt.json");
        let paths = SyncPullEvidencePaths {
            raw: &raw,
            result: &result,
            receipt: &receipt,
        };
        let error = client()
            .await
            .fetch_using(42, &paths, |request| async {
                fs::rename(&parent, &preserved).unwrap();
                symlink(&substituted, &parent).unwrap();
                response(request, 200, VALID, None)
            })
            .await
            .unwrap_err();
        assert!(format!("{error:#}").contains("symlink"));
        assert_eq!(fs::read_dir(&substituted).unwrap().count(), 0);
        assert_eq!(fs::read_dir(&preserved).unwrap().count(), 0);
    }

    #[tokio::test]
    async fn legacy_evidence_refuses_opaque_transport_without_replacing_its_policy() {
        let http = Client::builder().no_proxy().build().unwrap();
        let client = SyncClient::with_http_client("https://source.example.test", http).unwrap();
        let error = client.fetch_request_evidence(42).await.unwrap_err();
        assert!(error.to_string().contains("custom HTTP client"));
        assert!(error.to_string().contains("RequestEvidenceClient"));
    }
}
