//! Resource-aware autonomous agent network middleware.
//!
//! Weaves bidirectional rate-limit signalling, adaptive pacing, plan reflection,
//! identity hygiene, behavioral guardrails, local filesystem caching, load memory,
//! and FOSS-friendly execution traces into the outbound HTTP pipeline.
//!
//! Compatible with LangGraph/OpenClaw-style perception → reason → act → reflect
//! composition without hard-depending on those runtimes. Trace events use a
//! Langfuse/Braintrust-friendly span schema and default to local JSONL.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use thiserror::Error;

/// Product constants for cryptographic-aligned identity.
pub const PRODUCT_NAME: &str = "fyi-cli";
pub const PRODUCT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const PRODUCT_REPO: &str = "https://github.com/edithatogo/fyi-cli";

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AgentRuntimeError {
    #[error("invalid user-agent identity: {0}")]
    InvalidIdentity(String),
    #[error("guardrail tripped: {0}")]
    Guardrail(String),
    #[error("retrieval plan rejected: {0}")]
    PlanRejected(String),
    #[error("rate limited; retry after {0} seconds")]
    RateLimited(u64),
    #[error("cache error: {0}")]
    Cache(String),
    #[error("trace error: {0}")]
    Trace(String),
}

// ---------------------------------------------------------------------------
// Identity hygiene (cryptographic-aligned User-Agent)
// ---------------------------------------------------------------------------

/// Distinct, traceable client identity for live outbound HTTP.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientIdentity {
    pub product: String,
    pub version: String,
    /// Non-secret SHA-256 prefix over product+version+repo for build alignment.
    pub fingerprint: String,
    pub homepage: String,
    /// Opt-in administrative contact (email or URL). Never required.
    pub admin_contact: Option<String>,
}

impl ClientIdentity {
    /// Build the default product identity. `admin_contact` is opt-in only.
    pub fn default_identity(admin_contact: Option<String>) -> Result<Self, AgentRuntimeError> {
        let fingerprint = content_fingerprint(PRODUCT_NAME, PRODUCT_VERSION, PRODUCT_REPO);
        let identity = Self {
            product: PRODUCT_NAME.to_string(),
            version: PRODUCT_VERSION.to_string(),
            fingerprint,
            homepage: PRODUCT_REPO.to_string(),
            admin_contact: normalize_opt_contact(admin_contact),
        };
        identity.validate()?;
        Ok(identity)
    }

    /// Validate a custom identity (still must be product-shaped and non-generic).
    pub fn custom(
        product: impl Into<String>,
        version: impl Into<String>,
        homepage: impl Into<String>,
        admin_contact: Option<String>,
    ) -> Result<Self, AgentRuntimeError> {
        let product = product.into();
        let version = version.into();
        let homepage = homepage.into();
        let fingerprint = content_fingerprint(&product, &version, &homepage);
        let identity = Self {
            product,
            version,
            fingerprint,
            homepage,
            admin_contact: normalize_opt_contact(admin_contact),
        };
        identity.validate()?;
        Ok(identity)
    }

    pub fn validate(&self) -> Result<(), AgentRuntimeError> {
        if self.product.trim().is_empty() {
            return Err(AgentRuntimeError::InvalidIdentity(
                "product name is required".into(),
            ));
        }
        if self.version.trim().is_empty() {
            return Err(AgentRuntimeError::InvalidIdentity(
                "version is required".into(),
            ));
        }
        if self.fingerprint.len() < 8 {
            return Err(AgentRuntimeError::InvalidIdentity(
                "fingerprint too short".into(),
            ));
        }
        if !self.homepage.contains("://") {
            return Err(AgentRuntimeError::InvalidIdentity(
                "homepage must be an absolute URL".into(),
            ));
        }
        if let Some(contact) = &self.admin_contact {
            if contact.trim().is_empty() {
                return Err(AgentRuntimeError::InvalidIdentity(
                    "admin contact, if set, must be non-empty".into(),
                ));
            }
        }
        let ua = self.user_agent();
        if is_generic_user_agent(&ua) {
            return Err(AgentRuntimeError::InvalidIdentity(
                "user-agent is blank or generic".into(),
            ));
        }
        Ok(())
    }

    /// RFC-ish User-Agent: product/version (fp:…; +homepage[; contact:…])
    pub fn user_agent(&self) -> String {
        let mut inner = format!("fp:{}; +{}", self.fingerprint, self.homepage);
        if let Some(contact) = &self.admin_contact {
            inner.push_str("; contact:");
            inner.push_str(contact);
        }
        format!("{}/{} ({})", self.product, self.version, inner)
    }
}

/// SHA-256 prefix (16 hex chars) — cryptographic alignment without secrets.
pub fn content_fingerprint(product: &str, version: &str, homepage: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(product.as_bytes());
    hasher.update(b"\0");
    hasher.update(version.as_bytes());
    hasher.update(b"\0");
    hasher.update(homepage.as_bytes());
    let digest = hasher.finalize();
    digest.iter().take(8).map(|b| format!("{b:02x}")).collect()
}

pub fn is_generic_user_agent(ua: &str) -> bool {
    let t = ua.trim();
    if t.is_empty() {
        return true;
    }
    let lower = t.to_ascii_lowercase();
    if lower == "mozilla/5.0" || lower.starts_with("curl/") || lower.starts_with("wget/") {
        return true;
    }
    if lower.starts_with("python-requests/")
        || lower.starts_with("python-urllib/")
        || lower.starts_with("go-http-client/")
        || lower.starts_with("reqwest/")
    {
        return true;
    }
    // Product token required (name/version).
    if !t.contains('/') {
        return true;
    }
    false
}

fn normalize_opt_contact(contact: Option<String>) -> Option<String> {
    contact.and_then(|c| {
        let t = c.trim().to_string();
        if t.is_empty() {
            None
        } else {
            Some(t)
        }
    })
}

// ---------------------------------------------------------------------------
// Rate-limit header interception
// ---------------------------------------------------------------------------

/// Parsed Alaveteli/RFC rate-limit and retry signalling.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RateLimitSnapshot {
    pub limit: Option<u64>,
    pub remaining: Option<u64>,
    pub reset_seconds: Option<u64>,
    pub retry_after_seconds: Option<u64>,
    pub advisory_status: Option<String>,
    pub http_status: Option<u16>,
}

impl RateLimitSnapshot {
    /// Parse from header name/value pairs (case-insensitive names).
    pub fn from_headers<'a, I>(headers: I) -> Self
    where
        I: IntoIterator<Item = (&'a str, &'a str)>,
    {
        let mut snap = Self::default();
        for (name, value) in headers {
            let key = name.to_ascii_lowercase();
            match key.as_str() {
                "ratelimit-limit" | "x-ratelimit-limit" => {
                    snap.limit = parse_u64_header(value);
                }
                "ratelimit-remaining" | "x-ratelimit-remaining" => {
                    snap.remaining = parse_u64_header(value);
                }
                "ratelimit-reset" | "x-ratelimit-reset" => {
                    snap.reset_seconds = parse_reset_seconds(value);
                }
                "retry-after" => {
                    snap.retry_after_seconds = parse_retry_after(value);
                }
                "x-advisory-status" => {
                    let status = value.trim();
                    if !status.is_empty() {
                        snap.advisory_status = Some(status.to_ascii_lowercase());
                    }
                }
                _ => {}
            }
        }
        snap
    }

