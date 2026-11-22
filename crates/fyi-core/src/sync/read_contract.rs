//! Bounded, read-only source contracts shared with the MCP adapter.
//!
//! These methods use the existing source transport and validator cache. They
//! never open an application database and never follow a feed's links.
//! Construct the client with [`SyncClient::with_guardrails`] for exact wire
//! request bounds: legacy constructors retain their existing redirect policy.

use super::*;
use serde::Serialize;
use std::collections::BTreeMap;

/// Selected fields returned by the pinned upstream authority API. The
/// controller also returns authority-only fields that this read does not retain.
#[derive(serde::Deserialize)]
struct UpstreamAuthorityRequest {
    id: i64,
    title: String,
    request_text: String,
    url: String,
    status: Option<String>,
    created_at: Option<String>,
    updated_at: Option<String>,
}

/// Per-operation transfer limits. Request counts and aggregate transfer bytes
/// are additionally bounded by the client's shared `GuardrailConfig`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadLimits {
    pub max_bytes: u64,
    /// Guarded exchange time, including pacing, permit acquisition and body
    /// transfer. Callers enforce their own end-to-end deadline for parsing and
    /// result serialization after that exchange.
    pub max_duration: Duration,
}

/// The source exchange's cache behavior. Revalidation always performs one GET.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CacheStatus {
    Miss,
    Revalidated,
}

/// Public, non-secret provenance for one complete source exchange.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReadProvenance {
    /// Original representation retrieval time, retained when a 304 revalidates it.
    pub retrieved_at: String,
    pub cache_status: CacheStatus,
    /// Actual response status, including 304 when cached bytes were reused.
    pub source_status: u16,
    /// Downloaded response entity bytes, excluding a reused cached representation.
    pub response_bytes: u64,
}

/// A parsed source value and its matching response provenance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SourceRead<T> {
    pub value: T,
    pub provenance: ReadProvenance,
}

/// The public source route retained by a feed plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorityFeedReferenceKind {
    Request,
    RequestEvent,
}

/// A source reference. Only the explicit API request route carries a request
/// ID. Public request URL titles (including all-digit titles) and event IDs
/// remain unresolved; this planner never guesses IDs or follows those links.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AuthorityFeedReference {
    pub kind: AuthorityFeedReferenceKind,
    pub url: String,
    pub request_id: Option<i64>,
}

/// References discovered in one authority feed. Planning never resolves these
/// references, fetches request records, or applies them to a database.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AuthorityFeedPlan {
    pub feed_url: String,
    /// Unique normalized URLs in lexical order, subject to one shared item bound.
    pub references: Vec<AuthorityFeedReference>,
    /// Sorted, unique API request IDs from the retained references only.
    pub request_ids: Vec<i64>,
    /// More unique, valid references were present than the caller's item limit.
    pub truncated: bool,
}

/// A fixed-vocabulary failure with no source text, URL, body, or header values.
#[derive(Debug, Clone, Copy, thiserror::Error)]
#[error("source read failed ({kind:?})")]
pub struct SourceReadError {
    kind: SourceFailureKind,
}

impl SourceReadError {
    pub fn new(kind: SourceFailureKind) -> Self {
        Self { kind }
    }

    pub fn kind(&self) -> SourceFailureKind {
        self.kind
    }
}

impl GuardedResponse {
    fn read_provenance(&self) -> ReadProvenance {
        ReadProvenance {
            retrieved_at: self.retrieved_at.clone(),
            cache_status: if self.source_status == StatusCode::NOT_MODIFIED {
                CacheStatus::Revalidated
            } else {
                CacheStatus::Miss
            },
            source_status: self.source_status.as_u16(),
            response_bytes: self.response_bytes,
        }
    }

    fn into_source_read<T>(self, value: T) -> SourceRead<T> {
        SourceRead {
            provenance: self.read_provenance(),
            value,
        }
    }
}

fn malformed(_error: impl Into<anyhow::Error>) -> anyhow::Error {
    // Parser errors can include upstream string values or XML element names.
    // The bounded read contract intentionally retains only the classification.
    SourceReadError::new(SourceFailureKind::MalformedPayload).into()
}

fn safe_source_error(error: anyhow::Error) -> anyhow::Error {
    SourceReadError::new(source_failure_kind(&error).unwrap_or(SourceFailureKind::Transport)).into()
}

fn build_read_http_client(identity: &crate::agent_runtime::ClientIdentity) -> Result<Client> {
    identity.validate()?;
    Ok(Client::builder()
        .user_agent(identity.user_agent())
        .timeout(Duration::from_secs(60))
        // Redirects and implicit transport retries would create additional
        // wire requests after a single shared-budget admission.
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .build()?)
}

