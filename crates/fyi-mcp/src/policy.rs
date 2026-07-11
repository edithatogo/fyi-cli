//! Fail-closed policy and operator controls for remote MCP operations.
//!
//! This module deliberately has no telemetry dependency.  Callers can export
//! the bounded status and audit values to their preferred local system without
//! sending request content, credentials, or user data anywhere.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
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
}

#[derive(Debug, Clone)]
pub struct RemoteMcpPolicy {
    instances: BTreeMap<String, RemoteInstancePolicy>,
    state: Arc<Mutex<ControlState>>,
    circuit_failure_threshold: u32,
    circuit_cooldown: Duration,
}

impl RemoteMcpPolicy {
    pub fn new(instances: Vec<RemoteInstancePolicy>) -> Result<Self, PolicyError> {
        if instances.len() > MAX_INSTANCES {
            return Err(PolicyError::InvalidConfiguration(
                "too many remote instances".into(),
            ));
        }
        let mut map = BTreeMap::new();
        for instance in instances {
            validate_instance(&instance)?;
            if map.insert(instance.instance_id.clone(), instance).is_some() {
                return Err(PolicyError::InvalidConfiguration(
                    "duplicate remote instance id".into(),
                ));
            }
        }
        Ok(Self {
            instances: map,
            state: Arc::new(Mutex::new(ControlState {
                kill_switch: false,
                degraded: false,
                consecutive_failures: 0,
                circuit_open_until: None,
                usage: BTreeMap::new(),
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
        let state = self.state.lock().expect("policy mutex poisoned");
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
        if state
            .circuit_open_until
            .is_some_and(|until| until > Instant::now())
        {
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
        let (requests, bytes) = state.usage.get(instance_id).copied().unwrap_or_default();
        if requests >= instance.budget.max_requests || bytes >= instance.budget.max_bytes {
            return Err(PolicyError::BudgetExceeded(
                "remote instance budget exhausted".into(),
            ));
        }
        Ok(())
    }

    pub fn instance(&self, instance_id: &str) -> Option<RemoteInstancePolicy> {
        self.instances.get(instance_id).cloned()
    }

    pub fn record_request(&self, instance_id: &str, bytes: u64) -> Result<(), PolicyError> {
        let instance = self.instances.get(instance_id).ok_or_else(|| {
            PolicyError::Disabled("remote instance is not in the explicit allowlist".into())
        })?;
        let mut state = self.state.lock().expect("policy mutex poisoned");
        let usage = state.usage.entry(instance_id.to_string()).or_default();
        let next_bytes = usage
            .1
            .checked_add(bytes)
            .ok_or_else(|| PolicyError::BudgetExceeded("remote byte budget overflow".into()))?;
        if usage.0 >= instance.budget.max_requests || next_bytes > instance.budget.max_bytes {
            return Err(PolicyError::BudgetExceeded(
                "remote instance budget exhausted".into(),
            ));
        }
        usage.0 += 1;
        usage.1 = next_bytes;
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
        let mut state = self.state.lock().expect("policy mutex poisoned");
        state.consecutive_failures += 1;
        if state.consecutive_failures >= self.circuit_failure_threshold {
            state.circuit_open_until = Some(Instant::now() + self.circuit_cooldown);
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
}