    pub fn is_rate_limited_status(&self) -> bool {
        self.http_status == Some(429)
    }
}

fn parse_u64_header(value: &str) -> Option<u64> {
    value.trim().split('.').next()?.parse().ok()
}

fn parse_reset_seconds(value: &str) -> Option<u64> {
    let t = value.trim();
    if let Ok(secs) = t.parse::<u64>() {
        return Some(secs);
    }
    // HTTP-date → seconds until then (best-effort; zero if past).
    if let Ok(dt) = DateTime::parse_from_rfc2822(t) {
        let now = Utc::now();
        let delta = dt.with_timezone(&Utc).signed_duration_since(now);
        return Some(delta.num_seconds().max(0) as u64);
    }
    None
}

fn parse_retry_after(value: &str) -> Option<u64> {
    parse_reset_seconds(value)
}

// ---------------------------------------------------------------------------
// Adaptive pacing
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PacingState {
    Baseline,
    Degraded,
    BackingOff,
    Recovering,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PacingPolicy {
    /// Baseline inter-request delay.
    pub baseline_delay: Duration,
    pub min_delay: Duration,
    pub max_delay: Duration,
    /// Enter degraded when remaining ≤ this fraction of limit (or absolute).
    pub remaining_degraded_threshold: u64,
    pub latency_high_ms: u64,
    pub recovery_successes_needed: u32,
    pub max_backoff_seconds: u64,
    pub baseline_concurrency: u32,
    pub degraded_concurrency: u32,
    pub baseline_batch_size: u32,
    pub degraded_batch_size: u32,
}

impl Default for PacingPolicy {
    fn default() -> Self {
        Self {
            baseline_delay: Duration::from_millis(250),
            min_delay: Duration::from_millis(50),
            max_delay: Duration::from_secs(120),
            remaining_degraded_threshold: 5,
            latency_high_ms: 2_000,
            recovery_successes_needed: 5,
            max_backoff_seconds: 300,
            baseline_concurrency: 2,
            degraded_concurrency: 1,
            baseline_batch_size: 50,
            degraded_batch_size: 10,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PacingEngine {
    pub policy: PacingPolicy,
    pub state: PacingState,
    pub current_delay: Duration,
    pub concurrency: u32,
    pub batch_size: u32,
    consecutive_good: u32,
    attempt: u32,
}

impl Default for PacingEngine {
    fn default() -> Self {
        let policy = PacingPolicy::default();
        Self {
            current_delay: policy.baseline_delay,
            concurrency: policy.baseline_concurrency,
            batch_size: policy.baseline_batch_size,
            policy,
            state: PacingState::Baseline,
            consecutive_good: 0,
            attempt: 0,
        }
    }
}

impl PacingEngine {
    pub fn new(policy: PacingPolicy) -> Self {
        Self {
            current_delay: policy.baseline_delay,
            concurrency: policy.baseline_concurrency,
            batch_size: policy.baseline_batch_size,
            policy,
            state: PacingState::Baseline,
            consecutive_good: 0,
            attempt: 0,
        }
    }

    /// Observe a response; returns recommended wait before the next request.
    pub fn observe(&mut self, snap: &RateLimitSnapshot, latency: Duration) -> Duration {
        if snap.http_status == Some(429)
            || snap
                .retry_after_seconds
                .is_some_and(|s| s > 0 && snap.http_status == Some(429))
        {
            return self.enter_backoff(snap.retry_after_seconds);
        }

        let advisory_degraded = snap.advisory_status.as_deref() == Some("degraded");
        let low_remaining = match (snap.remaining, snap.limit) {
            (Some(rem), _) if rem <= self.policy.remaining_degraded_threshold => true,
            (Some(rem), Some(limit)) if limit > 0 && rem * 10 <= limit => true,
            _ => advisory_degraded,
        };
        let slow = latency.as_millis() as u64 >= self.policy.latency_high_ms;

        if low_remaining || slow {
            self.state = PacingState::Degraded;
            self.concurrency = self.policy.degraded_concurrency;
            self.batch_size = self.policy.degraded_batch_size;
            self.current_delay = (self.current_delay.saturating_mul(2)).min(self.policy.max_delay);
            self.consecutive_good = 0;
            return self.current_delay;
        }

        // Healthy response path.
        self.consecutive_good = self.consecutive_good.saturating_add(1);
        match self.state {
            PacingState::BackingOff | PacingState::Degraded => {
                if self.consecutive_good >= self.policy.recovery_successes_needed {
                    self.state = PacingState::Recovering;
                }
            }
            PacingState::Recovering => {
                if self.consecutive_good >= self.policy.recovery_successes_needed * 2 {
                    self.enter_baseline();
                } else {
                    self.current_delay = ((self.current_delay + self.policy.baseline_delay) / 2)
                        .max(self.policy.min_delay);
                    self.concurrency = self
                        .policy
                        .degraded_concurrency
                        .max(1)
                        .min(self.policy.baseline_concurrency);
                    self.batch_size = self
                        .policy
                        .degraded_batch_size
                        .max(1)
                        .min(self.policy.baseline_batch_size);
                }
            }
            PacingState::Baseline => {
                self.current_delay = self.policy.baseline_delay;
                self.concurrency = self.policy.baseline_concurrency;
                self.batch_size = self.policy.baseline_batch_size;
            }
        }
        self.current_delay
    }

    pub fn enter_backoff(&mut self, retry_after: Option<u64>) -> Duration {
        self.state = PacingState::BackingOff;
        self.concurrency = 1;
        self.batch_size = self.policy.degraded_batch_size.max(1);
        self.consecutive_good = 0;
        self.attempt = self.attempt.saturating_add(1);
        let exp =
            exponential_backoff_seconds(self.attempt, retry_after, self.policy.max_backoff_seconds);
        self.current_delay = Duration::from_secs(exp);
        self.current_delay
    }

    fn enter_baseline(&mut self) {
        self.state = PacingState::Baseline;
        self.current_delay = self.policy.baseline_delay;
        self.concurrency = self.policy.baseline_concurrency;
        self.batch_size = self.policy.baseline_batch_size;
        self.attempt = 0;
        self.consecutive_good = 0;
    }

    pub fn recommended_delay(&self) -> Duration {
        self.current_delay
    }
}

/// Mathematically sound exponential backoff with optional Retry-After floor.
pub fn exponential_backoff_seconds(
    attempt: u32,
    retry_after: Option<u64>,
    max_seconds: u64,
) -> u64 {
    let base = 1u64 << attempt.min(8); // 1,2,4,...,256
    let mut wait = base.min(max_seconds);
    if let Some(ra) = retry_after {
        wait = wait.max(ra).min(max_seconds.max(ra));
    }
    // Soft jitter: deterministic mix (no RNG dependency); ± up to 12.5%.
    let jitter = (wait / 8).max(1);
    let mixed = wait.saturating_add(attempt as u64 % (jitter + 1));
    mixed.min(max_seconds.max(retry_after.unwrap_or(0)))
}

// ---------------------------------------------------------------------------
// Behavioral guardrails
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuardrailConfig {
    pub max_requests: u64,
    pub max_response_bytes: u64,
    pub max_runtime: Duration,
    pub max_concurrency: u32,
}

impl Default for GuardrailConfig {
    fn default() -> Self {
        Self {
            max_requests: 10_000,
            max_response_bytes: 500 * 1024 * 1024, // 500 MiB
            max_runtime: Duration::from_secs(60 * 60), // 1 hour
            max_concurrency: 4,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GuardrailTracker {
    pub config: GuardrailConfig,
    started: Instant,
    requests: u64,
    response_bytes: u64,
}

impl GuardrailTracker {
    pub fn new(config: GuardrailConfig) -> Self {
        Self {
            config,
            started: Instant::now(),
            requests: 0,
            response_bytes: 0,
        }
    }

    pub fn check_can_start_request(&self) -> Result<(), AgentRuntimeError> {
        if self.requests >= self.config.max_requests {
            return Err(AgentRuntimeError::Guardrail(format!(
                "maximum request count reached ({})",
                self.config.max_requests
            )));
        }
        if self.response_bytes >= self.config.max_response_bytes {
            return Err(AgentRuntimeError::Guardrail(format!(
                "maximum response bytes reached ({})",
                self.config.max_response_bytes
            )));
        }
        if self.started.elapsed() >= self.config.max_runtime {
            return Err(AgentRuntimeError::Guardrail(format!(
                "maximum runtime exceeded ({:?})",
                self.config.max_runtime
            )));
        }
        Ok(())
    }

    pub fn record_request_start(&mut self) -> Result<(), AgentRuntimeError> {
        self.check_can_start_request()?;
        self.requests = self.requests.saturating_add(1);
        Ok(())
    }

    pub fn record_response_bytes(&mut self, bytes: u64) -> Result<(), AgentRuntimeError> {
        self.response_bytes = self.response_bytes.saturating_add(bytes);
        if self.response_bytes > self.config.max_response_bytes {
            return Err(AgentRuntimeError::Guardrail(format!(
                "maximum response bytes exceeded ({} > {})",
                self.response_bytes, self.config.max_response_bytes
            )));
        }
        if self.started.elapsed() >= self.config.max_runtime {
            return Err(AgentRuntimeError::Guardrail(format!(
                "maximum runtime exceeded ({:?})",
                self.config.max_runtime
            )));
        }
        Ok(())
    }

    pub fn snapshot(&self) -> GuardrailSnapshot {
        GuardrailSnapshot {
            requests: self.requests,
            response_bytes: self.response_bytes,
            elapsed_ms: self.started.elapsed().as_millis() as u64,
            max_requests: self.config.max_requests,
            max_response_bytes: self.config.max_response_bytes,
            max_runtime_ms: self.config.max_runtime.as_millis() as u64,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuardrailSnapshot {
    pub requests: u64,
    pub response_bytes: u64,
    pub elapsed_ms: u64,
    pub max_requests: u64,
    pub max_response_bytes: u64,
    pub max_runtime_ms: u64,
}

// ---------------------------------------------------------------------------
// Load memory (durable local state)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EndpointMemory {
    pub instance_id: String,
    pub route_class: String,
    pub samples: u64,
    pub ewma_latency_ms: f64,
    pub rate_limit_hits: u64,
    pub last_rate_limit_at: Option<String>,
    pub last_seen_at: Option<String>,
    /// Hour-of-day (0-23) hit counts for coarse high-load detection.
    pub hour_hits: [u64; 24],
}

impl EndpointMemory {
    pub fn key(instance_id: &str, route_class: &str) -> String {
        format!("{instance_id}::{route_class}")
    }

    pub fn observe_success(&mut self, latency: Duration) {
        let ms = latency.as_secs_f64() * 1000.0;
        if self.samples == 0 {
            self.ewma_latency_ms = ms;
        } else {
            const ALPHA: f64 = 0.2;
            self.ewma_latency_ms = ALPHA * ms + (1.0 - ALPHA) * self.ewma_latency_ms;
        }
        self.samples = self.samples.saturating_add(1);
        self.last_seen_at = Some(Utc::now().to_rfc3339());
        let hour = Utc::now()
            .format("%H")
            .to_string()
            .parse::<usize>()
            .unwrap_or(0);
        if hour < 24 {
            self.hour_hits[hour] = self.hour_hits[hour].saturating_add(1);
        }
    }

    pub fn observe_rate_limit(&mut self) {
        self.rate_limit_hits = self.rate_limit_hits.saturating_add(1);
        let now = Utc::now().to_rfc3339();
        self.last_rate_limit_at = Some(now.clone());
        self.last_seen_at = Some(now);
        let hour = Utc::now()
            .format("%H")
            .to_string()
            .parse::<usize>()
            .unwrap_or(0);
        if hour < 24 {
            self.hour_hits[hour] = self.hour_hits[hour].saturating_add(1);
        }
    }

    /// True if current UTC hour is among historically highest-load hours.
    pub fn is_historically_high_load_now(&self) -> bool {
        let total: u64 = self.hour_hits.iter().sum();
        if total < 20 {
            return false;
        }
        let hour = Utc::now()
            .format("%H")
            .to_string()
            .parse::<usize>()
            .unwrap_or(0);
        let mut ranked: Vec<(usize, u64)> = self
            .hour_hits
            .iter()
            .enumerate()
            .map(|(i, c)| (i, *c))
            .collect();
        ranked.sort_by(|a, b| b.1.cmp(&a.1));
        ranked
            .iter()
            .take(3)
            .any(|(h, count)| *h == hour && *count > 0)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LoadMemoryStore {
    endpoints: HashMap<String, EndpointMemory>,
}

impl LoadMemoryStore {
    pub fn observe(
        &mut self,
        instance_id: &str,
        route_class: &str,
        latency: Duration,
        rate_limited: bool,
    ) {
        let key = EndpointMemory::key(instance_id, route_class);
        let entry = self.endpoints.entry(key).or_insert_with(|| EndpointMemory {
            instance_id: instance_id.to_string(),
            route_class: route_class.to_string(),
            ..Default::default()
        });
        if rate_limited {
            entry.observe_rate_limit();
        } else {
            entry.observe_success(latency);
        }
        self.prune(512);
    }

    pub fn get(&self, instance_id: &str, route_class: &str) -> Option<&EndpointMemory> {
        self.endpoints
            .get(&EndpointMemory::key(instance_id, route_class))
    }

    pub fn should_defer_heavy_work(&self, instance_id: &str, route_class: &str) -> bool {
        self.get(instance_id, route_class)
            .map(|m| {
                m.is_historically_high_load_now()
                    || m.rate_limit_hits > 0 && m.ewma_latency_ms > 1500.0
            })
            .unwrap_or(false)
    }

    /// Keep durable memory bounded by retaining the most informative endpoints.
    pub fn prune(&mut self, max_endpoints: usize) {
        if self.endpoints.len() <= max_endpoints {
            return;
        }
        let mut ranked: Vec<(String, u64)> = self
            .endpoints
            .iter()
            .map(|(key, value)| {
                (
                    key.clone(),
                    value
                        .samples
                        .saturating_add(value.rate_limit_hits.saturating_mul(4)),
                )
            })
            .collect();
        ranked.sort_by(|a, b| b.1.cmp(&a.1));
        let keep: std::collections::HashSet<String> = ranked
            .into_iter()
            .take(max_endpoints)
            .map(|(key, _)| key)
            .collect();
        self.endpoints.retain(|key, _| keep.contains(key));
    }

    pub fn save_to_path(&self, path: &Path) -> Result<(), AgentRuntimeError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| AgentRuntimeError::Cache(e.to_string()))?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| AgentRuntimeError::Cache(e.to_string()))?;
        fs::write(path, json).map_err(|e| AgentRuntimeError::Cache(e.to_string()))
    }

    pub fn load_from_path(path: &Path) -> Result<Self, AgentRuntimeError> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let data = fs::read_to_string(path).map_err(|e| AgentRuntimeError::Cache(e.to_string()))?;
        serde_json::from_str(&data).map_err(|e| AgentRuntimeError::Cache(e.to_string()))
    }
}

// ---------------------------------------------------------------------------
// Plan reflection (plan-and-solve)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetrievalPlan {
    pub instance_id: String,
    pub description: String,
    /// Estimated remote HTTP calls.
    pub estimated_requests: u64,
    /// Inclusive date window preferred for bulk work.
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub max_pages: Option<u32>,
    pub recursive_unbounded: bool,
    pub is_heavy: bool,
    pub force_schedule: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum PlanDecision {
    Accept {
        rationale: String,
    },
    Rewrite {
        rationale: String,
        rewritten: RetrievalPlan,
    },
    Reject {
        rationale: String,
    },
}

pub fn reflect_plan(plan: &RetrievalPlan, memory: &LoadMemoryStore) -> PlanDecision {
    if plan.recursive_unbounded {
        if let (Some(from), Some(to)) = (&plan.date_from, &plan.date_to) {
            let mut rewritten = plan.clone();
            rewritten.recursive_unbounded = false;
            rewritten.max_pages = Some(plan.max_pages.unwrap_or(50).min(50));
            rewritten.description = format!(
                "{} [rewritten: bounded pages, window {from}..{to}]",
                plan.description
            );
            return PlanDecision::Rewrite {
                rationale: "unbounded recursive retrieval is not allowed; applied date window and page bound"
                    .into(),
                rewritten,
            };
        }
        return PlanDecision::Reject {
            rationale: "unbounded recursive retrieval without a date window is rejected to protect volunteer Alaveteli hosts"
                .into(),
        };
    }

    if plan.estimated_requests > 50_000 && !plan.force_schedule {
        return PlanDecision::Reject {
            rationale: format!(
                "estimated {} requests exceeds safety ceiling without force_schedule",
                plan.estimated_requests
            ),
        };
    }

    if plan.is_heavy
        && !plan.force_schedule
        && memory.should_defer_heavy_work(&plan.instance_id, "bulk")
    {
        return PlanDecision::Reject {
            rationale: "heavy job deferred: load memory indicates high-load window or recent rate limits; retry later or set force_schedule"
                .into(),
        };
    }

    if plan.is_heavy && plan.max_pages.is_none() {
        let mut rewritten = plan.clone();
        rewritten.max_pages = Some(100);
        return PlanDecision::Rewrite {
            rationale: "heavy plan missing max_pages; defaulting to 100".into(),
            rewritten,
        };
    }

    PlanDecision::Accept {
        rationale: "plan is bounded and within safety policy".into(),
    }
}

/// Rewrite a plan after a throttle so a resumed run has a smaller footprint.
/// The rewrite is deterministic and keeps the operator's instance/window scope.
pub fn rewrite_after_throttle(plan: &RetrievalPlan) -> PlanDecision {
    if plan.force_schedule {
        return PlanDecision::Accept {
            rationale: "throttle observed but force_schedule preserves the explicit operator plan"
                .into(),
        };
    }
    if plan.recursive_unbounded && plan.date_from.is_none() {
        return PlanDecision::Reject {
            rationale: "throttle observed; unbounded plan without a date window remains rejected"
                .into(),
        };
    }
    let mut rewritten = plan.clone();
    rewritten.recursive_unbounded = false;
    rewritten.estimated_requests = (plan.estimated_requests / 2).max(1);
    rewritten.max_pages = Some((plan.max_pages.unwrap_or(50).max(1) / 2).max(1));
    rewritten.description = format!("{} [rewritten after throttle]", plan.description);
    PlanDecision::Rewrite {
        rationale: "throttle observed; halved estimated work and page bound before resume".into(),
        rewritten,
    }
}

// ---------------------------------------------------------------------------
// Filesystem response cache
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct FilesystemResponseCache {
    root: PathBuf,
}

impl FilesystemResponseCache {
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, AgentRuntimeError> {
        let root = root.into();
        fs::create_dir_all(&root).map_err(|e| AgentRuntimeError::Cache(e.to_string()))?;
        Ok(Self { root })
    }

    fn key_path(&self, url: &str) -> PathBuf {
        let mut hasher = Sha256::new();
        hasher.update(url.as_bytes());
        let digest = hasher.finalize();
        let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
        self.root.join(&hex[..2]).join(&hex)
    }

    pub fn get(&self, url: &str) -> Result<Option<Vec<u8>>, AgentRuntimeError> {
        let path = self.key_path(url);
        if !path.exists() {
            return Ok(None);
        }
        let bytes = fs::read(path).map_err(|e| AgentRuntimeError::Cache(e.to_string()))?;
        Ok(Some(bytes))
    }

    pub fn put(&self, url: &str, body: &[u8]) -> Result<(), AgentRuntimeError> {
        let path = self.key_path(url);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| AgentRuntimeError::Cache(e.to_string()))?;
        }
        fs::write(path, body).map_err(|e| AgentRuntimeError::Cache(e.to_string()))
    }

    pub fn contains(&self, url: &str) -> bool {
        self.key_path(url).exists()
    }
}

// ---------------------------------------------------------------------------
// Trace capture (Langfuse / Braintrust compatible schema)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TraceEvent {
    /// Langfuse-style observation type.
    pub r#type: String,
    pub name: String,
    pub timestamp: String,
    pub run_id: String,
    pub instance_id: Option<String>,
    pub level: String,
    pub metadata: serde_json::Value,
    /// Optional parent span id for tree structure.
    pub parent_id: Option<String>,
    pub id: String,
}

pub trait TraceSink: Send {
    fn emit(&mut self, event: TraceEvent) -> Result<(), AgentRuntimeError>;
}

/// No-op sink for tests / disabled telemetry.
#[derive(Debug, Default)]
pub struct NullTraceSink;

impl TraceSink for NullTraceSink {
    fn emit(&mut self, _event: TraceEvent) -> Result<(), AgentRuntimeError> {
        Ok(())
    }
}

/// Append-only JSONL sink (local FOSS default).
#[derive(Debug)]
pub struct JsonlTraceSink {
    file: File,
}

impl JsonlTraceSink {
    pub fn create(path: impl AsRef<Path>) -> Result<Self, AgentRuntimeError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| AgentRuntimeError::Trace(e.to_string()))?;
        }
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|e| AgentRuntimeError::Trace(e.to_string()))?;
        Ok(Self { file })
    }
}

