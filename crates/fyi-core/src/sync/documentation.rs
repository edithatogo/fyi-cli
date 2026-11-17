//! Explicit, bounded observations of the six reviewed platform documentation sources.
//!
//! This adapter uses the ordinary source-network guard and acquisition receipt
//! contract. It does not query FOI records, authenticate, or enable a provider.
//! Each invocation observes exactly one compiled URL. Raw HTTP entity bytes are
//! private evidence: consumers must not publish them without a separate rights
//! and privacy review. `observation.json` is the atomic completion marker; a
//! directory without that marker is incomplete and must not be consumed.

use super::SyncClient;
use crate::acquisition_receipts::{
    canonical_json_bytes, verify_receipt, verify_receipt_raw_bytes, verify_receipt_result_bytes,
    write_evidence_bytes_atomic, write_receipt_atomic,
};
use crate::agent_runtime::ClientIdentity;
use crate::provenance::sha256_hex;
use anyhow::{anyhow, ensure, Context, Result};
use chrono::{SecondsFormat, Utc};
use reqwest::{header, Client, StatusCode, Url};
use serde_json::{json, Value};
use std::fs;
use std::future::Future;
use std::net::{IpAddr, SocketAddr};
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

/// Maximum original, identity-encoded HTTP entity length for one document.
pub const MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
/// Bound covering name resolution, pacing, transport, and response streaming.
pub const EXCHANGE_TIMEOUT: Duration = Duration::from_secs(30);

/// A reviewed documentation target, not a configurable provider URL.
#[derive(Debug, Clone, Copy)]
pub struct DocumentationSource {
    pub source_id: &'static str,
    pub platform_id: &'static str,
    pub url: &'static str,
    media_types: &'static [&'static str],
    markers: &'static [&'static str],
}

/// Exact public documentation URLs approved for this version of the adapter.
/// A new URL or document version requires a reviewed source change.
pub const SOURCES: &[DocumentationSource] = &[
    DocumentationSource {
        source_id: "muckrock_client",
        platform_id: "muckrock",
        url: "https://raw.githubusercontent.com/MuckRock/python-muckrock/main/docs/requests.rst",
        media_types: &["text/plain"],
        markers: &["muckrock.requests.RequestClient"],
    },
    DocumentationSource {
        source_id: "froide_api",
        platform_id: "fragdenstaat",
        url: "https://raw.githubusercontent.com/okfde/froide/main/docs/api.rst",
        media_types: &["text/plain"],
        markers: &["Froide API"],
    },
    DocumentationSource {
        source_id: "foia_developer",
        platform_id: "foia_gov",
        url: "https://www.foia.gov/developer/",
        media_types: &["text/html"],
        markers: &["FOIA.gov", "Developer resources"],
    },
    DocumentationSource {
        source_id: "uscis_openapi",
        platform_id: "uscis",
        url: "https://developer.uscis.gov/sites/default/files/2026-10/swagger.yaml",
        media_types: &[
            "application/octet-stream", "application/yaml", "application/x-yaml",
            "text/yaml", "text/plain",
        ],
        markers: &["openapi:", "FOIA/PA", "paths:"],
    },
    DocumentationSource {
        source_id: "canada_guidance",
        platform_id: "canada_atip",
        url: "https://www.canada.ca/en/treasury-board-secretariat/services/access-information-privacy/access-information/request-information/make-request.html",
        media_types: &["text/html"],
        markers: &["Make a request", "Access to Information"],
    },
    DocumentationSource {
        source_id: "india_faq",
        platform_id: "india_rti",
        url: "https://rtionline.gov.in/faq.php",
        media_types: &["text/html"],
        markers: &["RTI Online", "Frequently Asked Questions"],
    },
];

fn source_by_id(source_id: &str) -> Result<&'static DocumentationSource> {
    SOURCES
        .iter()
        .find(|source| source.source_id == source_id)
        .context("unknown documentation source")
}