impl SyncClient {
    /// Construct a client with an explicit identity and shared source limits.
    /// `max_runtime` applies to the lifetime of this client; per-operation time
    /// bounds are supplied separately through `ReadLimits`.
    pub fn with_guardrails(
        base_url: &str,
        identity: &crate::agent_runtime::ClientIdentity,
        guardrails: crate::agent_runtime::GuardrailConfig,
    ) -> Result<Self> {
        if guardrails.max_requests == 0
            || guardrails.max_response_bytes == 0
            || guardrails.max_runtime.is_zero()
            || guardrails.max_concurrency == 0
        {
            return Err(SourceReadError::new(SourceFailureKind::Validation).into());
        }
        identity.validate()?;
        let http = build_read_http_client(identity)?;
        let evidence_http = super::request_evidence::capture_http(identity)
            .map_err(|error| error.context(SourceReadError::new(SourceFailureKind::Validation)))?;
        let mut client = Self::with_http_client_identity_and_guardrails(
            base_url,
            http,
            identity.clone(),
            guardrails,
        )
        .map_err(|error| error.context(SourceReadError::new(SourceFailureKind::Validation)))?;
        client.evidence_http = Some(evidence_http);
        Ok(client)
    }

    async fn read_response(
        &self,
        request: RequestBuilder,
        limits: &ReadLimits,
    ) -> Result<GuardedResponse> {
        if limits.max_bytes == 0 || limits.max_duration.is_zero() {
            return Err(SourceReadError::new(SourceFailureKind::Validation).into());
        }
        let budget = self.middleware.lock().await.guardrails.snapshot();
        let remaining =
            Duration::from_millis(budget.max_runtime_ms.saturating_sub(budget.elapsed_ms));
        if remaining.is_zero() {
            return Err(SourceReadError::new(SourceFailureKind::Guardrail).into());
        }
        let response = tokio::time::timeout(
            limits.max_duration.min(remaining),
            self.send_guarded_with_limits(request, Some(limits.max_bytes), true, "alaveteli_api"),
        )
        .await
        .map_err(|_| SourceReadError::new(SourceFailureKind::Timeout))?
        .map_err(safe_source_error)?;
        ensure_success(response, "read endpoint")
            .await
            .map_err(safe_source_error)
    }

    /// Strict health probe retaining typed source and transport failures. It
    /// performs one GET when the client was built with `with_guardrails`.
    pub async fn health_check_read(&self, limits: &ReadLimits) -> Result<SourceRead<SyncHealth>> {
        let url = self.base_url.join("api/v2/request.json")?;
        let response = self.read_response(self.http.get(url), limits).await?;
        Ok(response.into_source_read(SyncHealth {
            network_reachable: true,
            api_reachable: true,
        }))
    }

    /// Fetch the version through the shared, bounded source transport.
    pub async fn get_api_version_read(&self, limits: &ReadLimits) -> Result<SourceRead<String>> {
        let url = self.base_url.join("api/v2/version.json")?;
        let response = self.read_response(self.http.get(url), limits).await?;
        let value: Value = serde_json::from_slice(&response.body).map_err(malformed)?;
        let value = parse_api_version(value).map_err(malformed)?;
        Ok(response.into_source_read(value))
    }

    /// Fetch one positive request ID; a mismatched upstream ID is malformed.
    pub async fn fetch_request_read(
        &self,
        request_id: i64,
        limits: &ReadLimits,
    ) -> Result<SourceRead<AlaveteliRequest>> {
        if request_id <= 0 {
            return Err(SourceReadError::new(SourceFailureKind::Validation).into());
        }
        let url = self
            .base_url
            .join(&format!("api/v2/request/{request_id}.json"))?;
        let response = self.read_response(self.http.get(url), limits).await?;
        let value: AlaveteliRequest = serde_json::from_slice(&response.body).map_err(malformed)?;
        if value.id != request_id {
            return Err(SourceReadError::new(SourceFailureKind::MalformedPayload).into());
        }
        Ok(response.into_source_read(value))
    }

    /// Read one request through the pinned upstream *authority* API profile.
    ///
    /// The caller explicitly supplies the authority's key; this is not an
    /// ordinary requester session. The dedicated no-redirect transport is
    /// required, and no validator cache or raw authority response is retained.
    /// Compatibility with an individual deployment must be qualified separately.
    pub async fn fetch_upstream_authority_request_read(
        &self,
        request_id: i64,
        authority_key: &str,
        limits: &ReadLimits,
    ) -> Result<SourceRead<AlaveteliRequest>> {
        let http = self
            .evidence_http
            .as_ref()
            .ok_or_else(|| SourceReadError::new(SourceFailureKind::Validation))?;
        self.authority_request_read_using(request_id, authority_key, limits, |request| async move {
            let response = http
                .execute(request)
                .await
                .map_err(|error| error.without_url())?;
            let url = response.url().clone();
            Ok((response, url))
        })
        .await
    }