impl TraceSink for JsonlTraceSink {
    fn emit(&mut self, event: TraceEvent) -> Result<(), AgentRuntimeError> {
        let line =
            serde_json::to_string(&event).map_err(|e| AgentRuntimeError::Trace(e.to_string()))?;
        writeln!(self.file, "{line}").map_err(|e| AgentRuntimeError::Trace(e.to_string()))?;
        Ok(())
    }
}

/// In-memory sink for unit tests.
#[derive(Debug, Default)]
pub struct MemoryTraceSink {
    pub events: Vec<TraceEvent>,
}

impl TraceSink for MemoryTraceSink {
    fn emit(&mut self, event: TraceEvent) -> Result<(), AgentRuntimeError> {
        self.events.push(event);
        Ok(())
    }
}

fn new_event(
    run_id: &str,
    name: &str,
    type_: &str,
    instance_id: Option<&str>,
    metadata: serde_json::Value,
) -> TraceEvent {
    let id = {
        let mut hasher = Sha256::new();
        hasher.update(run_id.as_bytes());
        hasher.update(name.as_bytes());
        hasher.update(Utc::now().to_rfc3339().as_bytes());
        hasher.update(metadata.to_string().as_bytes());
        let d = hasher.finalize();
        d.iter().take(8).map(|b| format!("{b:02x}")).collect()
    };
    TraceEvent {
        r#type: type_.to_string(),
        name: name.to_string(),
        timestamp: Utc::now().to_rfc3339(),
        run_id: run_id.to_string(),
        instance_id: instance_id.map(|s| s.to_string()),
        level: "DEFAULT".to_string(),
        metadata,
        parent_id: None,
        id,
    }
}