fn source_url(source: &DocumentationSource) -> Result<Url> {
    // String equality with the compiled registry prevents URL parser
    // normalization from turning an unreviewed input into an approved target.
    let registered = source_by_id(source.source_id)?;
    ensure!(
        registered.url == source.url && registered.platform_id == source.platform_id,
        "documentation source differs from the reviewed registry"
    );
    let url = Url::parse(source.url).context("invalid compiled documentation URL")?;
    ensure!(
        url.as_str() == source.url
            && url.scheme() == "https"
            && url.host_str().is_some()
            && url.port_or_known_default() == Some(443)
            && url.username().is_empty()
            && url.password().is_none()
            && url.query().is_none()
            && url.fragment().is_none(),
        "documentation URL must be exact credential-free HTTPS"
    );
    Ok(url)
}

fn public_address(address: SocketAddr) -> bool {
    if address.port() != 443 || !super::is_public_ip(address.ip()) {
        return false;
    }
    match address.ip() {
        IpAddr::V4(ip) => {
            let [a, b, c, _] = ip.octets();
            !ip.is_multicast()
                && !(a == 192 && b == 0 && matches!(c, 0 | 2))
                && !(a == 192 && b == 88 && c == 99)
        }
        IpAddr::V6(ip) => {
            // Restrict pinned IPv6 targets to ordinary global unicast. This
            // excludes mapped IPv4, transition/local ranges and documentation.
            let [a, b, ..] = ip.segments();
            (a & 0xe000) == 0x2000
                && a != 0x2002
                && a != 0x3fff
                && !(a == 0x2001 && (b < 0x0200 || b == 0x0db8))
        }
    }
}

fn pinned_http(identity: &ClientIdentity, host: &str, addresses: &[SocketAddr]) -> Result<Client> {
    ensure!(
        !addresses.is_empty() && addresses.iter().copied().all(public_address),
        "documentation DNS did not resolve exclusively to public addresses"
    );
    identity.validate()?;
    Client::builder()
        .user_agent(identity.user_agent())
        .https_only(true)
        .no_proxy()
        .referer(false)
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .no_gzip()
        .no_brotli()
        .no_deflate()
        .no_zstd()
        .timeout(EXCHANGE_TIMEOUT)
        .resolve_to_addrs(host, addresses)
        .build()
        .context("cannot construct documentation source transport")
}

struct DocumentationClient {
    source: &'static DocumentationSource,
    client: SyncClient,
}

#[derive(Debug)]
struct CapturedDocument {
    body: Vec<u8>,
    media_type: String,
    retrieved_at: String,
}

impl DocumentationClient {
    async fn new(source: &'static DocumentationSource) -> Result<Self> {
        let url = source_url(source)?;
        let host = url.host_str().context("documentation hostname missing")?;
        let addresses = tokio::net::lookup_host((host, 443))
            .await
            .map_err(|_| anyhow!("documentation name resolution failed"))?
            .collect::<Vec<_>>();
        let identity = ClientIdentity::default_identity(None)?;
        let http = pinned_http(&identity, host, &addresses)?;
        Ok(Self {
            source,
            client: SyncClient::with_http_client_and_identity(source.url, http, identity)?,
        })
    }

    async fn fetch(&self) -> Result<CapturedDocument> {
        self.fetch_using(EXCHANGE_TIMEOUT, MAX_RESPONSE_BYTES, |request| {
            self.client.execute_guarded_request(request)
        })
        .await
    }