    async fn authority_request_read_using<F, Fut>(
        &self,
        request_id: i64,
        authority_key: &str,
        limits: &ReadLimits,
        execute: F,
    ) -> Result<SourceRead<AlaveteliRequest>>
    where
        F: FnOnce(reqwest::Request) -> Fut,
        Fut: std::future::Future<Output = Result<(reqwest::Response, Url)>>,
    {
        if request_id <= 0
            || authority_key.is_empty()
            || authority_key.trim() != authority_key
            || authority_key.chars().any(char::is_control)
            || limits.max_bytes == 0
            || limits.max_duration.is_zero()
        {
            return Err(SourceReadError::new(SourceFailureKind::Validation).into());
        }
        let budget = self.middleware.lock().await.guardrails.snapshot();
        let remaining =
            Duration::from_millis(budget.max_runtime_ms.saturating_sub(budget.elapsed_ms));
        if remaining.is_zero() {
            return Err(SourceReadError::new(SourceFailureKind::Guardrail).into());
        }
        let mut url = self
            .base_url
            .join(&format!("api/v2/request/{request_id}.json"))
            .map_err(|_| SourceReadError::new(SourceFailureKind::Validation))?;
        url.query_pairs_mut().append_pair("k", authority_key);
        let request = self
            .http
            .get(url.clone())
            .header(header::ACCEPT_ENCODING, "identity");
        let response = tokio::time::timeout(
            limits.max_duration.min(remaining),
            self.send_guarded_with_limits_using(
                request,
                Some(limits.max_bytes),
                false,
                "alaveteli_authority_request",
                execute,
            ),
        )
        .await
        .map_err(|_| SourceReadError::new(SourceFailureKind::Timeout))?
        .map_err(safe_source_error)?;
        if response.final_url != url {
            return Err(SourceReadError::new(SourceFailureKind::Validation).into());
        }
        let response = ensure_success(response, "authority request endpoint")
            .await
            .map_err(safe_source_error)?;
        let upstream: UpstreamAuthorityRequest =
            serde_json::from_slice(&response.body).map_err(malformed)?;
        if upstream.id != request_id {
            return Err(SourceReadError::new(SourceFailureKind::MalformedPayload).into());
        }
        let value = AlaveteliRequest {
            id: upstream.id,
            title: upstream.title,
            body: upstream.request_text,
            user_name: None,
            status: upstream.status,
            created_at: upstream.created_at,
            updated_at: upstream.updated_at,
            url: Some(upstream.url),
            tags: None,
        };
        Ok(response.into_source_read(value))
    }

    /// Fetch one search page with the same query construction and response
    /// variants as the ordinary source client. Pagination is never automatic.
    pub async fn search_requests_read(
        &self,
        options: &SearchOptions,
        limits: &ReadLimits,
    ) -> Result<SourceRead<Vec<AlaveteliRequest>>> {
        let url = self.search_url(options)?;
        let response = self.read_response(self.http.get(url), limits).await?;
        let value: Value = serde_json::from_slice(&response.body).map_err(malformed)?;
        let value = parse_search_response(value).map_err(malformed)?;
        Ok(response.into_source_read(value))
    }

    /// Fetch the authority catalogue with bounded bytes and no database access.
    pub async fn list_authorities_read(
        &self,
        limits: &ReadLimits,
    ) -> Result<SourceRead<Vec<Authority>>> {
        let url = self.base_url.join("api/v2/authority.json")?;
        let response = self.read_response(self.http.get(url), limits).await?;
        let value: Value = serde_json::from_slice(&response.body).map_err(malformed)?;
        let value = parse_authorities(value).map_err(malformed)?;
        Ok(response.into_source_read(value))
    }

    /// Discover the canonical Alaveteli authority-feed route without I/O.
    /// This declares a route convention, not its availability on the source.
    pub fn authority_feed_url(&self, authority_slug: &str) -> Result<String> {
        if !is_source_slug(authority_slug) {
            return Err(SourceReadError::new(SourceFailureKind::Validation).into());
        }
        let mut url = self.base_url.clone();
        url.set_path(&format!(
            "{}/feed/body/{authority_slug}",
            self.base_url.path().trim_end_matches('/')
        ));
        Ok(url.to_string())
    }