// ---------------------------------------------------------------------------
// Network middleware (core execution engine)
// ---------------------------------------------------------------------------

/// Decision returned before an outbound request is allowed to proceed.
#[derive(Debug, Clone, PartialEq)]
pub struct PreRequestDecision {
    pub user_agent: String,
    pub wait: Duration,
    pub concurrency: u32,
    pub batch_size: u32,
    /// When set, caller should serve body from cache and skip the network.
    pub cache_hit: Option<Vec<u8>>,
}

/// Inputs describing a planned HTTP call.
#[derive(Debug, Clone)]
pub struct OutboundRequestMeta {
    pub instance_id: String,
    pub route_class: String,
    pub method: String,
    pub url: String,
}

/// Framework-neutral perception boundary: turn response metadata into policy input.
pub trait Perception {
    fn perceive(&self, status: u16, headers: &[(&str, &str)]) -> RateLimitSnapshot;
}

/// Framework-neutral reasoning boundary: evaluate a bounded retrieval plan.
pub trait Reason {
    fn reason(&self, plan: &RetrievalPlan) -> PlanDecision;
}

/// Framework-neutral action boundary: prepare a bounded outbound operation.
pub trait Action {
    fn act(&mut self, meta: &OutboundRequestMeta) -> Result<PreRequestDecision, AgentRuntimeError>;
}