    async fn fetch_using<F, Fut>(
        &self,
        timeout: Duration,
        max_bytes: usize,
        execute: F,
    ) -> Result<CapturedDocument>
    where
        F: FnOnce(reqwest::Request) -> Fut,
        Fut: Future<Output = Result<(reqwest::Response, Url)>>,
    {
        let url = source_url(self.source)?;
        ensure!(
            !timeout.is_zero()
                && timeout <= EXCHANGE_TIMEOUT
                && (1..=MAX_RESPONSE_BYTES).contains(&max_bytes),
            "documentation exchange bounds are invalid"
        );
        let request = self
            .client
            .http
            .get(url.clone())
            .header(header::ACCEPT_ENCODING, "identity")
            .header(header::ACCEPT, self.source.media_types.join(", "))
            .timeout(timeout);
        let response = tokio::time::timeout(
            timeout,
            self.client.send_guarded_with_limits_using(
                request,
                Some(max_bytes as u64),
                false,
                "platform_documentation",
                execute,
            ),
        )
        .await
        .map_err(|_| anyhow!("documentation exchange timed out"))?
        .map_err(|_| anyhow!("documentation source exchange failed"))?;
        ensure!(
            response.final_url == url,
            "documentation source URL changed"
        );
        ensure!(
            response.source_status == StatusCode::OK,
            "documentation source did not return HTTP 200"
        );
        ensure!(
            !response.headers.contains_key("deprecation")
                && !response.headers.contains_key("sunset"),
            "documentation source advertises deprecation"
        );
        ensure!(
            !response.body.is_empty(),
            "documentation source body is empty"
        );
        if self.source.source_id == "uscis_openapi" {
            ensure!(
                std::str::from_utf8(&response.body).is_ok(),
                "documentation schema must be UTF-8 text"
            );
        }
        for value in response.headers.get_all(header::CONTENT_ENCODING) {
            ensure!(
                value.to_str().is_ok_and(|encoding| encoding
                    .split(',')
                    .all(|part| part.trim().eq_ignore_ascii_case("identity"))),
                "documentation response encoding is unsupported"
            );
        }
        let content_types = response
            .headers
            .get_all(header::CONTENT_TYPE)
            .iter()
            .collect::<Vec<_>>();
        ensure!(
            content_types.len() == 1,
            "documentation media type is missing or ambiguous"
        );
        let media_type = content_types[0]
            .to_str()
            .context("documentation media type is invalid")?
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        ensure!(
            self.source.media_types.contains(&media_type.as_str()),
            "documentation media type is unsupported"
        );
        let lengths = response
            .headers
            .get_all(header::CONTENT_LENGTH)
            .iter()
            .collect::<Vec<_>>();
        ensure!(
            lengths.len() <= 1,
            "documentation content length is ambiguous"
        );
        if let Some(length) = lengths.first() {
            let expected = length
                .to_str()
                .ok()
                .and_then(|value| value.parse::<usize>().ok());
            ensure!(
                expected == Some(response.body.len()),
                "documentation content length mismatch"
            );
        }
        ensure!(
            self.source.markers.iter().all(|marker| {
                response
                    .body
                    .windows(marker.len())
                    .any(|window| window == marker.as_bytes())
            }),
            "documentation content does not identify the reviewed document"
        );
        Ok(CapturedDocument {
            body: response.body,
            media_type,
            retrieved_at: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
        })
    }
}

fn reserve_output(output: &Path) -> Result<PathBuf> {
    ensure!(
        output.file_name().is_some()
            && !output
                .components()
                .any(|component| matches!(component, Component::ParentDir)),
        "documentation output must name a fresh directory without parent traversal"
    );
    let absolute = if output.is_absolute() {
        output.to_owned()
    } else {
        std::env::current_dir()?.join(output)
    };
    for ancestor in absolute.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(metadata) => ensure!(
                !metadata.file_type().is_symlink(),
                "documentation evidence path contains a symlink"
            ),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(anyhow!("cannot inspect documentation evidence path")),
        }
    }
    // create_dir is exclusive, including against dangling links and an empty
    // raced destination. Do not create or replace caller-owned parent paths.
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder
        .create(&absolute)
        .map_err(|_| anyhow!("documentation output already exists or cannot be reserved"))?;
    Ok(absolute)
}

