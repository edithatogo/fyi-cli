//! Bounded, read-only source contracts shared with the MCP adapter.
//!
//! These methods use the existing source transport and validator cache. They
//! never open an application database and never follow a feed's links.

use super::*;
use serde::Serialize;

/// Per-operation transfer limits. Request counts and aggregate transfer bytes
/// are additionally bounded by the client's shared `GuardrailConfig`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadLimits {
    pub max_bytes: u64,
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

/// Request identifiers discovered in one authority feed. Planning never fetches
/// these request records or applies them to a database.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AuthorityFeedPlan {
    pub feed_url: String,
    pub request_ids: Vec<i64>,
    /// More unique, valid identifiers were present than the caller's item limit.
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
        Self::with_http_client_identity_and_guardrails(base_url, http, identity.clone(), guardrails)
            .map_err(|error| error.context(SourceReadError::new(SourceFailureKind::Validation)))
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

    /// Strict one-GET health probe. Unlike the legacy convenience health check,
    /// source and transport failures retain their typed error classification.
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
        if authority_slug.is_empty()
            || authority_slug.len() > 128
            || !authority_slug
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        {
            return Err(SourceReadError::new(SourceFailureKind::Validation).into());
        }
        let url = self.base_url.join(&format!("body/{authority_slug}/feed"))?;
        Ok(url.to_string())
    }

    /// Fetch and validate one RSS/Atom feed, returning a bounded pull plan.
    /// Only same-origin positive request IDs are admitted. There is no request
    /// detail traversal, local database mutation, retry, or remote write.
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
        let (request_ids, truncated) =
            feed_request_ids(feed, &self.base_url, max_items).map_err(malformed)?;
        Ok(response.into_source_read(AuthorityFeedPlan {
            feed_url,
            request_ids,
            truncated,
        }))
    }
}

#[cfg(test)]
mod tests;

/// A bounded XML feed reader. It shares the numeric request-ID extractor with
/// legacy feed acquisition, but checks XML structure and URL origin before
/// considering link content. DTDs and custom entity resolution are forbidden.
fn feed_request_ids(feed: &str, base_url: &Url, limit: usize) -> Result<(Vec<i64>, bool)> {
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
    let mut ids = BTreeSet::new();
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
                let inside_entry = is_feed_item_path(&stack);
                for attribute in element.attributes() {
                    let attribute = attribute?;
                    let value = attribute.normalized_value(quick_xml::XmlVersion::Explicit1_0)?;
                    if attribute.key.as_ref() == "xml:base" {
                        // Ignoring an alternate base could relabel a foreign
                        // relative link as belonging to this configured source.
                        return Err(anyhow!("feed XML base overrides are not supported"));
                    }
                    if name == "link" && inside_entry && attribute.key.as_ref() == "href" {
                        admit_link(&value, base_url, limit, &mut ids, &mut truncated);
                    }
                }
                if inside_entry && matches!(name.as_str(), "link" | "id" | "guid") {
                    link_text.clear();
                }
                stack.push(name);
            }
            Event::End(_) => {
                let name = stack.pop().ok_or_else(|| anyhow!("unexpected feed end"))?;
                if matches!(name.as_str(), "link" | "id" | "guid") && is_feed_item_path(&stack) {
                    admit_link(&link_text, base_url, limit, &mut ids, &mut truncated);
                    link_text.clear();
                }
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
    Ok((ids.into_iter().collect(), truncated))
}

fn is_feed_item_path(stack: &[String]) -> bool {
    stack == ["rss", "channel", "item"] || stack == ["feed", "entry"]
}

fn captures_link_text(stack: &[String]) -> bool {
    stack
        .last()
        .is_some_and(|name| matches!(name.as_str(), "link" | "id" | "guid"))
        && is_feed_item_path(&stack[..stack.len().saturating_sub(1)])
}

fn admit_link(
    link: &str,
    base_url: &Url,
    limit: usize,
    ids: &mut BTreeSet<i64>,
    truncated: &mut bool,
) {
    let Ok(url) = base_url.join(link.trim()) else {
        return;
    };
    if url.origin() != base_url.origin()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return;
    }
    let path = url.path();
    let Ok(singular) = base_url.join("request/") else {
        return;
    };
    let Ok(plural) = base_url.join("requests/") else {
        return;
    };
    let Some(rest) = path
        .strip_prefix(singular.path())
        .or_else(|| path.strip_prefix(plural.path()))
    else {
        return;
    };
    let segment = rest.split('/').next().unwrap_or_default();
    if segment.is_empty() || !segment.bytes().all(|byte| byte.is_ascii_digit()) {
        return;
    }
    for id in request_ids_from_feed(&format!("/request/{segment}"))
        .into_iter()
        .filter(|id| *id > 0)
    {
        ids.insert(id);
        if ids.len() > limit {
            ids.pop_last();
            *truncated = true;
        }
    }
}