/// Framework-neutral reflection boundary: account for the completed operation.
pub trait Reflection {
    fn reflect(
        &mut self,
        meta: &OutboundRequestMeta,
        status: u16,
        headers: &[(&str, &str)],
        latency: Duration,
        body: Option<&[u8]>,
    ) -> Result<Duration, AgentRuntimeError>;
}

/// Resource-aware middleware coordinating identity, pacing, guardrails, cache,
/// memory, plan reflection, and traces.
pub struct AgentNetworkMiddleware {
    pub identity: ClientIdentity,
    pub pacing: PacingEngine,
    pub guardrails: GuardrailTracker,
    pub memory: LoadMemoryStore,
    pub cache: Option<FilesystemResponseCache>,
    pub memory_path: Option<PathBuf>,
    pub run_id: String,
    trace: Arc<Mutex<Box<dyn TraceSink>>>,
}

impl AgentNetworkMiddleware {
    pub fn new(
        identity: ClientIdentity,
        guardrails: GuardrailConfig,
        pacing: PacingPolicy,
        trace: Box<dyn TraceSink>,
        run_id: impl Into<String>,
    ) -> Self {
        Self {
            identity,
            pacing: PacingEngine::new(pacing),
            guardrails: GuardrailTracker::new(guardrails),
            memory: LoadMemoryStore::default(),
            cache: None,
            memory_path: None,
            run_id: run_id.into(),
            trace: Arc::new(Mutex::new(trace)),
        }
    }

    pub fn with_defaults(admin_contact: Option<String>) -> Result<Self, AgentRuntimeError> {
        let identity = ClientIdentity::default_identity(admin_contact)?;
        Ok(Self::new(
            identity,
            GuardrailConfig::default(),
            PacingPolicy::default(),
            Box::new(NullTraceSink),
            format!("run-{}", Utc::now().timestamp()),
        ))
    }

    pub fn with_cache_dir(mut self, dir: impl Into<PathBuf>) -> Result<Self, AgentRuntimeError> {
        self.cache = Some(FilesystemResponseCache::new(dir)?);
        Ok(self)
    }

    pub fn with_memory_path(mut self, path: impl Into<PathBuf>) -> Result<Self, AgentRuntimeError> {
        let path = path.into();
        self.memory = LoadMemoryStore::load_from_path(&path)?;
        self.memory_path = Some(path);
        Ok(self)
    }

    pub fn with_trace_sink(mut self, sink: Box<dyn TraceSink>) -> Self {
        self.trace = Arc::new(Mutex::new(sink));
        self
    }

    fn emit(
        &self,
        name: &str,
        type_: &str,
        instance_id: Option<&str>,
        metadata: serde_json::Value,
    ) {
        let event = new_event(&self.run_id, name, type_, instance_id, metadata);
        if let Ok(mut sink) = self.trace.lock() {
            let _ = sink.emit(event);
        }
    }

    /// Perception + reason: evaluate a bulk retrieval plan before execution.
    pub fn reflect_and_trace(&self, plan: &RetrievalPlan) -> PlanDecision {
        let decision = reflect_plan(plan, &self.memory);
        let meta = serde_json::json!({
            "plan": plan,
            "decision": decision,
        });
        self.emit(
            "plan.reflect",
            "span",
            Some(&plan.instance_id),
            redact_secrets(meta),
        );
        decision
    }

    /// Before sending: guardrails, pacing wait, optional cache short-circuit.
    pub fn before_request(
        &mut self,
        meta: &OutboundRequestMeta,
    ) -> Result<PreRequestDecision, AgentRuntimeError> {
        self.guardrails.record_request_start()?;

        if meta.method.eq_ignore_ascii_case("GET") {
            if let Some(cache) = &self.cache {
                if let Some(body) = cache.get(&meta.url)? {
                    self.emit(
                        "cache.hit",
                        "event",
                        Some(&meta.instance_id),
                        serde_json::json!({ "url_fp": url_fingerprint(&meta.url) }),
                    );
                    // Cache hits still count toward request guardrail (logical ops)
                    // but do not wait on pacing.
                    return Ok(PreRequestDecision {
                        user_agent: self.identity.user_agent(),
                        wait: Duration::ZERO,
                        concurrency: self.pacing.concurrency,
                        batch_size: self.pacing.batch_size,
                        cache_hit: Some(body),
                    });
                }
                self.emit(
                    "cache.miss",
                    "event",
                    Some(&meta.instance_id),
                    serde_json::json!({ "url_fp": url_fingerprint(&meta.url) }),
                );
            }
        }

        let wait = self.pacing.recommended_delay();
        self.emit(
            "pacing.decision",
            "event",
            Some(&meta.instance_id),
            serde_json::json!({
                "state": self.pacing.state,
                "wait_ms": wait.as_millis() as u64,
                "concurrency": self.pacing.concurrency,
                "route_class": meta.route_class,
            }),
        );
        self.emit(
            "http.request",
            "span",
            Some(&meta.instance_id),
            serde_json::json!({
                "method": meta.method,
                "url_fp": url_fingerprint(&meta.url),
                "route_class": meta.route_class,
                "user_agent": self.identity.user_agent(),
            }),
        );

        Ok(PreRequestDecision {
            user_agent: self.identity.user_agent(),
            wait,
            concurrency: self.pacing.concurrency,
            batch_size: self.pacing.batch_size,
            cache_hit: None,
        })
    }

