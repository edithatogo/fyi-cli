//! Fail-closed policy and operator controls for remote MCP operations.
//!
//! This module requires no telemetry backend. Callers can export
//! the bounded status and audit values to their preferred local system without
//! sending request content, credentials, or user data anywhere.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const DEFAULT_MAX_REQUESTS: u32 = 20;
const DEFAULT_MAX_BYTES: u64 = 4 * 1024 * 1024;
const DEFAULT_MAX_DURATION_SECONDS: u64 = 120;
const MAX_INSTANCES: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteCapability {
    Read,
    Write,
}

impl RemoteCapability {
    fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemoteBudget {
    pub max_requests: u32,
    pub max_bytes: u64,
    pub max_duration_seconds: u64,
}

impl Default for RemoteBudget {
    fn default() -> Self {
        Self {
            max_requests: DEFAULT_MAX_REQUESTS,
            max_bytes: DEFAULT_MAX_BYTES,
            max_duration_seconds: DEFAULT_MAX_DURATION_SECONDS,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemoteInstancePolicy {
    pub instance_id: String,
    pub base_url: String,
    pub read_enabled: bool,
    pub write_enabled: bool,
    pub budget: RemoteBudget,
}

impl RemoteInstancePolicy {
    /// Protected server-side credential name; never reads or exposes its value.
    /// Policy construction rejects aliases of this legacy operator-facing name.
    pub fn bot_token_env_name(&self) -> String {
        format!(
            "FYI_MCP_REMOTE_BOT_TOKEN_{}",
            self.instance_id.to_ascii_uppercase().replace('-', "_")
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemotePolicyStatus {
    pub schema_version: u8,
    pub remote_enabled: bool,
    pub kill_switch: bool,
    pub degraded: bool,
    pub circuit_open: bool,
    pub instances: Vec<RemoteInstanceStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemoteInstanceStatus {
    pub instance_id: String,
    pub read_enabled: bool,
    pub write_enabled: bool,
    pub budget: RemoteBudget,
    pub requests_used: u32,
    pub bytes_used: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemoteAuditEvent {
    pub schema_version: u8,
    pub correlation_id: String,
    pub operation: String,
    pub instance_id: String,
    pub outcome: String,
    pub error_class: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyError {
    InvalidConfiguration(String),
    Disabled(String),
    BudgetExceeded(String),
}

impl std::fmt::Display for PolicyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidConfiguration(message)
            | Self::Disabled(message)
            | Self::BudgetExceeded(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for PolicyError {}

#[derive(Debug)]
struct ControlState {
    kill_switch: bool,
    degraded: bool,
    consecutive_failures: u32,
    circuit_open_until: Option<Instant>,
    usage: BTreeMap<String, (u32, u64)>,
    in_flight: BTreeSet<String>,
}

#[derive(Debug, Clone)]
pub struct RemoteMcpPolicy {
    instances: BTreeMap<String, RemoteInstancePolicy>,
    state: Arc<Mutex<ControlState>>,
    pub(crate) telemetry: super::telemetry::RemoteTelemetry,
    circuit_failure_threshold: u32,
    circuit_cooldown: Duration,
}

/// One admitted remote operation. Dropping it never refunds the request attempt.
///
/// A permit reserves one active dispatch for its instance until it is consumed
/// by `finish` or dropped on an error, timeout or cancellation. The caller must
/// bound the complete remote future with `duration_budget`; `finish` also
/// rejects a late result so a delayed timeout cannot publish an overrun.
#[derive(Debug)]
#[must_use = "hold the permit during the remote operation and finish it before returning data"]
pub struct RemoteRequestPermit {
    state: Arc<Mutex<ControlState>>,
    instance: RemoteInstancePolicy,
    capability: RemoteCapability,
    started_at: Instant,
    duration_budget: Duration,
}

impl RemoteRequestPermit {
    /// Maximum wall-clock duration of the entire admitted remote operation.
    pub fn duration_budget(&self) -> Duration {
        self.duration_budget
    }

    /// Account for the serialized returned payload before publishing it.
    ///
    /// Bytes remain counted even when an overrun, overflow or policy change
    /// rejects the result. This is an MCP result budget, not a raw HTTP body or
    /// allocator limit; the source transport retains those separate bounds.
    pub fn finish(self, bytes: u64) -> Result<(), PolicyError> {
        self.finish_with_clock(bytes, Instant::now)
    }

    #[cfg(test)]
    fn finish_at(self, bytes: u64, now: Instant) -> Result<(), PolicyError> {
        self.finish_with_clock(bytes, || now)
    }

    fn finish_with_clock(
        self,
        bytes: u64,
        clock: impl FnOnce() -> Instant,
    ) -> Result<(), PolicyError> {
        let instance = &self.instance;
        let mut state = self.state.lock().map_err(|_| policy_unavailable())?;
        let now = clock();
        let usage = state.usage.entry(instance.instance_id.clone()).or_default();
        let next_bytes = usage.1.checked_add(bytes);
        usage.1 = next_bytes.unwrap_or(u64::MAX);
        if next_bytes.is_none() || usage.1 > instance.budget.max_bytes {
            return Err(PolicyError::BudgetExceeded(
                "remote response byte budget exhausted".into(),
            ));
        }
        let elapsed = now.checked_duration_since(self.started_at);
        if elapsed.is_none_or(|elapsed| elapsed >= self.duration_budget) {
            return Err(PolicyError::BudgetExceeded(
                "remote operation duration budget exhausted".into(),
            ));
        }
        authorize_controls(instance, self.capability, &state, now)
    }
}

impl Drop for RemoteRequestPermit {
    fn drop(&mut self) {
        // A poisoned state already rejects every new permit. Drop must not
        // panic while another failure or cancellation is unwinding.
        if let Ok(mut state) = self.state.lock() {
            state.in_flight.remove(&self.instance.instance_id);
        }
    }
}

impl RemoteMcpPolicy {
    pub fn new(instances: Vec<RemoteInstancePolicy>) -> Result<Self, PolicyError> {
        if instances.len() > MAX_INSTANCES {
            return Err(PolicyError::InvalidConfiguration(
                "too many remote instances".into(),
            ));
        }
        let mut map = BTreeMap::new();
        let mut credential_names = BTreeSet::new();
        for instance in instances {
            validate_instance(&instance)?;
            let credential_name = instance.bot_token_env_name();
            if map.insert(instance.instance_id.clone(), instance).is_some() {
                return Err(PolicyError::InvalidConfiguration(
                    "duplicate remote instance id".into(),
                ));
            }
            if !credential_names.insert(credential_name) {
                return Err(PolicyError::InvalidConfiguration(
                    "remote instance ids must use distinct credential environment variables".into(),
                ));
            }
        }
        Ok(Self {
            instances: map,
            telemetry: super::telemetry::RemoteTelemetry::default(),
            state: Arc::new(Mutex::new(ControlState {
                kill_switch: false,
                degraded: false,
                consecutive_failures: 0,
                circuit_open_until: None,
                usage: BTreeMap::new(),
                in_flight: BTreeSet::new(),
            })),
            circuit_failure_threshold: 3,
            circuit_cooldown: Duration::from_secs(30),
        })
    }

    /// Load the explicit allowlist from `FYI_MCP_REMOTE_INSTANCES`.
    ///
    /// The format is `id=https://host,id2=https://other`.  An absent value is
    /// a valid, secure local-only configuration.  Any malformed value fails
    /// startup rather than being partially accepted.
    pub fn from_env() -> Result<Self, PolicyError> {
        let degraded =
            degraded_startup_value(std::env::var_os("FYI_MCP_REMOTE_DEGRADED").as_deref())?;
        let policy = Self::instances_from_env()?;
        policy.set_degraded(degraded);
        Ok(policy)
    }

    fn instances_from_env() -> Result<Self, PolicyError> {
        let raw = std::env::var("FYI_MCP_REMOTE_INSTANCES").unwrap_or_default();
        if raw.trim().is_empty() {
            return Self::new(Vec::new());
        }
        let read = csv_env("FYI_MCP_REMOTE_READ")?;
        let write = csv_env("FYI_MCP_REMOTE_WRITE")?;
        let configured_ids = raw
            .split(',')
            .filter_map(|item| item.split_once('=').map(|(id, _)| id.trim().to_string()))
            .collect::<BTreeSet<_>>();
        if !read.is_subset(&configured_ids) || !write.is_subset(&configured_ids) {
            return Err(PolicyError::InvalidConfiguration(
                "remote capability references an instance outside the allowlist".into(),
            ));
        }
        let mut instances = Vec::new();
        for item in raw.split(',') {
            let (id, url) = item.split_once('=').ok_or_else(|| {
                PolicyError::InvalidConfiguration("remote instance must be id=https-url".into())
            })?;
            let id = id.trim().to_string();
            let url = url.trim().to_string();
            instances.push(RemoteInstancePolicy {
                read_enabled: read.contains(&id),
                write_enabled: write.contains(&id),
                instance_id: id,
                base_url: url,
                budget: RemoteBudget::default(),
            });
        }
        Self::new(instances)
    }

    pub fn authorize(
        &self,
        instance_id: &str,
        capability: RemoteCapability,
    ) -> Result<(), PolicyError> {
        let instance = self.instances.get(instance_id).ok_or_else(|| {
            PolicyError::Disabled("remote instance is not in the explicit allowlist".into())
        })?;
        let state = self.state.lock().map_err(|_| policy_unavailable())?;
        authorize_controls(instance, capability, &state, Instant::now())?;
        authorize_budget(instance, &state)
    }

    /// Authorize retrieval of an already completed result without granting I/O.
    ///
    /// Replay consumes no dispatch allowance, but must still obey the configured
    /// capability, kill switch, degraded mode and circuit breaker.
    pub fn authorize_cached_result(
        &self,
        instance_id: &str,
        capability: RemoteCapability,
    ) -> Result<(), PolicyError> {
        let instance = self.instances.get(instance_id).ok_or_else(|| {
            PolicyError::Disabled("remote instance is not in the explicit allowlist".into())
        })?;
        let state = self.state.lock().map_err(|_| policy_unavailable())?;
        authorize_controls(instance, capability, &state, Instant::now())
    }

    /// Atomically authorize and reserve one remote dispatch before any I/O.
    ///
    /// Only one operation can be active per configured instance. Each admitted
    /// attempt consumes its request allowance, including attempts that later
    /// fail or time out. Rejected callers do not dispatch and are not counted.
    pub fn begin_request(
        &self,
        instance_id: &str,
        capability: RemoteCapability,
    ) -> Result<RemoteRequestPermit, PolicyError> {
        self.begin_request_with_clock(instance_id, capability, None, Instant::now)
    }

    /// Reserve a dispatch only while its confirmation deadline remains valid.
    ///
    /// The clock is sampled after acquiring the policy lock, so a confirmation
    /// that expires during contention consumes neither a slot nor an attempt.
    pub fn begin_request_before(
        &self,
        instance_id: &str,
        capability: RemoteCapability,
        deadline: Instant,
    ) -> Result<RemoteRequestPermit, PolicyError> {
        self.begin_request_with_clock(instance_id, capability, Some(deadline), Instant::now)
    }

    #[cfg(test)]
    fn begin_request_at(
        &self,
        instance_id: &str,
        capability: RemoteCapability,
        now: Instant,
    ) -> Result<RemoteRequestPermit, PolicyError> {
        self.begin_request_with_clock(instance_id, capability, None, || now)
    }

    fn begin_request_with_clock(
        &self,
        instance_id: &str,
        capability: RemoteCapability,
        deadline: Option<Instant>,
        clock: impl FnOnce() -> Instant,
    ) -> Result<RemoteRequestPermit, PolicyError> {
        let instance = self.instances.get(instance_id).ok_or_else(|| {
            PolicyError::Disabled("remote instance is not in the explicit allowlist".into())
        })?;
        let mut state = self.state.lock().map_err(|_| policy_unavailable())?;
        let now = clock();
        if deadline.is_some_and(|deadline| now >= deadline) {
            return Err(PolicyError::Disabled(
                "remote operation deadline has expired".into(),
            ));
        }
        authorize_controls(instance, capability, &state, now)?;
        authorize_budget(instance, &state)?;
        if state.in_flight.contains(instance_id) {
            return Err(PolicyError::BudgetExceeded(
                "remote instance concurrency budget exhausted".into(),
            ));
        }
        let usage = state.usage.entry(instance_id.to_string()).or_default();
        // authorize_budget checked the u32 bound before the same-lock update.
        usage.0 += 1;
        state.in_flight.insert(instance_id.to_string());
        Ok(RemoteRequestPermit {
            state: Arc::clone(&self.state),
            instance: instance.clone(),
            capability,
            started_at: now,
            duration_budget: Duration::from_secs(instance.budget.max_duration_seconds),
        })
    }

    pub fn instance(&self, instance_id: &str) -> Option<RemoteInstancePolicy> {
        self.instances.get(instance_id).cloned()
    }

    /// Legacy post-dispatch accounting. New callers should use `begin_request`.
    ///
    /// Count observed requests and bytes even when they exceed a limit, so
    /// oversized responses cannot leave a fresh allowance behind.
    pub fn record_request(&self, instance_id: &str, bytes: u64) -> Result<(), PolicyError> {
        let instance = self.instances.get(instance_id).ok_or_else(|| {
            PolicyError::Disabled("remote instance is not in the explicit allowlist".into())
        })?;
        let mut state = self.state.lock().map_err(|_| policy_unavailable())?;
        let usage = state.usage.entry(instance_id.to_string()).or_default();
        let next_requests = usage.0.checked_add(1);
        let next_bytes = usage.1.checked_add(bytes);
        usage.0 = next_requests.unwrap_or(u32::MAX);
        usage.1 = next_bytes.unwrap_or(u64::MAX);
        if next_requests.is_none()
            || next_bytes.is_none()
            || usage.0 > instance.budget.max_requests
            || usage.1 > instance.budget.max_bytes
        {
            return Err(PolicyError::BudgetExceeded(
                "remote instance budget exhausted".into(),
            ));
        }
        Ok(())
    }

    pub fn set_kill_switch(&self, enabled: bool) {
        self.state
            .lock()
            .expect("policy mutex poisoned")
            .kill_switch = enabled;
    }

    pub fn set_degraded(&self, enabled: bool) {
        self.state.lock().expect("policy mutex poisoned").degraded = enabled;
    }

    pub fn record_failure(&self) {
        self.record_failure_at(Instant::now());
    }

    fn record_failure_at(&self, now: Instant) {
        let mut state = self.state.lock().expect("policy mutex poisoned");
        state.consecutive_failures = state.consecutive_failures.saturating_add(1);
        if state.consecutive_failures >= self.circuit_failure_threshold {
            state.circuit_open_until = now.checked_add(self.circuit_cooldown);
            if state.circuit_open_until.is_none() {
                state.kill_switch = true;
            }
        }
    }

    pub fn record_success(&self) {
        let mut state = self.state.lock().expect("policy mutex poisoned");
        state.consecutive_failures = 0;
        state.circuit_open_until = None;
    }

    pub fn status(&self) -> RemotePolicyStatus {
        let state = self.state.lock().expect("policy mutex poisoned");
        let circuit_open = state
            .circuit_open_until
            .is_some_and(|until| until > Instant::now());
        RemotePolicyStatus {
            schema_version: 1,
            remote_enabled: !self.instances.is_empty(),
            kill_switch: state.kill_switch,
            degraded: state.degraded,
            circuit_open,
            instances: self
                .instances
                .values()
                .map(|instance| {
                    let (requests_used, bytes_used) = state
                        .usage
                        .get(&instance.instance_id)
                        .copied()
                        .unwrap_or_default();
                    RemoteInstanceStatus {
                        instance_id: instance.instance_id.clone(),
                        read_enabled: instance.read_enabled,
                        write_enabled: instance.write_enabled,
                        budget: instance.budget.clone(),
                        requests_used,
                        bytes_used,
                    }
                })
                .collect(),
        }
    }

    pub fn audit_event(
        &self,
        correlation_id: &str,
        operation: &str,
        instance_id: &str,
        outcome: &str,
        error_class: Option<&str>,
    ) -> RemoteAuditEvent {
        RemoteAuditEvent {
            schema_version: 1,
            correlation_id: safe_token(correlation_id),
            operation: safe_token(operation),
            instance_id: safe_token(instance_id),
            outcome: safe_token(outcome),
            error_class: error_class.map(safe_token),
        }
    }
}

fn policy_unavailable() -> PolicyError {
    PolicyError::Disabled("remote policy state is unavailable".into())
}

fn authorize_controls(
    instance: &RemoteInstancePolicy,
    capability: RemoteCapability,
    state: &ControlState,
    now: Instant,
) -> Result<(), PolicyError> {
    if state.kill_switch {
        return Err(PolicyError::Disabled(
            "remote MCP kill switch is active".into(),
        ));
    }
    if state.degraded && capability == RemoteCapability::Write {
        return Err(PolicyError::Disabled(
            "remote MCP is in degraded read-only mode".into(),
        ));
    }
    if state.circuit_open_until.is_some_and(|until| until > now) {
        return Err(PolicyError::Disabled(
            "remote MCP circuit breaker is open".into(),
        ));
    }
    let enabled = match capability {
        RemoteCapability::Read => instance.read_enabled,
        RemoteCapability::Write => instance.write_enabled,
    };
    if !enabled {
        return Err(PolicyError::Disabled(format!(
            "remote {} capability is disabled for instance",
            capability.as_str()
        )));
    }
    Ok(())
}

fn authorize_budget(
    instance: &RemoteInstancePolicy,
    state: &ControlState,
) -> Result<(), PolicyError> {
    let instance_id = &instance.instance_id;
    let (requests, bytes) = state.usage.get(instance_id).copied().unwrap_or_default();
    if requests >= instance.budget.max_requests || bytes >= instance.budget.max_bytes {
        return Err(PolicyError::BudgetExceeded(
            "remote instance budget exhausted".into(),
        ));
    }
    Ok(())
}

/// An invalid operator control must never silently restore write capability.
fn degraded_startup_value(value: Option<&OsStr>) -> Result<bool, PolicyError> {
    match value {
        None => Ok(false),
        Some(value) if value == "true" || value == "1" => Ok(true),
        Some(value) if value == "false" || value == "0" => Ok(false),
        _ => Err(PolicyError::InvalidConfiguration(
            "FYI_MCP_REMOTE_DEGRADED must be true, false, 1, or 0".into(),
        )),
    }
}

fn csv_env(name: &str) -> Result<BTreeSet<String>, PolicyError> {
    let raw = std::env::var(name).unwrap_or_default();
    Ok(raw
        .split(',')
        .filter(|value| !value.trim().is_empty())
        .map(|value| value.trim().to_string())
        .collect())
}

fn validate_instance(instance: &RemoteInstancePolicy) -> Result<(), PolicyError> {
    if instance.instance_id.is_empty()
        || instance.instance_id.len() > 64
        || !instance
            .instance_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
    {
        return Err(PolicyError::InvalidConfiguration(
            "invalid remote instance id".into(),
        ));
    }
    if !instance.base_url.starts_with("https://")
        || instance.base_url.contains('@')
        || instance.base_url.contains('*')
        || instance.base_url.ends_with('/')
    {
        return Err(PolicyError::InvalidConfiguration(
            "remote instance must use an explicit credential-free https URL".into(),
        ));
    }
    if instance.budget.max_requests == 0
        || instance.budget.max_bytes == 0
        || instance.budget.max_duration_seconds == 0
    {
        return Err(PolicyError::InvalidConfiguration(
            "remote budgets must be positive".into(),
        ));
    }
    Ok(())
}

fn safe_token(value: &str) -> String {
    value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ':'))
        .take(128)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn degraded_startup_control_rejects_ambiguous_or_private_values() {
        assert_eq!(degraded_startup_value(None), Ok(false));
        for (value, enabled) in [("true", true), ("1", true), ("false", false), ("0", false)] {
            assert_eq!(degraded_startup_value(Some(OsStr::new(value))), Ok(enabled));
        }
        for value in [
            "",
            "TRUE",
            "yes",
            "2",
            " false",
            "false\nsynthetic-private-value",
        ] {
            assert_eq!(
                degraded_startup_value(Some(OsStr::new(value))),
                Err(PolicyError::InvalidConfiguration(
                    "FYI_MCP_REMOTE_DEGRADED must be true, false, 1, or 0".into()
                ))
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn non_unicode_degraded_control_is_not_treated_as_unset() {
        use std::os::unix::ffi::OsStrExt;
        assert_eq!(
            degraded_startup_value(Some(OsStr::from_bytes(b"synthetic-private-\xff"))),
            Err(PolicyError::InvalidConfiguration(
                "FYI_MCP_REMOTE_DEGRADED must be true, false, 1, or 0".into()
            ))
        );
    }

    fn instance(read: bool, write: bool) -> RemoteInstancePolicy {
        RemoteInstancePolicy {
            instance_id: "nz".into(),
            base_url: "https://foi-repository.example".into(),
            read_enabled: read,
            write_enabled: write,
            budget: RemoteBudget {
                max_requests: 1,
                max_bytes: 10,
                max_duration_seconds: 1,
            },
        }
    }

    #[test]
    fn default_policy_is_local_only_and_fail_closed() {
        let policy = RemoteMcpPolicy::new(Vec::new()).unwrap();
        assert!(!policy.status().remote_enabled);
        assert!(matches!(
            policy.authorize("nz", RemoteCapability::Read),
            Err(PolicyError::Disabled(_))
        ));
    }

    #[test]
    fn read_enablement_does_not_enable_write() {
        let policy = RemoteMcpPolicy::new(vec![instance(true, false)]).unwrap();
        assert!(policy.authorize("nz", RemoteCapability::Read).is_ok());
        assert!(policy.authorize("nz", RemoteCapability::Write).is_err());
    }

    #[test]
    fn kill_switch_and_degraded_mode_are_deterministic() {
        let policy = RemoteMcpPolicy::new(vec![instance(true, true)]).unwrap();
        policy.set_degraded(true);
        assert!(policy.authorize("nz", RemoteCapability::Read).is_ok());
        assert!(policy.authorize("nz", RemoteCapability::Write).is_err());
        policy.set_kill_switch(true);
        assert!(policy.authorize("nz", RemoteCapability::Read).is_err());
    }

    #[test]
    fn budget_and_circuit_breaker_fail_closed() {
        let policy = RemoteMcpPolicy::new(vec![instance(true, false)]).unwrap();
        policy.record_failure();
        policy.record_failure();
        policy.record_failure();
        assert!(policy.status().circuit_open);
        policy.record_success();
        assert!(!policy.status().circuit_open);
        policy.record_request("nz", 10).unwrap();
        assert!(matches!(
            policy.authorize("nz", RemoteCapability::Read),
            Err(PolicyError::BudgetExceeded(_))
        ));
    }

    #[test]
    fn audit_event_contains_only_bounded_safe_tokens() {
        let policy = RemoteMcpPolicy::new(Vec::new()).unwrap();
        let event = policy.audit_event("corr secret/1", "read body", "nz/1", "ok", Some("401 pii"));
        assert_eq!(event.correlation_id, "corrsecret1");
        assert_eq!(event.operation, "readbody");
        assert_eq!(event.error_class.as_deref(), Some("401pii"));
        assert!(!serde_json::to_string(&event).unwrap().contains("secret/"));
    }

    #[test]
    fn rejects_broad_or_credential_bearing_urls() {
        assert!(RemoteMcpPolicy::new(vec![instance(true, false)]).is_ok());
        let mut bad = instance(true, false);
        bad.base_url = "https://user:pass@example.com".into();
        assert!(RemoteMcpPolicy::new(vec![bad]).is_err());
    }

    #[test]
    fn credential_mappings_reject_generated_case_and_separator_aliases() {
        // Each group maps to the same protected environment variable. Exercise
        // both configuration orders and every pair of write-capability flags.
        for prefix in ["a", "z", "0", "region1"] {
            let aliases = [
                format!("{prefix}-api"),
                format!("{prefix}_api"),
                format!("{prefix}-API"),
                format!("{prefix}_API"),
                format!("{prefix}-aPi"),
                format!("{prefix}_aPi"),
            ];
            for (index, left) in aliases.iter().enumerate() {
                for right in &aliases[index + 1..] {
                    for (first, second) in [(left, right), (right, left)] {
                        for (first_write, second_write) in
                            [(false, false), (false, true), (true, false), (true, true)]
                        {
                            let mut first_instance = instance(true, first_write);
                            first_instance.instance_id = first.clone();
                            first_instance.base_url = "https://first.example".into();
                            let mut second_instance = instance(true, second_write);
                            second_instance.instance_id = second.clone();
                            second_instance.base_url = "https://second.example".into();
                            let error = RemoteMcpPolicy::new(vec![first_instance, second_instance])
                                .expect_err("credential aliases must fail before dispatch");
                            assert_eq!(
                                error,
                                PolicyError::InvalidConfiguration(
                                    "remote instance ids must use distinct credential environment variables"
                                        .into()
                                )
                            );
                            assert!(!error.to_string().contains(first));
                            assert!(!error.to_string().contains(second));
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn distinct_credential_mappings_preserve_instance_identity_and_capabilities() {
        for (first, second) in [
            ("north-foi", "south_foi"),
            ("a1", "a_1"),
            ("1a", "_1a"),
            ("MiXeD", "mixed2"),
        ] {
            let mut read_only = instance(true, false);
            read_only.instance_id = first.into();
            let mut write_only = instance(false, true);
            write_only.instance_id = second.into();
            let policy = RemoteMcpPolicy::new(vec![read_only, write_only]).unwrap();
            assert!(policy.authorize(first, RemoteCapability::Read).is_ok());
            assert!(policy.authorize(first, RemoteCapability::Write).is_err());
            assert!(policy.authorize(second, RemoteCapability::Write).is_ok());
            assert!(policy.authorize(second, RemoteCapability::Read).is_err());
            let ids: BTreeSet<_> = policy
                .status()
                .instances
                .into_iter()
                .map(|instance| instance.instance_id)
                .collect();
            assert_eq!(ids, BTreeSet::from([first.into(), second.into()]));
        }
    }

    #[test]
    fn credential_environment_names_preserve_existing_operator_keys() {
        for (id, expected) in [
            ("nz", "FYI_MCP_REMOTE_BOT_TOKEN_NZ"),
            ("nz-fyi", "FYI_MCP_REMOTE_BOT_TOKEN_NZ_FYI"),
            ("MiXeD_id-7", "FYI_MCP_REMOTE_BOT_TOKEN_MIXED_ID_7"),
        ] {
            let mut configured = instance(true, true);
            configured.instance_id = id.into();
            assert_eq!(configured.bot_token_env_name(), expected);
        }
    }

    fn bounded_policy(max_requests: u32, max_bytes: u64) -> RemoteMcpPolicy {
        let mut configured = instance(true, true);
        configured.budget.max_requests = max_requests;
        configured.budget.max_bytes = max_bytes;
        RemoteMcpPolicy::new(vec![configured]).unwrap()
    }

    #[test]
    fn reservation_counts_attempt_before_dispatch_and_drop_releases_slot() {
        let policy = bounded_policy(3, 10);
        let permit = policy.begin_request("nz", RemoteCapability::Read).unwrap();
        assert_eq!(permit.duration_budget(), Duration::from_secs(1));
        assert_eq!(policy.status().instances[0].requests_used, 1);
        assert!(policy.begin_request("nz", RemoteCapability::Read).is_err());
        assert_eq!(policy.status().instances[0].requests_used, 1);
        drop(permit);
        let second = policy.begin_request("nz", RemoteCapability::Read).unwrap();
        second.finish(1).unwrap();
        assert_eq!(policy.status().instances[0].requests_used, 2);
        assert_eq!(policy.status().instances[0].bytes_used, 1);
    }

    #[test]
    fn concurrent_reservations_admit_exactly_one_dispatch_per_instance() {
        let policy = bounded_policy(20, 100);
        let barrier = std::sync::Barrier::new(8);
        let admitted = std::thread::scope(|scope| {
            let workers = (0..8)
                .map(|_| {
                    scope.spawn(|| {
                        barrier.wait();
                        let result = policy.begin_request("nz", RemoteCapability::Read);
                        let admitted = result.is_ok();
                        barrier.wait();
                        drop(result);
                        admitted
                    })
                })
                .collect::<Vec<_>>();
            workers
                .into_iter()
                .map(|worker| usize::from(worker.join().unwrap()))
                .sum::<usize>()
        });
        assert_eq!(admitted, 1);
        assert_eq!(policy.status().instances[0].requests_used, 1);
        assert!(policy.begin_request("nz", RemoteCapability::Read).is_ok());
    }

    #[test]
    fn concurrency_is_independent_between_instances() {
        let first = instance(true, true);
        let mut second = first.clone();
        second.instance_id = "au".into();
        let policy = RemoteMcpPolicy::new(vec![first, second]).unwrap();
        let nz = policy.begin_request("nz", RemoteCapability::Read).unwrap();
        let au = policy.begin_request("au", RemoteCapability::Read).unwrap();
        nz.finish(2).unwrap();
        au.finish(3).unwrap();
        let status = policy.status();
        assert_eq!(status.instances[0].bytes_used, 3);
        assert_eq!(status.instances[1].bytes_used, 2);
    }

    #[test]
    fn dropped_and_failed_attempts_do_not_regain_request_allowance() {
        let policy = bounded_policy(2, 10);
        drop(policy.begin_request("nz", RemoteCapability::Read).unwrap());
        drop(policy.begin_request("nz", RemoteCapability::Read).unwrap());
        assert!(matches!(
            policy.begin_request("nz", RemoteCapability::Read),
            Err(PolicyError::BudgetExceeded(_))
        ));
        assert_eq!(policy.status().instances[0].requests_used, 2);
    }

    #[test]
    fn oversized_response_is_counted_and_exhausts_future_allowance() {
        let policy = bounded_policy(3, 10);
        let permit = policy.begin_request("nz", RemoteCapability::Read).unwrap();
        assert!(matches!(
            permit.finish(11),
            Err(PolicyError::BudgetExceeded(_))
        ));
        assert_eq!(policy.status().instances[0].requests_used, 1);
        assert_eq!(policy.status().instances[0].bytes_used, 11);
        assert!(policy.begin_request("nz", RemoteCapability::Read).is_err());
    }

    #[test]
    fn exact_budget_response_succeeds_but_prevents_further_requests() {
        let policy = bounded_policy(3, 10);
        policy
            .begin_request("nz", RemoteCapability::Read)
            .unwrap()
            .finish(10)
            .unwrap();
        assert!(policy.begin_request("nz", RemoteCapability::Read).is_err());
        assert_eq!(policy.status().instances[0].bytes_used, 10);
    }

    #[test]
    fn byte_overflow_saturates_and_cannot_reopen_budget() {
        let policy = bounded_policy(3, u64::MAX);
        policy
            .begin_request("nz", RemoteCapability::Read)
            .unwrap()
            .finish(u64::MAX - 1)
            .unwrap();
        let permit = policy.begin_request("nz", RemoteCapability::Read).unwrap();
        assert!(permit.finish(2).is_err());
        assert_eq!(policy.status().instances[0].requests_used, 2);
        assert_eq!(policy.status().instances[0].bytes_used, u64::MAX);
        assert!(policy.begin_request("nz", RemoteCapability::Read).is_err());
    }

    #[test]
    fn final_request_succeeds_without_counter_overflow() {
        let policy = bounded_policy(u32::MAX, 10);
        policy
            .state
            .lock()
            .unwrap()
            .usage
            .insert("nz".into(), (u32::MAX - 1, 0));
        policy
            .begin_request("nz", RemoteCapability::Read)
            .unwrap()
            .finish(1)
            .unwrap();
        assert_eq!(policy.status().instances[0].requests_used, u32::MAX);
        assert!(policy.begin_request("nz", RemoteCapability::Read).is_err());
    }

    #[test]
    fn duration_deadline_is_enforced_without_sleeping() {
        let policy = bounded_policy(3, 10);
        let start = Instant::now();
        let permit = policy
            .begin_request_at("nz", RemoteCapability::Read, start)
            .unwrap();
        assert!(permit.finish_at(1, start + Duration::from_secs(1)).is_err());
        assert_eq!(policy.status().instances[0].requests_used, 1);
        assert_eq!(policy.status().instances[0].bytes_used, 1);
        let next = policy
            .begin_request_at("nz", RemoteCapability::Read, start)
            .unwrap();
        next.finish_at(1, start + Duration::from_millis(999))
            .unwrap();
    }

    #[test]
    fn backwards_clock_sample_fails_closed_and_releases_slot() {
        let policy = bounded_policy(3, 10);
        let start = Instant::now();
        let permit = policy
            .begin_request_at("nz", RemoteCapability::Read, start + Duration::from_secs(1))
            .unwrap();
        assert!(permit.finish_at(1, start).is_err());
        assert!(policy.begin_request("nz", RemoteCapability::Read).is_ok());
    }

    #[test]
    fn reservations_obey_policy_controls_and_recheck_before_returning() {
        let policy = bounded_policy(5, 10);
        assert!(policy
            .begin_request("unknown", RemoteCapability::Read)
            .is_err());
        policy.set_kill_switch(true);
        assert!(policy.begin_request("nz", RemoteCapability::Read).is_err());
        policy.set_kill_switch(false);
        policy.set_degraded(true);
        assert!(policy.begin_request("nz", RemoteCapability::Write).is_err());
        policy
            .begin_request("nz", RemoteCapability::Read)
            .unwrap()
            .finish(1)
            .unwrap();
        policy.set_degraded(false);
        let write = policy.begin_request("nz", RemoteCapability::Write).unwrap();
        policy.set_degraded(true);
        assert!(write.finish(1).is_err());
        let read = policy.begin_request("nz", RemoteCapability::Read).unwrap();
        policy.set_kill_switch(true);
        assert!(read.finish(1).is_err());
        assert_eq!(policy.status().instances[0].requests_used, 3);
        assert_eq!(policy.status().instances[0].bytes_used, 3);
    }

    #[test]
    fn reservation_does_not_enable_an_unconfigured_capability() {
        let policy = RemoteMcpPolicy::new(vec![instance(true, false)]).unwrap();
        assert!(policy.begin_request("nz", RemoteCapability::Write).is_err());
        assert_eq!(policy.status().instances[0].requests_used, 0);
    }

    #[test]
    fn circuit_cooldown_and_recovery_are_deterministic() {
        let policy = bounded_policy(5, 10);
        let start = Instant::now();
        for _ in 0..3 {
            policy.record_failure_at(start);
        }
        assert!(policy
            .begin_request_at(
                "nz",
                RemoteCapability::Read,
                start + Duration::from_secs(29)
            )
            .is_err());
        let recovered = policy
            .begin_request_at(
                "nz",
                RemoteCapability::Read,
                start + Duration::from_secs(30),
            )
            .unwrap();
        recovered
            .finish_at(1, start + Duration::from_secs(30))
            .unwrap();
        policy.record_success();
        policy.record_failure_at(start + Duration::from_secs(30));
        assert!(policy
            .begin_request_at(
                "nz",
                RemoteCapability::Read,
                start + Duration::from_secs(30)
            )
            .is_ok());
    }

    #[test]
    fn legacy_accounting_cannot_drop_an_oversized_response() {
        let policy = bounded_policy(3, 10);
        assert!(policy.record_request("nz", 11).is_err());
        assert_eq!(policy.status().instances[0].requests_used, 1);
        assert_eq!(policy.status().instances[0].bytes_used, 11);
        assert!(policy.authorize("nz", RemoteCapability::Read).is_err());
    }

    #[test]
    fn cached_results_remain_available_after_the_final_dispatch_allowance() {
        let policy = bounded_policy(1, 10);
        policy
            .begin_request("nz", RemoteCapability::Write)
            .unwrap()
            .finish(10)
            .unwrap();
        assert!(policy
            .authorize_cached_result("nz", RemoteCapability::Write)
            .is_ok());
        assert!(policy.begin_request("nz", RemoteCapability::Write).is_err());
        assert!(policy.authorize("nz", RemoteCapability::Write).is_err());
        let status = policy.status();
        assert_eq!(status.instances[0].requests_used, 1);
        assert_eq!(status.instances[0].bytes_used, 10);
    }

    #[test]
    fn cached_results_still_obey_capabilities_and_all_operator_controls() {
        let read_only = RemoteMcpPolicy::new(vec![instance(true, false)]).unwrap();
        assert!(read_only
            .authorize_cached_result("nz", RemoteCapability::Write)
            .is_err());
        let policy = bounded_policy(1, 10);
        assert!(policy
            .authorize_cached_result("unknown", RemoteCapability::Read)
            .is_err());
        policy.set_degraded(true);
        assert!(policy
            .authorize_cached_result("nz", RemoteCapability::Write)
            .is_err());
        assert!(policy
            .authorize_cached_result("nz", RemoteCapability::Read)
            .is_ok());
        policy.set_degraded(false);
        policy.set_kill_switch(true);
        assert!(policy
            .authorize_cached_result("nz", RemoteCapability::Write)
            .is_err());
        policy.set_kill_switch(false);
        for _ in 0..3 {
            policy.record_failure();
        }
        assert!(policy
            .authorize_cached_result("nz", RemoteCapability::Write)
            .is_err());
        assert_eq!(policy.status().instances[0].requests_used, 0);
    }

    #[test]
    fn expired_confirmation_deadline_does_not_reserve_or_consume_allowance() {
        let policy = bounded_policy(1, 10);
        let deadline = Instant::now();
        assert!(policy
            .begin_request_before("nz", RemoteCapability::Write, deadline)
            .is_err());
        assert_eq!(policy.status().instances[0].requests_used, 0);
        assert!(policy
            .begin_request_before(
                "nz",
                RemoteCapability::Write,
                deadline + Duration::from_secs(30),
            )
            .is_ok());
    }

    #[test]
    fn admission_checks_current_time_while_holding_the_policy_lock() {
        let policy = bounded_policy(2, 10);
        let deadline = Instant::now();
        let clock = || {
            assert!(matches!(
                policy.state.try_lock(),
                Err(std::sync::TryLockError::WouldBlock)
            ));
            deadline
        };
        assert!(policy
            .begin_request_with_clock("nz", RemoteCapability::Write, Some(deadline), clock)
            .is_err());
        assert_eq!(policy.status().instances[0].requests_used, 0);
        let permit = policy
            .begin_request_with_clock("nz", RemoteCapability::Write, None, clock)
            .unwrap();
        permit.finish_at(1, deadline).unwrap();
    }

    #[test]
    fn completion_samples_current_time_only_after_acquiring_policy_lock() {
        let policy = bounded_policy(2, 10);
        let start = Instant::now();
        let permit = policy
            .begin_request_at("nz", RemoteCapability::Read, start)
            .unwrap();
        let result = permit.finish_with_clock(1, || {
            assert!(matches!(
                policy.state.try_lock(),
                Err(std::sync::TryLockError::WouldBlock)
            ));
            start + Duration::from_secs(1)
        });
        assert!(result.is_err());
        assert_eq!(policy.status().instances[0].bytes_used, 1);
        assert!(policy.begin_request("nz", RemoteCapability::Read).is_ok());
    }
}