fn publish_document(
    source: &DocumentationSource,
    output: &Path,
    started_at: &str,
    captured: CapturedDocument,
) -> Result<Value> {
    let observation = json!({
        "schema_version": "1.0.0", "adapter": "platform-documentation", "adapter_version": "1.0.0",
        "source_id": source.source_id, "platform_id": source.platform_id, "source_url": source.url,
        "retrieved_at": captured.retrieved_at, "http_status": 200, "media_type": captured.media_type,
        "body_bytes": captured.body.len(), "body_sha256": sha256_hex(&captured.body),
        "body_path": "body.bin", "receipt_path": "receipt.json",
    });
    let result = canonical_json_bytes(&observation)?;
    let mut receipt = json!({
        "schema": "urn:fyi-cli:acquisition-receipt", "schema_version": "1.0.0",
        "command": "observe-platform-docs",
        "adapter": {"id": "platform-documentation", "version": "1.0.0"},
        "source": {"url": source.url},
        "request_bounds": {"source_id": source.source_id, "max_requests": 1,
            "max_response_bytes": MAX_RESPONSE_BYTES, "timeout_seconds": EXCHANGE_TIMEOUT.as_secs(),
            "redirects": 0, "retries": 0},
        "started_at": started_at, "completed_at": observation["retrieved_at"], "status": "succeeded",
        "responses": [{"url": source.url, "status": 200, "bytes": captured.body.len(),
            "sha256": observation["body_sha256"], "attempts": 1, "retry_delays_seconds": []}],
        "totals": {"requests": 1, "bytes": captured.body.len(), "retries": 0},
        "rate_limit": {"name": null, "minimum_interval_seconds": null},
        "checkpoint": {"before_sha256": null, "after_sha256": null},
        "result": {"representation": "canonical_result_projection", "media_type": "application/json",
            "bytes": result.len(), "sha256": sha256_hex(&result)},
    });
    receipt["receipt_sha256"] = json!(sha256_hex(&canonical_json_bytes(&receipt)?));
    verify_receipt(&receipt)?;
    verify_receipt_raw_bytes(&receipt, &captured.body)?;
    verify_receipt_result_bytes(&receipt, &result)?;
    let staging = tempfile::Builder::new()
        .prefix(".pending-")
        .tempdir_in(output)?;
    write_evidence_bytes_atomic(&staging.path().join("body.bin"), &captured.body)?;
    write_receipt_atomic(&staging.path().join("receipt.json"), &receipt)?;
    // Links fail if any destination was created after reservation. No existing
    // byte, receipt or completion marker is ever replaced, even when identical.
    for name in ["body.bin", "receipt.json"] {
        fs::hard_link(staging.path().join(name), output.join(name))
            .map_err(|_| anyhow!("cannot publish immutable documentation evidence"))?;
    }
    #[cfg(unix)]
    fs::File::open(output)?.sync_all()?;
    write_evidence_bytes_atomic(&staging.path().join("observation.json"), &result)?;
    fs::hard_link(
        staging.path().join("observation.json"),
        output.join("observation.json"),
    )
    .map_err(|_| anyhow!("cannot publish documentation completion marker"))?;
    #[cfg(unix)]
    fs::File::open(output)?.sync_all()?;
    Ok(observation)
}