    /// Fetch and validate one RSS/Atom feed, returning a bounded pull plan.
    /// Retain same-origin request and request-event URLs within the configured
    /// base path. Descriptive slugs and event IDs remain unresolved. No links
    /// are followed, no database is changed, and no remote write is sent.
    /// Construct with `with_guardrails` for one GET without redirects or retries.
    pub async fn plan_authority_feed(
        &self,
        authority_slug: &str,
        max_items: usize,
        limits: &ReadLimits,
    ) -> Result<SourceRead<AuthorityFeedPlan>> {
        if !(1..=200).contains(&max_items) {
            return Err(SourceReadError::new(SourceFailureKind::Validation).into());
        }
        let feed_url = self.authority_feed_url(authority_slug)?;
        let response = self.read_response(self.http.get(&feed_url), limits).await?;
        let feed = std::str::from_utf8(&response.body).map_err(malformed)?;
        let (references, truncated) =
            feed_references(feed, &self.base_url, &response.final_url, max_items)
                .map_err(malformed)?;
        let request_ids = references
            .iter()
            .filter_map(|reference| reference.request_id)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        Ok(response.into_source_read(AuthorityFeedPlan {
            feed_url,
            references,
            request_ids,
            truncated,
        }))
    }
}

#[cfg(test)]
mod tests;

/// A bounded XML feed reader. Retain canonical source references without
/// interpreting event IDs or descriptive slugs as numeric request IDs.
/// DTDs and custom entity resolution are forbidden.
fn feed_references(
    feed: &str,
    base_url: &Url,
    document_url: &Url,
    limit: usize,
) -> Result<(Vec<AuthorityFeedReference>, bool)> {
    use quick_xml::events::Event;
    if feed
        .chars()
        .any(|character| character.is_control() && !matches!(character, '\t' | '\n' | '\r'))
    {
        return Err(anyhow!("feed contains invalid XML characters"));
    }
    let mut reader = quick_xml::Reader::from_str(feed);
    reader.config_mut().check_comments = true;
    reader.config_mut().check_end_names = true;
    reader.config_mut().expand_empty_elements = true;
    let mut stack: Vec<String> = Vec::new();
    let mut seen_root = false;
    let mut closed_root = false;
    let mut seen_declaration = false;
    let mut rss = false;
    let mut channels = 0;
    let mut link_text = String::new();
    let mut references = BTreeMap::new();
    let mut truncated = false;
    loop {
        match reader.read_event()? {
            Event::Start(element) => {
                let name = element.local_name().as_ref().to_owned();
                if stack.is_empty() {
                    if seen_root || closed_root || !matches!(name.as_str(), "rss" | "feed") {
                        return Err(anyhow!("feed requires one RSS or Atom root"));
                    }
                    seen_root = true;
                    rss = name == "rss";
                }
                if stack.len() >= 64 {
                    return Err(anyhow!("feed nesting limit exceeded"));
                }
                if stack.as_slice() == ["rss"] && name == "channel" {
                    channels += 1;
                }
                if captures_link_text(&stack) {
                    return Err(anyhow!("feed request links must contain text only"));
                }
                let atom_link = stack.as_slice() == ["feed", "entry"] && name == "link";
                let mut href = None;
                let mut relation = None;
                for attribute in element.attributes() {
                    let attribute = attribute?;
                    let value = attribute.normalized_value(quick_xml::XmlVersion::Explicit1_0)?;
                    if attribute.key.as_ref() == "xml:base" {
                        // Ignoring an alternate base could relabel a foreign
                        // relative link as belonging to this configured source.
                        return Err(anyhow!("feed XML base overrides are not supported"));
                    }
                    if atom_link {
                        match attribute.key.as_ref() {
                            "href" => href = Some(value.into_owned()),
                            "rel" => relation = Some(value.into_owned()),
                            _ => {}
                        }
                    }
                }
                if relation
                    .as_deref()
                    .is_none_or(|relation| relation == "alternate")
                {
                    if let Some(href) = href {
                        admit_link(
                            &href,
                            base_url,
                            document_url,
                            limit,
                            &mut references,
                            &mut truncated,
                        );
                    }
                }
                stack.push(name);
                if captures_link_text(&stack) {
                    link_text.clear();
                }
            }
            Event::End(_) => {
                if captures_link_text(&stack) {
                    admit_link(
                        &link_text,
                        base_url,
                        document_url,
                        limit,
                        &mut references,
                        &mut truncated,
                    );
                    link_text.clear();
                }
                stack.pop().ok_or_else(|| anyhow!("unexpected feed end"))?;
                if stack.is_empty() {
                    closed_root = true;
                }
            }
            Event::Text(text) => {
                if stack.is_empty() && !text.trim().is_empty() {
                    return Err(anyhow!("text outside feed root"));
                }
                if captures_link_text(&stack) {
                    link_text.push_str(&text);
                }
            }
            Event::CData(text) => {
                if stack.is_empty() {
                    return Err(anyhow!("CDATA outside feed root"));
                }
                if captures_link_text(&stack) {
                    link_text.push_str(&text);
                }
            }
            Event::GeneralRef(reference) => {
                let escaped = format!("&{};", reference.as_ref());
                let value = quick_xml::escape::unescape(&escaped)?;
                if stack.is_empty() {
                    return Err(anyhow!("reference outside feed root"));
                }
                if value.chars().any(|character| {
                    character.is_control() && !matches!(character, '\t' | '\n' | '\r')
                }) {
                    return Err(anyhow!("feed contains an invalid character reference"));
                }
                if captures_link_text(&stack) {
                    link_text.push_str(&value);
                }
            }
            Event::DocType(_) => return Err(anyhow!("feed DTDs are not supported")),
            Event::Empty(_) => return Err(anyhow!("unexpected unexpanded feed element")),
            Event::Decl(declaration) => {
                if seen_root || seen_declaration || declaration.version()?.as_ref() != "1.0" {
                    return Err(anyhow!("unsupported XML declaration"));
                }
                seen_declaration = true;
                if declaration
                    .encoding()
                    .transpose()?
                    .is_some_and(|encoding| !encoding.eq_ignore_ascii_case("utf-8"))
                {
                    return Err(anyhow!("feed encoding must be UTF-8"));
                }
            }
            Event::Eof => break,
            Event::Comment(_) | Event::PI(_) => {}
        }
    }
    if !seen_root || !closed_root || !stack.is_empty() || (rss && channels != 1) {
        return Err(anyhow!("incomplete feed document"));
    }
    Ok((references.into_values().collect(), truncated))
}