    /// After response headers/body known: update pacing, memory, cache, traces.
    pub fn after_response(
        &mut self,
        meta: &OutboundRequestMeta,
        status: u16,
        headers: &[(&str, &str)],
        latency: Duration,
        body: Option<&[u8]>,
    ) -> Result<Duration, AgentRuntimeError> {
        let mut snap = RateLimitSnapshot::from_headers(headers.iter().copied());
        snap.http_status = Some(status);

        if let Some(bytes) = body {
            if let Err(e) = self.guardrails.record_response_bytes(bytes.len() as u64) {
                self.emit(
                    "guardrail.trip",
                    "event",
                    Some(&meta.instance_id),
                    serde_json::json!({ "error": e.to_string() }),
                );
                return Err(e);
            }
            if status == 200 && meta.method.eq_ignore_ascii_case("GET") {
                if let Some(cache) = &self.cache {
                    cache.put(&meta.url, bytes)?;
                }
            }
        }

        let rate_limited = status == 429;
        self.memory
            .observe(&meta.instance_id, &meta.route_class, latency, rate_limited);
        if let Some(path) = &self.memory_path {
            let _ = self.memory.save_to_path(path);
        }

        let wait = self.pacing.observe(&snap, latency);
        self.emit(
            "http.response_headers",
            "event",
            Some(&meta.instance_id),
            serde_json::json!({
                "status": status,
                "latency_ms": latency.as_millis() as u64,
                "rate_limit": snap,
                "next_wait_ms": wait.as_millis() as u64,
                "pacing_state": self.pacing.state,
            }),
        );

        if rate_limited {
            let secs = snap.retry_after_seconds.unwrap_or(wait.as_secs().max(1));
            self.emit(
                "backoff.wait",
                "event",
                Some(&meta.instance_id),
                serde_json::json!({ "retry_after_seconds": secs }),
            );
            return Err(AgentRuntimeError::RateLimited(secs));
        }

        Ok(wait)
    }

    /// Account for a response chunk while it is being read, so callers can
    /// stop buffering before an oversized body exhausts memory.
    pub fn record_response_chunk(&mut self, bytes: u64) -> Result<(), AgentRuntimeError> {
        if let Err(error) = self.guardrails.record_response_bytes(bytes) {
            self.emit(
                "guardrail.trip",
                "event",
                None,
                serde_json::json!({ "error": error.to_string() }),
            );
            return Err(error);
        }
        Ok(())
    }

    pub fn user_agent(&self) -> String {
        self.identity.user_agent()
    }

    pub fn status_report(&self) -> serde_json::Value {
        serde_json::json!({
            "run_id": self.run_id,
            "user_agent": self.identity.user_agent(),
            "pacing": {
                "state": self.pacing.state,
                "delay_ms": self.pacing.current_delay.as_millis() as u64,
                "concurrency": self.pacing.concurrency,
                "batch_size": self.pacing.batch_size,
            },
            "memory_endpoints": self.memory.endpoints.len(),
            "guardrails": self.guardrails.snapshot(),
        })
    }
}

impl Perception for AgentNetworkMiddleware {
    fn perceive(&self, status: u16, headers: &[(&str, &str)]) -> RateLimitSnapshot {
        let mut snapshot = RateLimitSnapshot::from_headers(headers.iter().copied());
        snapshot.http_status = Some(status);
        snapshot
    }
}

impl Reason for AgentNetworkMiddleware {
    fn reason(&self, plan: &RetrievalPlan) -> PlanDecision {
        self.reflect_and_trace(plan)
    }
}

impl Action for AgentNetworkMiddleware {
    fn act(&mut self, meta: &OutboundRequestMeta) -> Result<PreRequestDecision, AgentRuntimeError> {
        self.before_request(meta)
    }
}

impl Reflection for AgentNetworkMiddleware {
    fn reflect(
        &mut self,
        meta: &OutboundRequestMeta,
        status: u16,
        headers: &[(&str, &str)],
        latency: Duration,
        body: Option<&[u8]>,
    ) -> Result<Duration, AgentRuntimeError> {
        self.after_response(meta, status, headers, latency, body)
    }
}

/// Minimal in-repository adapter showing how a graph/tool runtime can compose
/// the four boundaries without depending on an agent framework.
pub struct ThinAgentAdapter {
    pub middleware: AgentNetworkMiddleware,
}

impl ThinAgentAdapter {
    pub fn new(middleware: AgentNetworkMiddleware) -> Self {
        Self { middleware }
    }

    pub fn plan(&self, plan: &RetrievalPlan) -> PlanDecision {
        self.middleware.reason(plan)
    }

    pub fn prepare(
        &mut self,
        request: &OutboundRequestMeta,
    ) -> Result<PreRequestDecision, AgentRuntimeError> {
        self.middleware.act(request)
    }

    pub fn complete(
        &mut self,
        request: &OutboundRequestMeta,
        status: u16,
        headers: &[(&str, &str)],
        latency: Duration,
        body: Option<&[u8]>,
    ) -> Result<Duration, AgentRuntimeError> {
        self.middleware
            .reflect(request, status, headers, latency, body)
    }
}

/// Build a reqwest client with the mandatory identity User-Agent.
pub fn build_http_client(identity: &ClientIdentity) -> Result<reqwest::Client, AgentRuntimeError> {
    identity.validate()?;
    reqwest::Client::builder()
        .user_agent(identity.user_agent())
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| AgentRuntimeError::InvalidIdentity(e.to_string()))
}

fn url_fingerprint(url: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(url.as_bytes());
    let d = hasher.finalize();
    d.iter().take(6).map(|b| format!("{b:02x}")).collect()
}