/// Observe exactly one public documentation source after explicit opt-in.
///
/// The caller must supply a fresh directory under an existing, non-symlink
/// parent. The operation never takes credentials or a caller-provided URL. DNS
/// resolution is checked and pinned to public addresses; redirects, proxies,
/// retries, cookies, decompression and cache reuse are disabled. A failure can
/// leave an incomplete private directory but never publishes a completion marker
/// unless both body and receipt have already been written and verified.
pub async fn observe_documentation(source_id: &str, output: &Path, live: bool) -> Result<Value> {
    ensure!(
        live,
        "documentation retrieval requires explicit live authorization"
    );
    let source = source_by_id(source_id)?;
    source_url(source)?;
    let output = reserve_output(output)?;
    let started_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    let captured = tokio::time::timeout(EXCHANGE_TIMEOUT, async {
        DocumentationClient::new(source).await?.fetch().await
    })
    .await
    .map_err(|_| anyhow!("documentation acquisition deadline exceeded"))??;
    publish_document(source, &output, &started_at, captured)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    async fn fixture_for_source(source: &'static DocumentationSource) -> DocumentationClient {
        let identity = ClientIdentity::default_identity(None).unwrap();
        let url = source_url(source).unwrap();
        let http = pinned_http(
            &identity,
            url.host_str().unwrap(),
            &["185.199.108.133:443".parse().unwrap()],
        )
        .unwrap();
        let client = SyncClient::with_http_client_and_identity(source.url, http, identity).unwrap();
        client.middleware.lock().await.pacing.current_delay = Duration::ZERO;
        DocumentationClient { source, client }
    }

    async fn fixture_client() -> DocumentationClient {
        fixture_for_source(&SOURCES[0]).await
    }

    fn response(
        request: reqwest::Request,
        status: u16,
        content_type: &str,
        body: Vec<u8>,
    ) -> Result<(reqwest::Response, Url)> {
        Ok((
            http::Response::builder()
                .status(status)
                .header(header::CONTENT_TYPE, content_type)
                .body(reqwest::Body::from(body))
                .unwrap()
                .into(),
            request.url().clone(),
        ))
    }

    fn document() -> Vec<u8> {
        b"Synthetic documentation\n.. class:: muckrock.requests.RequestClient\n".to_vec()
    }

    #[test]
    fn registry_has_six_distinct_exact_credential_free_sources() {
        let ids = SOURCES
            .iter()
            .map(|s| s.source_id)
            .collect::<std::collections::BTreeSet<_>>();
        let platforms = SOURCES
            .iter()
            .map(|s| s.platform_id)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(ids.len(), 6);
        assert_eq!(platforms.len(), 6);
        for source in SOURCES {
            source_url(source).unwrap();
        }
        let mut changed = SOURCES[0];
        changed.url = "https://raw.githubusercontent.com/MuckRock/private/main/accounts.json";
        assert!(source_url(&changed).is_err());
        assert!(source_by_id("https://example.invalid/").is_err());
    }

    #[test]
    fn dns_pinning_refuses_private_mixed_empty_and_non_https_targets() {
        let identity = ClientIdentity::default_identity(None).unwrap();
        assert!(pinned_http(&identity, "raw.githubusercontent.com", &[]).is_err());
        for address in [
            "127.0.0.1:443",
            "10.1.2.3:443",
            "169.254.169.254:443",
            "192.168.0.1:443",
            "100.64.0.1:443",
            "192.0.2.1:443",
            "239.1.2.3:443",
            "[::1]:443",
            "[fc00::1]:443",
            "[::ffff:127.0.0.1]:443",
            "[2001:db8::1]:443",
            "[2002:7f00:1::]:443",
            "185.199.108.133:80",
        ] {
            let addresses = [
                "185.199.108.133:443".parse().unwrap(),
                address.parse().unwrap(),
            ];
            assert!(
                pinned_http(&identity, "raw.githubusercontent.com", &addresses).is_err(),
                "{address}"
            );
        }
        assert!(public_address("[2606:50c0:8000::154]:443".parse().unwrap()));
    }

    #[tokio::test]
    async fn live_and_source_preflight_fail_without_output_or_network() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("observation");
        assert!(observe_documentation("muckrock_client", &output, false)
            .await
            .is_err());
        assert!(observe_documentation("unknown", &output, true)
            .await
            .is_err());
        assert!(!output.exists());
        fs::create_dir(&output).unwrap();
        fs::write(output.join("prior"), b"preserved").unwrap();
        assert!(observe_documentation("muckrock_client", &output, true)
            .await
            .is_err());
        assert_eq!(fs::read(output.join("prior")).unwrap(), b"preserved");
        assert_eq!(fs::read_dir(&output).unwrap().count(), 1);
    }

    #[tokio::test]
    async fn guarded_exchange_keeps_original_bytes_and_has_no_ambient_or_cached_headers() {
        let client = fixture_client().await;
        let calls = AtomicUsize::new(0);
        for _ in 0..2 {
            let observed = client
                .fetch_using(Duration::from_secs(1), 1024, |request| {
                    calls.fetch_add(1, Ordering::SeqCst);
                    async move {
                        assert_eq!(request.method(), reqwest::Method::GET);
                        assert_eq!(request.url().as_str(), SOURCES[0].url);
                        assert_eq!(request.headers()[header::ACCEPT_ENCODING], "identity");
                        for name in [
                            "authorization",
                            "proxy-authorization",
                            "cookie",
                            "if-none-match",
                            "if-modified-since",
                            "x-fyi-bot-token",
                            "referer",
                        ] {
                            assert!(!request.headers().contains_key(name));
                        }
                        let mut result =
                            response(request, 200, "text/plain; charset=utf-8", document())?;
                        result
                            .0
                            .headers_mut()
                            .insert(header::ETAG, "\"old\"".parse().unwrap());
                        Ok(result)
                    }
                })
                .await
                .unwrap();
            assert_eq!(observed.body, document());
            assert_eq!(observed.media_type, "text/plain");
        }
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn redirects_non_success_cache_and_wrong_document_are_not_observations() {
        for (status, media, body) in [
            (302, "text/plain", document()),
            (304, "text/plain", document()),
            (404, "text/plain", document()),
            (410, "text/plain", document()),
            (503, "text/plain", document()),
            (200, "application/json", document()),
            (200, "text/plain", b"sign in to proceed".to_vec()),
            (200, "text/plain", Vec::new()),
        ] {
            let client = fixture_client().await;
            let calls = AtomicUsize::new(0);
            assert!(
                client
                    .fetch_using(Duration::from_secs(1), 1024, |request| {
                        calls.fetch_add(1, Ordering::SeqCst);
                        async move { response(request, status, media, body) }
                    })
                    .await
                    .is_err(),
                "{status} {media}"
            );
            assert_eq!(calls.load(Ordering::SeqCst), 1);
        }
    }

    #[tokio::test]
    async fn encoding_response_url_and_length_changes_are_rejected() {
        for mutation in [
            "encoding",
            "url",
            "length",
            "multiple_media",
            "deprecation",
            "sunset",
        ] {
            let client = fixture_client().await;
            assert!(
                client
                    .fetch_using(Duration::from_secs(1), 1024, |request| async move {
                        let (mut received, mut url) =
                            response(request, 200, "text/plain", document())?;
                        match mutation {
                            "encoding" => {
                                received
                                    .headers_mut()
                                    .insert(header::CONTENT_ENCODING, "gzip".parse().unwrap());
                            }
                            "url" => {
                                url = Url::parse("https://example.invalid/login").unwrap();
                            }
                            "length" => {
                                received
                                    .headers_mut()
                                    .insert(header::CONTENT_LENGTH, "999".parse().unwrap());
                            }
                            "multiple_media" => {
                                received
                                    .headers_mut()
                                    .append(header::CONTENT_TYPE, "text/html".parse().unwrap());
                            }
                            "deprecation" | "sunset" => {
                                received.headers_mut().insert(
                                    header::HeaderName::from_static(mutation),
                                    "upstream marker".parse().unwrap(),
                                );
                            }
                            _ => unreachable!(),
                        }
                        Ok((received, url))
                    })
                    .await
                    .is_err(),
                "{mutation}"
            );
        }
    }

    #[tokio::test]
    async fn every_compiled_source_accepts_its_document_media_and_preserves_bytes() {
        for source in SOURCES {
            let client = fixture_for_source(source).await;
            let body =
                format!("Synthetic documentation\n{}\n", source.markers.join("\n")).into_bytes();
            let expected = body.clone();
            let media = format!("{}; charset=utf-8", source.media_types[0].to_uppercase());
            let captured = client
                .fetch_using(Duration::from_secs(1), 1024, |request| async move {
                    response(request, 200, &media, body)
                })
                .await
                .unwrap();
            assert_eq!(captured.body, expected);
            assert_eq!(captured.media_type, source.media_types[0]);
        }
        let source = source_by_id("uscis_openapi").unwrap();
        let client = fixture_for_source(source).await;
        let mut body = source.markers.join("\n").into_bytes();
        body.push(0xff);
        assert!(client
            .fetch_using(Duration::from_secs(1), 1024, |request| async move {
                response(request, 200, "application/octet-stream", body)
            })
            .await
            .is_err());
    }

    #[tokio::test]
    async fn response_stream_is_bounded_and_incomplete_or_slow_streams_fail() {
        let client = fixture_client().await;
        assert!(client
            .fetch_using(Duration::from_secs(1), 10, |request| async move {
                let chunks = futures_util::stream::iter(vec![
                    Ok::<_, std::io::Error>(b"123456".to_vec()),
                    Ok(b"789012".to_vec()),
                ]);
                let url = request.url().clone();
                Ok((
                    http::Response::builder()
                        .status(200)
                        .header(header::CONTENT_TYPE, "text/plain")
                        .body(reqwest::Body::wrap_stream(chunks))
                        .unwrap()
                        .into(),
                    url,
                ))
            })
            .await
            .is_err());
        let client = fixture_client().await;
        assert!(client
            .fetch_using(Duration::from_secs(1), 1024, |request| async move {
                let chunks = futures_util::stream::iter(vec![
                    Ok(document()),
                    Err(std::io::Error::other("SECRET")),
                ]);
                let url = request.url().clone();
                Ok((
                    http::Response::builder()
                        .status(200)
                        .header(header::CONTENT_TYPE, "text/plain")
                        .body(reqwest::Body::wrap_stream(chunks))
                        .unwrap()
                        .into(),
                    url,
                ))
            })
            .await
            .unwrap_err()
            .to_string()
            .contains("source exchange failed"));
        let client = fixture_client().await;
        assert!(client
            .fetch_using(Duration::from_millis(10), 1024, |_request| {
                std::future::pending::<Result<(reqwest::Response, Url)>>()
            })
            .await
            .unwrap_err()
            .to_string()
            .contains("timed out"));
    }

    #[tokio::test]
    async fn invalid_limits_never_reach_transport() {
        let client = fixture_client().await;
        for (timeout, bytes) in [
            (Duration::ZERO, 10),
            (Duration::from_secs(31), 10),
            (Duration::from_secs(1), 0),
            (Duration::from_secs(1), MAX_RESPONSE_BYTES + 1),
        ] {
            assert!(client
                .fetch_using(timeout, bytes, |_| async {
                    panic!("invalid bounds must not contact source")
                })
                .await
                .is_err());
        }
    }

    #[test]
    fn publication_binds_body_and_observation_bytes_and_never_overwrites() {
        let dir = tempfile::tempdir().unwrap();
        let output = reserve_output(&dir.path().join("captured")).unwrap();
        let observed = publish_document(
            &SOURCES[0],
            &output,
            "2026-11-17T00:00:00.000Z",
            CapturedDocument {
                body: document(),
                media_type: "text/plain".into(),
                retrieved_at: "2026-11-17T00:00:01.000Z".into(),
            },
        )
        .unwrap();
        let receipt: Value =
            serde_json::from_slice(&fs::read(output.join("receipt.json")).unwrap()).unwrap();
        let body = fs::read(output.join("body.bin")).unwrap();
        let result = fs::read(output.join("observation.json")).unwrap();
        verify_receipt_raw_bytes(&receipt, &body).unwrap();
        verify_receipt_result_bytes(&receipt, &result).unwrap();
        assert_eq!(serde_json::from_slice::<Value>(&result).unwrap(), observed);
        let mut tampered = body.clone();
        tampered.push(b'!');
        assert!(verify_receipt_raw_bytes(&receipt, &tampered).is_err());
        assert!(verify_receipt_result_bytes(&receipt, b"{}").is_err());
        assert!(reserve_output(&output).is_err());
        assert_eq!(fs::read_dir(&output).unwrap().count(), 3);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&output).unwrap().permissions().mode() & 0o777,
                0o700
            );
            assert_eq!(
                fs::metadata(output.join("body.bin"))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }
    }

    #[test]
    fn publication_failure_leaves_no_completion_marker_or_replaced_evidence() {
        let dir = tempfile::tempdir().unwrap();
        let output = reserve_output(&dir.path().join("captured")).unwrap();
        fs::write(output.join("receipt.json"), b"prior").unwrap();
        assert!(publish_document(
            &SOURCES[0],
            &output,
            "2026-11-17T00:00:00.000Z",
            CapturedDocument {
                body: document(),
                media_type: "text/plain".into(),
                retrieved_at: "2026-11-17T00:00:01.000Z".into()
            }
        )
        .is_err());
        assert_eq!(fs::read(output.join("receipt.json")).unwrap(), b"prior");
        assert!(!output.join("observation.json").exists());
        assert!(!fs::read_dir(&output).unwrap().any(|entry| entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".pending-")));
    }

    #[test]
    fn output_preflight_rejects_parent_traversal_and_symlinks() {
        let dir = tempfile::tempdir().unwrap();
        assert!(reserve_output(&dir.path().join("child/../capture")).is_err());
        #[cfg(unix)]
        {
            let link = dir.path().join("alias");
            std::os::unix::fs::symlink(dir.path(), &link).unwrap();
            assert!(reserve_output(&link.join("capture")).is_err());
            let dangling = dir.path().join("dangling");
            std::os::unix::fs::symlink(dir.path().join("absent"), &dangling).unwrap();
            assert!(reserve_output(&dangling).is_err());
            assert!(!dir.path().join("capture").exists());
        }
    }
}