fn captures_link_text(stack: &[String]) -> bool {
    stack == ["rss", "channel", "item", "link"]
        || stack == ["rss", "channel", "item", "guid"]
        || stack == ["feed", "entry", "id"]
}

fn admit_link(
    link: &str,
    base_url: &Url,
    document_url: &Url,
    limit: usize,
    references: &mut BTreeMap<String, AuthorityFeedReference>,
    truncated: &mut bool,
) {
    let Some(reference) = canonical_feed_reference(link, base_url, document_url) else {
        return;
    };
    references.insert(reference.url.clone(), reference);
    if references.len() > limit {
        references.pop_last();
        *truncated = true;
    }
}

fn is_source_slug(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

fn canonical_feed_reference(
    link: &str,
    base_url: &Url,
    document_url: &Url,
) -> Option<AuthorityFeedReference> {
    let link = link.trim();
    // Reject encodings before URL parsing can normalize encoded dot segments;
    // fragment text is discarded and has no effect on route admission.
    if link.is_empty()
        || link
            .bytes()
            .any(|byte| byte.is_ascii_control() || byte == b'\\')
        || link.split(['?', '#']).next()?.contains('%')
    {
        return None;
    }
    let mut url = document_url.join(link).ok()?;
    if url.origin() != base_url.origin()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
    {
        return None;
    }
    url.set_fragment(None);
    let prefix = format!("{}/", base_url.path().trim_end_matches('/'));
    let route = url.path().strip_prefix(&prefix)?;
    let (kind, request_id) = if let Some(id) = route
        .strip_prefix("api/v2/request/")
        .and_then(|route| route.strip_suffix(".json"))
    {
        (
            AuthorityFeedReferenceKind::Request,
            Some(positive_source_id(id)?),
        )
    } else {
        let (kind, segment) = route.split_once('/')?;
        if !is_source_slug(segment) {
            return None;
        }
        match kind {
            // Alaveteli uses find_by_url_title!, including for all-digit titles.
            "request" => (AuthorityFeedReferenceKind::Request, None),
            "request_event" => {
                positive_source_id(segment)?;
                (AuthorityFeedReferenceKind::RequestEvent, None)
            }
            _ => return None,
        }
    };
    Some(AuthorityFeedReference {
        kind,
        url: url.to_string(),
        request_id,
    })
}

fn positive_source_id(value: &str) -> Option<i64> {
    if value.is_empty() || value.len() > 19 || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    value.parse::<i64>().ok().filter(|id| *id > 0)
}