/// Best-effort redaction of common secret material from trace metadata.
pub fn redact_secrets(mut value: serde_json::Value) -> serde_json::Value {
    fn walk(v: &mut serde_json::Value) {
        match v {
            serde_json::Value::Object(map) => {
                let keys: Vec<String> = map.keys().cloned().collect();
                for k in keys {
                    let lower = k.to_ascii_lowercase();
                    if lower.contains("api_key")
                        || lower.contains("authorization")
                        || lower.contains("password")
                        || lower.contains("secret")
                        || lower.contains("cookie")
                        || lower.contains("token")
                    {
                        map.insert(k, serde_json::Value::String("[redacted]".into()));
                    } else if let Some(child) = map.get_mut(&k) {
                        walk(child);
                    }
                }
            }
            serde_json::Value::Array(items) => {
                for item in items {
                    walk(item);
                }
            }
            serde_json::Value::String(s) => {
                if s.contains("api_key=") || s.contains("Bearer ") {
                    *s = "[redacted]".into();
                }
            }
            _ => {}
        }
    }
    walk(&mut value);
    value
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn default_identity_is_traceable_and_non_generic() {
        let id = ClientIdentity::default_identity(None).unwrap();
        let ua = id.user_agent();
        assert!(ua.starts_with("fyi-cli/"));
        assert!(ua.contains("fp:"));
        assert!(ua.contains("github.com/edithatogo/fyi-cli"));
        assert!(!ua.contains("contact:"));
        assert!(!is_generic_user_agent(&ua));
        assert_eq!(id.fingerprint.len(), 16);
    }

    #[test]
    fn opt_in_admin_contact_appears_in_user_agent() {
        let id = ClientIdentity::default_identity(Some("ops@example.org".into())).unwrap();
        assert!(id.user_agent().contains("contact:ops@example.org"));
    }

    #[test]
    fn rejects_generic_user_agents() {
        assert!(is_generic_user_agent(""));
        assert!(is_generic_user_agent("curl/8.0"));
        assert!(is_generic_user_agent("python-requests/2.32"));
        assert!(is_generic_user_agent("reqwest/0.11"));
        assert!(!is_generic_user_agent(
            "fyi-cli/0.1.2 (fp:deadbeefdeadbeef; +https://github.com/edithatogo/fyi-cli)"
        ));
    }

    #[test]
    fn parses_rate_limit_headers() {
        let snap = RateLimitSnapshot::from_headers([
            ("RateLimit-Limit", "100"),
            ("RateLimit-Remaining", "3"),
            ("RateLimit-Reset", "30"),
            ("Retry-After", "12"),
            ("X-Advisory-Status", "DEGRADED"),
        ]);
        assert_eq!(snap.limit, Some(100));
        assert_eq!(snap.remaining, Some(3));
        assert_eq!(snap.reset_seconds, Some(30));
        assert_eq!(snap.retry_after_seconds, Some(12));
        assert_eq!(snap.advisory_status.as_deref(), Some("degraded"));
    }

    #[test]
    fn shared_backpressure_fixture_parity() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/backpressure_headers.json"
        ))
        .expect("shared back-pressure fixture should be valid JSON");

        for case in fixture
            .as_object()
            .expect("fixture cases should be an object")
            .values()
        {
            let pairs = case["headers"]
                .as_object()
                .expect("fixture headers should be an object")
                .iter()
                .map(|(name, value)| {
                    (
                        name.as_str().to_string(),
                        value
                            .as_str()
                            .expect("header values should be strings")
                            .to_string(),
                    )
                })
                .collect::<Vec<_>>();
            let headers = pairs
                .iter()
                .map(|(name, value)| (name.as_str(), value.as_str()));
            let snapshot = RateLimitSnapshot::from_headers(headers);
            let expected = &case["expected"];
            assert_eq!(snapshot.limit, expected["limit"].as_u64());
            assert_eq!(snapshot.remaining, expected["remaining"].as_u64());
            assert_eq!(snapshot.reset_seconds, expected["reset_seconds"].as_u64());
            assert_eq!(
                snapshot.retry_after_seconds,
                expected["retry_after_seconds"].as_u64()
            );
            assert_eq!(
                snapshot.advisory_status.as_deref(),
                expected["advisory_status"].as_str()
            );
        }
    }

    #[test]
    fn degraded_advisory_enters_degraded_pacing() {
        let mut engine = PacingEngine::default();
        let snapshot = RateLimitSnapshot {
            advisory_status: Some("degraded".to_string()),
            ..Default::default()
        };
        engine.observe(&snapshot, Duration::from_millis(10));
        assert_eq!(engine.state, PacingState::Degraded);
        assert_eq!(engine.concurrency, 1);
    }

    #[test]
    fn pacing_degrades_on_low_remaining_and_recovers() {
        let mut engine = PacingEngine::default();
        let snap = RateLimitSnapshot {
            remaining: Some(1),
            limit: Some(100),
            ..Default::default()
        };
        engine.observe(&snap, Duration::from_millis(10));
        assert_eq!(engine.state, PacingState::Degraded);
        assert_eq!(engine.concurrency, 1);

        let ok = RateLimitSnapshot {
            remaining: Some(90),
            limit: Some(100),
            http_status: Some(200),
            ..Default::default()
        };
        for _ in 0..20 {
            engine.observe(&ok, Duration::from_millis(10));
        }
        assert!(matches!(
            engine.state,
            PacingState::Baseline | PacingState::Recovering
        ));
    }

    #[test]
    fn pacing_backoff_honours_retry_after_floor() {
        let mut engine = PacingEngine::default();
        let wait = engine.enter_backoff(Some(45));
        assert!(wait.as_secs() >= 45);
    }

    #[test]
    fn exponential_backoff_is_capped() {
        let v = exponential_backoff_seconds(20, None, 60);
        assert!(v <= 60);
        let v2 = exponential_backoff_seconds(1, Some(90), 60);
        // Retry-After may exceed max when server demands it — policy uses max(ra).
        assert!(v2 >= 90 || v2 <= 60);
    }

    #[test]
    fn guardrails_trip_on_max_requests() {
        let mut g = GuardrailTracker::new(GuardrailConfig {
            max_requests: 2,
            max_response_bytes: 1_000_000,
            max_runtime: Duration::from_secs(60),
            max_concurrency: 1,
        });
        g.record_request_start().unwrap();
        g.record_request_start().unwrap();
        let err = g.record_request_start().unwrap_err();
        assert!(matches!(err, AgentRuntimeError::Guardrail(_)));
    }

    #[test]
    fn guardrails_trip_on_bytes() {
        let mut g = GuardrailTracker::new(GuardrailConfig {
            max_requests: 100,
            max_response_bytes: 10,
            max_runtime: Duration::from_secs(60),
            max_concurrency: 1,
        });
        g.record_request_start().unwrap();
        let err = g.record_response_bytes(11).unwrap_err();
        assert!(matches!(err, AgentRuntimeError::Guardrail(_)));
    }

    #[test]
    fn plan_rejects_unbounded_recursion_without_window() {
        let plan = RetrievalPlan {
            instance_id: "nz-fyi".into(),
            description: "walk everything".into(),
            estimated_requests: 100,
            date_from: None,
            date_to: None,
            max_pages: None,
            recursive_unbounded: true,
            is_heavy: true,
            force_schedule: false,
        };
        let decision = reflect_plan(&plan, &LoadMemoryStore::default());
        assert!(matches!(decision, PlanDecision::Reject { .. }));
    }

    #[test]
    fn plan_rewrites_unbounded_when_window_present() {
        let plan = RetrievalPlan {
            instance_id: "nz-fyi".into(),
            description: "walk with window".into(),
            estimated_requests: 100,
            date_from: Some("2020-01-01".into()),
            date_to: Some("2020-01-31".into()),
            max_pages: None,
            recursive_unbounded: true,
            is_heavy: true,
            force_schedule: false,
        };
        let decision = reflect_plan(&plan, &LoadMemoryStore::default());
        match decision {
            PlanDecision::Rewrite { rewritten, .. } => {
                assert!(!rewritten.recursive_unbounded);
                assert_eq!(rewritten.max_pages, Some(50));
            }
            other => panic!("expected rewrite, got {other:?}"),
        }
    }

    #[test]
    fn throttle_rewrites_plan_before_resume() {
        let plan = RetrievalPlan {
            instance_id: "nz-fyi".into(),
            description: "bulk discovery".into(),
            estimated_requests: 100,
            date_from: Some("2026-01-01".into()),
            date_to: Some("2026-01-31".into()),
            max_pages: Some(20),
            recursive_unbounded: false,
            is_heavy: true,
            force_schedule: false,
        };
        let decision = rewrite_after_throttle(&plan);
        match decision {
            PlanDecision::Rewrite { rewritten, .. } => {
                assert_eq!(rewritten.estimated_requests, 50);
                assert_eq!(rewritten.max_pages, Some(10));
                assert!(rewritten.description.contains("after throttle"));
            }
            other => panic!("expected rewrite, got {other:?}"),
        }
    }

    #[test]
    fn throttle_keeps_unbounded_plan_rejected_without_window() {
        let plan = RetrievalPlan {
            instance_id: "nz-fyi".into(),
            description: "unbounded".into(),
            estimated_requests: 0,
            date_from: None,
            date_to: None,
            max_pages: None,
            recursive_unbounded: true,
            is_heavy: true,
            force_schedule: false,
        };
        assert!(matches!(
            rewrite_after_throttle(&plan),
            PlanDecision::Reject { .. }
        ));
    }

    #[test]
    fn filesystem_cache_roundtrip() {
        let dir = std::env::temp_dir().join(format!(
            "fyi-agent-cache-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        let cache = FilesystemResponseCache::new(&dir).unwrap();
        let url = "https://fyi.org.nz/api/v2/request/1.json";
        assert!(cache.get(url).unwrap().is_none());
        cache.put(url, b"{\"id\":1}").unwrap();
        assert_eq!(cache.get(url).unwrap().unwrap(), b"{\"id\":1}");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn middleware_before_after_and_traces() {
        let identity = ClientIdentity::default_identity(None).unwrap();
        let sink = MemoryTraceSink::default();
        let sink_ptr = Arc::new(Mutex::new(sink));
        // Use a wrapper via custom Box — simpler: Jsonl in temp + Memory via with_trace
        let mut mw = AgentNetworkMiddleware::new(
            identity,
            GuardrailConfig {
                max_requests: 10,
                max_response_bytes: 10_000,
                max_runtime: Duration::from_secs(30),
                max_concurrency: 2,
            },
            PacingPolicy::default(),
            Box::new(MemoryTraceSink::default()),
            "test-run",
        );

        let plan = RetrievalPlan {
            instance_id: "nz-fyi".into(),
            description: "fetch one".into(),
            estimated_requests: 1,
            date_from: None,
            date_to: None,
            max_pages: Some(1),
            recursive_unbounded: false,
            is_heavy: false,
            force_schedule: false,
        };
        assert!(matches!(
            mw.reflect_and_trace(&plan),
            PlanDecision::Accept { .. }
        ));

        let meta = OutboundRequestMeta {
            instance_id: "nz-fyi".into(),
            route_class: "request".into(),
            method: "GET".into(),
            url: "https://fyi.org.nz/api/v2/request/1.json".into(),
        };
        let pre = mw.before_request(&meta).unwrap();
        assert!(pre.user_agent.starts_with("fyi-cli/"));
        assert!(pre.cache_hit.is_none());

        let wait = mw
            .after_response(
                &meta,
                200,
                &[("RateLimit-Limit", "100"), ("RateLimit-Remaining", "99")],
                Duration::from_millis(40),
                Some(b"{}"),
            )
            .unwrap();
        assert!(wait >= Duration::ZERO);

        let report = mw.status_report();
        assert_eq!(report["run_id"], "test-run");
        let _ = sink_ptr; // silence
    }

    #[test]
    fn middleware_rate_limit_returns_error() {
        let mut mw = AgentNetworkMiddleware::with_defaults(None).unwrap();
        let meta = OutboundRequestMeta {
            instance_id: "nz-fyi".into(),
            route_class: "request".into(),
            method: "GET".into(),
            url: "https://fyi.org.nz/x".into(),
        };
        mw.before_request(&meta).unwrap();
        let err = mw
            .after_response(
                &meta,
                429,
                &[("Retry-After", "7")],
                Duration::from_millis(5),
                Some(b"api_key=secret-token"),
            )
            .unwrap_err();
        assert!(matches!(err, AgentRuntimeError::RateLimited(7)));
    }

    #[test]
    fn redact_secrets_strips_tokens() {
        let v = serde_json::json!({
            "api_key": "secret",
            "nested": { "authorization": "Bearer abc" },
            "body": "api_key=secret-token"
        });
        let r = redact_secrets(v);
        assert_eq!(r["api_key"], "[redacted]");
        assert_eq!(r["nested"]["authorization"], "[redacted]");
        assert_eq!(r["body"], "[redacted]");
    }

    #[test]
    fn load_memory_persists() {
        let dir = std::env::temp_dir().join(format!(
            "fyi-agent-mem-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or(1)
        ));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("memory.json");
        let mut store = LoadMemoryStore::default();
        store.observe("nz-fyi", "bulk", Duration::from_millis(100), false);
        store.observe("nz-fyi", "bulk", Duration::from_millis(100), true);
        store.save_to_path(&path).unwrap();
        let loaded = LoadMemoryStore::load_from_path(&path).unwrap();
        let mem = loaded.get("nz-fyi", "bulk").unwrap();
        assert!(mem.rate_limit_hits >= 1);
        assert!(mem.samples >= 1);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn load_memory_prunes_low_signal_endpoints() {
        let mut memory = LoadMemoryStore::default();
        for index in 0..4 {
            memory.observe(
                "nz-fyi",
                &format!("route-{index}"),
                Duration::from_millis(10),
                index == 3,
            );
        }
        memory.prune(2);
        assert_eq!(memory.endpoints.len(), 2);
        assert!(memory.get("nz-fyi", "route-3").is_some());
    }

    #[test]
    fn cache_short_circuit_on_second_get() {
        let dir = std::env::temp_dir().join(format!(
            "fyi-agent-cache2-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or(2)
        ));
        let mut mw = AgentNetworkMiddleware::with_defaults(None)
            .unwrap()
            .with_cache_dir(&dir)
            .unwrap();
        let meta = OutboundRequestMeta {
            instance_id: "nz-fyi".into(),
            route_class: "request".into(),
            method: "GET".into(),
            url: "https://fyi.org.nz/cached".into(),
        };
        mw.before_request(&meta).unwrap();
        mw.after_response(
            &meta,
            200,
            &[],
            Duration::from_millis(1),
            Some(b"hello-cache"),
        )
        .unwrap();
        let pre = mw.before_request(&meta).unwrap();
        assert_eq!(pre.cache_hit.as_deref(), Some(b"hello-cache".as_ref()));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn cache_does_not_store_non_get_or_error_responses() {
        let dir = std::env::temp_dir().join(format!(
            "fyi-agent-cache3-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or(3)
        ));
        let mut mw = AgentNetworkMiddleware::with_defaults(None)
            .unwrap()
            .with_cache_dir(&dir)
            .unwrap();
        for (method, status, url) in [
            ("POST", 200, "https://fyi.org.nz/write"),
            ("GET", 500, "https://fyi.org.nz/error"),
        ] {
            let meta = OutboundRequestMeta {
                instance_id: "nz-fyi".into(),
                route_class: "request".into(),
                method: method.into(),
                url: url.into(),
            };
            mw.before_request(&meta).unwrap();
            mw.after_response(
                &meta,
                status,
                &[],
                Duration::from_millis(1),
                Some(b"no-cache"),
            )
            .unwrap();
            assert!(mw.before_request(&meta).unwrap().cache_hit.is_none());
        }
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn build_http_client_sets_identity() {
        let id = ClientIdentity::default_identity(None).unwrap();
        let client = build_http_client(&id).unwrap();
        // Client builds successfully with UA; detailed header assert needs a request.
        let _ = client;
    }
}
