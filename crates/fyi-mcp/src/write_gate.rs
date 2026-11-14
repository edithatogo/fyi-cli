//! Bounded, process-local confirmation and replay protection for remote writes.
//!
//! Unused confirmations expire after five monotonic minutes. Once a confirmation
//! is consumed, its idempotency reservation is never evicted during this process:
//! an uncertain upstream outcome must not silently permit a duplicate write.

use crate::policy::{RemoteCapability, RemoteMcpPolicy, RemoteRequestPermit};
use fyi_core::security::generate_totp_secret;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const MAX_PREPARE_BYTES: usize = 64 * 1024;
const MAX_RESULT_BYTES: usize = 64 * 1024;
const MAX_TRACKED_WRITES: usize = 128;
const CONFIRMATION_TTL: Duration = Duration::from_secs(300);
const UNAVAILABLE_TOKEN: &str = "confirmation_token is missing, expired, or already used";

#[derive(Clone)]
pub struct PreparedWrite {
    pub instance_id: String,
    pub operation: String,
    pub arguments: Value,
    pub idempotency_key: String,
}

impl std::fmt::Debug for PreparedWrite {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PreparedWrite")
            .finish_non_exhaustive()
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct WriteKey {
    instance_id: String,
    operation: String,
    idempotency_key: String,
}

impl From<&PreparedWrite> for WriteKey {
    fn from(prepared: &PreparedWrite) -> Self {
        Self {
            instance_id: prepared.instance_id.clone(),
            operation: prepared.operation.clone(),
            idempotency_key: prepared.idempotency_key.clone(),
        }
    }
}

enum WriteState {
    Pending { token: String, created_at: Instant },
    InFlight,
    Completed(Value),
}

struct WriteEntry {
    prepared: PreparedWrite,
    state: WriteState,
}

#[derive(Clone, Default)]
pub struct RemoteWriteGate {
    entries: Arc<Mutex<HashMap<WriteKey, WriteEntry>>>,
}

impl std::fmt::Debug for RemoteWriteGate {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RemoteWriteGate")
            .finish_non_exhaustive()
    }
}

impl RemoteWriteGate {
    pub fn prepare(
        &self,
        policy: &RemoteMcpPolicy,
        instance_id: &str,
        operation: &str,
        arguments: &Value,
    ) -> Result<Value, String> {
        self.prepare_with_clock(policy, instance_id, operation, arguments, Instant::now)
    }

    #[cfg(test)]
    fn prepare_at(
        &self,
        policy: &RemoteMcpPolicy,
        instance_id: &str,
        operation: &str,
        arguments: &Value,
        now: Instant,
    ) -> Result<Value, String> {
        self.prepare_with_clock(policy, instance_id, operation, arguments, || now)
    }

    fn prepare_with_clock(
        &self,
        policy: &RemoteMcpPolicy,
        instance_id: &str,
        operation: &str,
        arguments: &Value,
        clock: impl FnOnce() -> Instant,
    ) -> Result<Value, String> {
        policy
            .authorize_cached_result(instance_id, RemoteCapability::Write)
            .map_err(|error| error.to_string())?;
        if !matches!(
            operation,
            "remote_create_request" | "remote_add_correspondence" | "remote_update_state"
        ) {
            return Err("unsupported remote write operation".into());
        }
        let arguments = validated_arguments(arguments, instance_id)?;
        let idempotency_key = arguments["idempotency_key"]
            .as_str()
            .ok_or_else(|| "idempotency_key is required".to_string())?
            .to_string();
        let prepared = PreparedWrite {
            instance_id: instance_id.to_string(),
            operation: operation.to_string(),
            arguments,
            idempotency_key,
        };
        let key = WriteKey::from(&prepared);
        let mut entries = self
            .entries
            .lock()
            .map_err(|_| "write confirmation state is unavailable".to_string())?;
        let now = clock();
        // Reclaim only unused confirmations. Never forget an attempted write.
        entries.retain(|_, entry| match &entry.state {
            WriteState::Pending { created_at, .. } => remaining(*created_at, now).is_some(),
            WriteState::InFlight | WriteState::Completed(_) => true,
        });
        if let Some(entry) = entries.get(&key) {
            if entry.prepared.arguments != prepared.arguments {
                return Err("idempotency_key is already bound to different write arguments".into());
            }
            return match &entry.state {
                WriteState::Pending { token, created_at } => {
                    policy
                        .authorize(instance_id, RemoteCapability::Write)
                        .map_err(|error| error.to_string())?;
                    Ok(prepared_response(
                        token,
                        &prepared.idempotency_key,
                        remaining(*created_at, now).unwrap_or_default(),
                    ))
                }
                WriteState::InFlight => {
                    Err("write outcome is pending or uncertain; do not retry with a new key".into())
                }
                WriteState::Completed(result) => {
                    // Recheck after waiting for the ledger. Cached data still
                    // respects current controls, but performs no remote dispatch.
                    policy
                        .authorize_cached_result(instance_id, RemoteCapability::Write)
                        .map_err(|error| error.to_string())?;
                    Ok(json!({
                        "status": "already_committed",
                        "idempotency_key": prepared.idempotency_key,
                        "result": result,
                    }))
                }
            };
        }
        policy
            .authorize(instance_id, RemoteCapability::Write)
            .map_err(|error| error.to_string())?;
        if entries.len() >= MAX_TRACKED_WRITES {
            return Err("write confirmation capacity reached; reconcile outstanding writes".into());
        }
        // Reuse the core's OS-CSPRNG-backed 160-bit base32 secret generator.
        // No TOTP code is computed; the complete random secret is the capability.
        let secret = generate_totp_secret()
            .map_err(|_| "secure confirmation token generation is unavailable".to_string())?;
        let token = secret.as_str().to_string();
        if entries.values().any(|entry| {
            matches!(&entry.state, WriteState::Pending { token: existing, .. } if same_token(existing, &token))
        }) {
            return Err("secure confirmation token generation failed".into());
        }
        let response = prepared_response(&token, &prepared.idempotency_key, CONFIRMATION_TTL);
        entries.insert(
            key,
            WriteEntry {
                prepared,
                state: WriteState::Pending {
                    token,
                    created_at: now,
                },
            },
        );
        Ok(response)
    }

    /// Atomically admit a dispatch and consume its exact prepared confirmation.
    ///
    /// Invalid confirmations spend no allowance. Policy admission runs while the
    /// ledger is locked, and rechecks the confirmation deadline after the policy
    /// lock is acquired. A capacity/control denial leaves the token unused.
    pub fn reserve_for(
        &self,
        policy: &RemoteMcpPolicy,
        token: &str,
        instance_id: &str,
        operation: &str,
        idempotency_key: &str,
    ) -> Result<(PreparedWrite, RemoteRequestPermit), String> {
        self.consume_pending(
            token,
            instance_id,
            operation,
            idempotency_key,
            Instant::now,
            |deadline| {
                policy
                    .begin_request_before(instance_id, RemoteCapability::Write, deadline)
                    .map_err(|error| error.to_string())
            },
        )
    }

    /// Consume a confirmation without dispatch admission for internal verification.
    /// Remote dispatch callers must use `reserve_for` instead.
    pub fn take_for(
        &self,
        token: &str,
        instance_id: &str,
        operation: &str,
        idempotency_key: &str,
    ) -> Result<PreparedWrite, String> {
        self.take_for_with_clock(token, instance_id, operation, idempotency_key, Instant::now)
    }

    #[cfg(test)]
    fn take_for_at(
        &self,
        token: &str,
        instance_id: &str,
        operation: &str,
        idempotency_key: &str,
        now: Instant,
    ) -> Result<PreparedWrite, String> {
        self.take_for_with_clock(token, instance_id, operation, idempotency_key, || now)
    }

    fn take_for_with_clock(
        &self,
        token: &str,
        instance_id: &str,
        operation: &str,
        idempotency_key: &str,
        clock: impl FnOnce() -> Instant,
    ) -> Result<PreparedWrite, String> {
        self.consume_pending(
            token,
            instance_id,
            operation,
            idempotency_key,
            clock,
            |_| Ok(()),
        )
        .map(|(prepared, ())| prepared)
    }

    fn consume_pending<T>(
        &self,
        token: &str,
        instance_id: &str,
        operation: &str,
        idempotency_key: &str,
        clock: impl FnOnce() -> Instant,
        admit: impl FnOnce(Instant) -> Result<T, String>,
    ) -> Result<(PreparedWrite, T), String> {
        if !(32..=80).contains(&token.len())
            || !token
                .bytes()
                .all(|byte| byte.is_ascii_uppercase() || (b'2'..=b'7').contains(&byte))
        {
            return Err("confirmation_token is invalid".into());
        }
        let mut entries = self
            .entries
            .lock()
            .map_err(|_| "write confirmation state is unavailable".to_string())?;
        let now = clock();
        let key = entries
            .iter()
            .find_map(|(key, entry)| match &entry.state {
                WriteState::Pending {
                    token: candidate, ..
                } if same_token(candidate, token) => Some(key.clone()),
                _ => None,
            })
            .ok_or_else(|| UNAVAILABLE_TOKEN.to_string())?;
        if key.instance_id != instance_id
            || key.operation != operation
            || key.idempotency_key != idempotency_key
        {
            return Err("confirmation token does not match this write operation".into());
        }
        let entry = entries
            .get_mut(&key)
            .ok_or_else(|| UNAVAILABLE_TOKEN.to_string())?;
        let deadline = match entry.state {
            WriteState::Pending { created_at, .. } if remaining(created_at, now).is_some() => {
                created_at.checked_add(CONFIRMATION_TTL)
            }
            _ => None,
        };
        let Some(deadline) = deadline else {
            entries.remove(&key);
            return Err(UNAVAILABLE_TOKEN.into());
        };
        // Preserve Pending on every admission failure. Reversing these two
        // operations either spends allowance on invalid tokens or loses a valid
        // confirmation when another dispatch has reserved the instance slot.
        let permit = admit(deadline)?;
        entry.state = WriteState::InFlight;
        Ok((entry.prepared.clone(), permit))
    }

    /// Retain a bounded result only for the previously consumed immutable write.
    pub fn remember_result(&self, prepared: &PreparedWrite, result: Value) -> Result<(), String> {
        let mut entries = self
            .entries
            .lock()
            .map_err(|_| "write confirmation state is unavailable".to_string())?;
        let entry = entries
            .get_mut(&WriteKey::from(prepared))
            .ok_or_else(|| "write confirmation was not consumed".to_string())?;
        if !matches!(entry.state, WriteState::InFlight)
            || entry.prepared.arguments != prepared.arguments
        {
            return Err("write result does not match a consumed confirmation".into());
        }
        if !fits_json_budget(&result, MAX_RESULT_BYTES) {
            // Keep the reservation even though the complete result cannot be cached.
            return Err(
                "write completed but its replay result exceeds the 64 KiB safety limit".into(),
            );
        }
        entry.state = WriteState::Completed(result);
        Ok(())
    }
}

fn validated_arguments(arguments: &Value, instance_id: &str) -> Result<Value, String> {
    let values = arguments
        .as_object()
        .ok_or_else(|| "write arguments must be an object".to_string())?;
    if values
        .get("phase")
        .is_some_and(|phase| phase.as_str() != Some("prepare"))
        || values.contains_key("confirmation_token")
    {
        return Err("prepare requires phase prepare and no confirmation_token".into());
    }
    if values
        .get("instance_id")
        .is_some_and(|value| value.as_str() != Some(instance_id))
    {
        return Err("write arguments do not match the configured instance".into());
    }
    let key = values
        .get("idempotency_key")
        .and_then(Value::as_str)
        .ok_or_else(|| "idempotency_key is required".to_string())?;
    if key.is_empty()
        || key.len() > 128
        || !key.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        })
    {
        return Err("idempotency_key is invalid".into());
    }
    if !fits_json_budget(arguments, MAX_PREPARE_BYTES) {
        return Err("write arguments exceed the 64 KiB safety limit".into());
    }
    if let Some(attachments) = values.get("attachments") {
        let attachments = attachments
            .as_array()
            .ok_or_else(|| "attachments must be an array".to_string())?;
        if attachments.len() > 8 {
            return Err("attachments exceed the 8 file limit".into());
        }
        for attachment in attachments {
            let path = attachment
                .as_str()
                .ok_or_else(|| "attachment paths must be strings".to_string())?;
            if path.is_empty()
                || path.len() > 1024
                || path.contains(['\0', ':'])
                || path
                    .split(['/', '\\'])
                    .any(|part| part.is_empty() || matches!(part, "." | ".."))
            {
                return Err("attachment path is invalid".into());
            }
        }
    }
    let mut normalized = values.clone();
    normalized.remove("phase");
    Ok(Value::Object(normalized))
}

fn remaining(created_at: Instant, now: Instant) -> Option<Duration> {
    let elapsed = now.checked_duration_since(created_at)?;
    (elapsed < CONFIRMATION_TTL).then(|| CONFIRMATION_TTL - elapsed)
}

fn prepared_response(token: &str, key: &str, remaining: Duration) -> Value {
    json!({
        "status": "prepared",
        "confirmation_token": token,
        "idempotency_key": key,
        "expires_in_seconds": remaining.as_secs().max(1),
        "requires_commit": true,
    })
}

fn same_token(left: &str, right: &str) -> bool {
    left.len() == right.len()
        && left
            .bytes()
            .zip(right.bytes())
            .fold(0, |difference, (a, b)| difference | (a ^ b))
            == 0
}

fn fits_json_budget(value: &Value, limit: usize) -> bool {
    struct ByteBudget(usize);
    impl Write for ByteBudget {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0 = self
                .0
                .checked_sub(bytes.len())
                .ok_or_else(|| io::Error::other("JSON byte budget exceeded"))?;
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    serde_json::to_writer(ByteBudget(limit), value).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::{RemoteBudget, RemoteInstancePolicy};

    const OPERATION: &str = "remote_create_request";

    fn policy() -> RemoteMcpPolicy {
        RemoteMcpPolicy::new(
            ["nz", "au"]
                .into_iter()
                .map(|instance_id| RemoteInstancePolicy {
                    instance_id: instance_id.into(),
                    base_url: "https://foi-repository.example".into(),
                    read_enabled: true,
                    write_enabled: true,
                    budget: RemoteBudget::default(),
                })
                .collect(),
        )
        .unwrap()
    }

    fn args(key: &str) -> Value {
        json!({"idempotency_key": key, "body": "Synthetic body"})
    }

    fn prepare(gate: &RemoteWriteGate, now: Instant, key: &str) -> String {
        gate.prepare_at(&policy(), "nz", OPERATION, &args(key), now)
            .unwrap()["confirmation_token"]
            .as_str()
            .unwrap()
            .to_string()
    }

    #[test]
    fn prepare_requires_capability_operation_and_replay_key() {
        let gate = RemoteWriteGate::default();
        assert!(gate
            .prepare(
                &RemoteMcpPolicy::new(vec![]).unwrap(),
                "nz",
                OPERATION,
                &args("k1")
            )
            .is_err());
        assert!(gate
            .prepare(&policy(), "nz", "remote_get_request", &args("k1"))
            .is_err());
        assert!(gate
            .prepare(&policy(), "nz", OPERATION, &json!({}))
            .is_err());
        assert!(gate.entries.lock().unwrap().is_empty());
    }

    #[test]
    fn duplicate_prepare_reuses_confirmation_without_extending_expiry() {
        let gate = RemoteWriteGate::default();
        let now = Instant::now();
        let token = prepare(&gate, now, "k1");
        let repeated = gate
            .prepare_at(
                &policy(),
                "nz",
                OPERATION,
                &args("k1"),
                now + Duration::from_secs(100),
            )
            .unwrap();
        assert_eq!(repeated["confirmation_token"], token);
        assert_eq!(repeated["expires_in_seconds"], 200);
        assert_eq!(gate.entries.lock().unwrap().len(), 1);
    }

    #[test]
    fn explicit_prepare_phase_has_the_same_payload_identity() {
        let gate = RemoteWriteGate::default();
        let now = Instant::now();
        let token = prepare(&gate, now, "k1");
        let mut explicit = args("k1");
        explicit["phase"] = json!("prepare");
        let repeated = gate
            .prepare_at(&policy(), "nz", OPERATION, &explicit, now)
            .unwrap();
        assert_eq!(repeated["confirmation_token"], token);
    }

    #[test]
    fn confirmation_is_opaque_and_bound_to_all_scope_fields() {
        let gate = RemoteWriteGate::default();
        let now = Instant::now();
        let token = prepare(&gate, now, "k1");
        assert_eq!(token.len(), 32);
        assert!(token
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || (b'2'..=b'7').contains(&byte)));
        for (instance, operation, key) in [
            ("au", OPERATION, "k1"),
            ("nz", "remote_update_state", "k1"),
            ("nz", OPERATION, "other-key"),
        ] {
            assert!(gate
                .take_for_at(&token, instance, operation, key, now)
                .is_err());
        }
        let prepared = gate
            .take_for_at(&token, "nz", OPERATION, "k1", now)
            .unwrap();
        assert_eq!(prepared.arguments, args("k1"));
        assert!(gate
            .take_for_at(&token, "nz", OPERATION, "k1", now)
            .is_err());
    }

    #[test]
    fn expiry_and_clock_skew_fail_closed_without_sleep() {
        for age in [Duration::ZERO, Duration::from_secs(299)] {
            let gate = RemoteWriteGate::default();
            let now = Instant::now();
            let token = prepare(&gate, now, "k1");
            assert!(gate
                .take_for_at(&token, "nz", OPERATION, "k1", now + age)
                .is_ok());
        }
        for age in [CONFIRMATION_TTL, CONFIRMATION_TTL + Duration::from_secs(1)] {
            let gate = RemoteWriteGate::default();
            let now = Instant::now();
            let token = prepare(&gate, now, "k1");
            assert!(gate
                .take_for_at(&token, "nz", OPERATION, "k1", now + age)
                .is_err());
            assert!(gate.entries.lock().unwrap().is_empty());
        }
        let gate = RemoteWriteGate::default();
        let now = Instant::now();
        let token = prepare(&gate, now + Duration::from_secs(1), "k1");
        assert!(gate
            .take_for_at(&token, "nz", OPERATION, "k1", now)
            .is_err());
    }

    #[test]
    fn expired_unused_confirmation_can_be_prepared_again() {
        let gate = RemoteWriteGate::default();
        let now = Instant::now();
        let first = prepare(&gate, now, "k1");
        let second = prepare(&gate, now + CONFIRMATION_TTL, "k1");
        assert_ne!(first, second);
        assert!(gate
            .take_for_at(&first, "nz", OPERATION, "k1", now + CONFIRMATION_TTL)
            .is_err());
        assert!(gate
            .take_for_at(&second, "nz", OPERATION, "k1", now + CONFIRMATION_TTL)
            .is_ok());
    }

    #[test]
    fn in_flight_reservation_survives_expiry_and_blocks_duplicate_write() {
        let gate = RemoteWriteGate::default();
        let now = Instant::now();
        let token = prepare(&gate, now, "k1");
        gate.take_for_at(&token, "nz", OPERATION, "k1", now)
            .unwrap();
        let error = gate
            .prepare_at(
                &policy(),
                "nz",
                OPERATION,
                &args("k1"),
                now + CONFIRMATION_TTL,
            )
            .unwrap_err();
        assert!(error.contains("uncertain"));
        assert_eq!(gate.entries.lock().unwrap().len(), 1);
    }

    #[test]
    fn completed_result_is_bound_to_instance_operation_and_payload() {
        let gate = RemoteWriteGate::default();
        let now = Instant::now();
        let token = prepare(&gate, now, "k1");
        let prepared = gate
            .take_for_at(&token, "nz", OPERATION, "k1", now)
            .unwrap();
        gate.remember_result(&prepared, json!({"id": 7})).unwrap();
        let result = gate
            .prepare_at(
                &policy(),
                "nz",
                OPERATION,
                &args("k1"),
                now + CONFIRMATION_TTL,
            )
            .unwrap();
        assert_eq!(
            result,
            json!({"status": "already_committed", "idempotency_key": "k1", "result": {"id": 7}})
        );
        for (instance, operation) in [("au", OPERATION), ("nz", "remote_update_state")] {
            let separate = gate
                .prepare_at(&policy(), instance, operation, &args("k1"), now)
                .unwrap();
            assert_eq!(separate["status"], "prepared");
            assert!(separate.get("result").is_none());
        }
        let mut changed = args("k1");
        changed["body"] = json!("Synthetic changed body");
        assert!(gate
            .prepare_at(&policy(), "nz", OPERATION, &changed, now)
            .is_err());
        assert!(gate.remember_result(&prepared, json!({"id": 8})).is_err());
    }

    #[test]
    fn pending_key_rejects_changed_payload() {
        let gate = RemoteWriteGate::default();
        let now = Instant::now();
        prepare(&gate, now, "k1");
        let mut changed = args("k1");
        changed["body"] = json!("Synthetic changed body");
        assert!(gate
            .prepare_at(&policy(), "nz", OPERATION, &changed, now)
            .is_err());
    }

    #[test]
    fn recording_a_fabricated_or_changed_write_cannot_prime_replay() {
        let gate = RemoteWriteGate::default();
        let now = Instant::now();
        let token = prepare(&gate, now, "k1");
        let mut forged = PreparedWrite {
            instance_id: "nz".into(),
            operation: OPERATION.into(),
            arguments: args("k1"),
            idempotency_key: "k1".into(),
        };
        assert!(gate.remember_result(&forged, json!({"id": 7})).is_err());
        let prepared = gate
            .take_for_at(&token, "nz", OPERATION, "k1", now)
            .unwrap();
        forged.arguments["body"] = json!("Synthetic changed body");
        assert!(gate.remember_result(&forged, json!({"id": 7})).is_err());
        forged.idempotency_key = "missing".into();
        assert!(gate.remember_result(&forged, json!({"id": 7})).is_err());
        gate.remember_result(&prepared, json!({"id": 7})).unwrap();
    }

    #[test]
    fn capacity_is_bounded_and_only_unused_expired_entries_are_reclaimed() {
        let gate = RemoteWriteGate::default();
        let now = Instant::now();
        for index in 0..MAX_TRACKED_WRITES {
            prepare(&gate, now, &format!("key-{index}"));
        }
        assert!(gate
            .prepare_at(&policy(), "nz", OPERATION, &args("overflow"), now)
            .is_err());
        prepare(&gate, now + CONFIRMATION_TTL, "after-expiry");
        assert_eq!(gate.entries.lock().unwrap().len(), 1);
        let other = RemoteWriteGate::default();
        for index in 0..MAX_TRACKED_WRITES {
            let key = format!("key-{index}");
            let token = prepare(&other, now, &key);
            let prepared = other
                .take_for_at(&token, "nz", OPERATION, &key, now)
                .unwrap();
            if index % 2 == 0 {
                other
                    .remember_result(&prepared, json!({"id": index}))
                    .unwrap();
            }
        }
        assert!(other
            .prepare_at(
                &policy(),
                "nz",
                OPERATION,
                &args("overflow"),
                now + CONFIRMATION_TTL
            )
            .is_err());
        assert_eq!(other.entries.lock().unwrap().len(), MAX_TRACKED_WRITES);
    }

    #[test]
    fn oversized_result_keeps_the_consumed_reservation() {
        let gate = RemoteWriteGate::default();
        let now = Instant::now();
        let token = prepare(&gate, now, "k1");
        let prepared = gate
            .take_for_at(&token, "nz", OPERATION, "k1", now)
            .unwrap();
        assert!(gate
            .remember_result(&prepared, json!({"data": "x".repeat(MAX_RESULT_BYTES)}))
            .is_err());
        assert!(gate
            .prepare_at(
                &policy(),
                "nz",
                OPERATION,
                &args("k1"),
                now + CONFIRMATION_TTL
            )
            .is_err());
    }

    #[test]
    fn prepare_validates_shape_phase_scope_and_byte_bounds() {
        let gate = RemoteWriteGate::default();
        let cases = [
            json!(null),
            json!([]),
            json!("synthetic-input"),
            json!({"idempotency_key": ""}),
            json!({"idempotency_key": "x".repeat(129)}),
            json!({"idempotency_key": "synthetic@example.test"}),
            json!({"idempotency_key": true}),
            json!({"idempotency_key": "k1", "phase": "commit"}),
            json!({"idempotency_key": "k1", "phase": false}),
            json!({"idempotency_key": "k1", "confirmation_token": "synthetic-token"}),
            json!({"idempotency_key": "k1", "instance_id": "au"}),
            json!({"idempotency_key": "k1", "body": "x".repeat(MAX_PREPARE_BYTES)}),
        ];
        for input in cases {
            assert!(gate.prepare(&policy(), "nz", OPERATION, &input).is_err());
        }
        assert!(gate.entries.lock().unwrap().is_empty());
        assert!(gate
            .prepare(&policy(), "nz", OPERATION, &args(&"x".repeat(128)))
            .is_ok());
    }

    #[test]
    fn attachment_paths_are_relative_on_every_supported_platform() {
        let gate = RemoteWriteGate::default();
        for attachments in [
            json!("file.pdf"),
            json!([false]),
            json!([""]),
            json!(["../file.pdf"]),
            json!(["folder/../file.pdf"]),
            json!(["folder\\..\\file.pdf"]),
            json!(["/file.pdf"]),
            json!(["C:\\file.pdf"]),
            json!(["\\\\host\\file.pdf"]),
            json!(["file.pdf:stream"]),
            json!(["nul\u{0}.pdf"]),
            json!(["x".repeat(1025)]),
            json!(vec!["file.pdf"; 9]),
        ] {
            let mut input = args("k1");
            input["attachments"] = attachments;
            assert!(gate.prepare(&policy(), "nz", OPERATION, &input).is_err());
        }
        let mut valid = args("k1");
        valid["attachments"] = json!(["folder/file.pdf", "folder\\other.pdf"]);
        assert!(gate.prepare(&policy(), "nz", OPERATION, &valid).is_ok());
    }

    #[test]
    fn invalid_and_unknown_tokens_do_not_disclose_input() {
        let gate = RemoteWriteGate::default();
        for token in ["", "secret@example.test", &"A".repeat(81), &"A".repeat(32)] {
            let error = gate.take_for(token, "nz", OPERATION, "k1").unwrap_err();
            assert!(!error.contains("secret@example.test"));
            assert!(!error.contains(&"A".repeat(32)));
        }
    }

    #[test]
    fn debug_output_redacts_tokens_payloads_and_replay_keys() {
        let gate = RemoteWriteGate::default();
        let now = Instant::now();
        let token = prepare(&gate, now, "synthetic-private-key");
        let before = format!("{gate:?}");
        let prepared = gate
            .take_for_at(&token, "nz", OPERATION, "synthetic-private-key", now)
            .unwrap();
        let after = format!("{prepared:?}");
        for output in [before, after] {
            assert!(!output.contains(&token));
            assert!(!output.contains("synthetic-private-key"));
            assert!(!output.contains("Synthetic body"));
        }
    }

    #[test]
    fn concurrent_prepare_and_consume_are_atomic() {
        let gate = RemoteWriteGate::default();
        let handles: Vec<_> = (0..8)
            .map(|_| {
                let shared = gate.clone();
                std::thread::spawn(move || {
                    shared
                        .prepare(&policy(), "nz", OPERATION, &args("k1"))
                        .unwrap()["confirmation_token"]
                        .as_str()
                        .unwrap()
                        .to_string()
                })
            })
            .collect();
        let tokens: Vec<_> = handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect();
        assert!(tokens.iter().all(|token| token == &tokens[0]));
        let admission_policy = single_attempt_policy();
        let handles: Vec<_> = tokens
            .into_iter()
            .map(|token| {
                let shared = gate.clone();
                let policy = admission_policy.clone();
                std::thread::spawn(move || {
                    shared
                        .reserve_for(&policy, &token, "nz", OPERATION, "k1")
                        .is_ok()
                })
            })
            .collect();
        assert_eq!(
            handles
                .into_iter()
                .map(|handle| handle.join().unwrap())
                .filter(|consumed| *consumed)
                .count(),
            1
        );
        assert_eq!(admission_policy.status().instances[0].requests_used, 1);
    }

    #[test]
    fn json_budget_counts_encoded_utf8_and_escapes_exactly() {
        for value in [json!("é"), json!("\n"), json!({"a": [1, 2, 3]})] {
            let bytes = serde_json::to_vec(&value).unwrap().len();
            assert!(fits_json_budget(&value, bytes));
            assert!(!fits_json_budget(&value, bytes - 1));
        }
        assert!(same_token("ABCD", "ABCD"));
        assert!(!same_token("ABCD", "ABCE"));
        assert!(!same_token("ABCD", "ABC"));
    }
    fn single_attempt_policy() -> RemoteMcpPolicy {
        RemoteMcpPolicy::new(vec![RemoteInstancePolicy {
            instance_id: "nz".into(),
            base_url: "https://foi-repository.example".into(),
            read_enabled: true,
            write_enabled: true,
            budget: RemoteBudget {
                max_requests: 1,
                ..RemoteBudget::default()
            },
        }])
        .unwrap()
    }

    #[tokio::test]
    async fn admission_invalid_confirmation_does_not_spend_dispatch_allowance() {
        let gate = RemoteWriteGate::default();
        let policy = single_attempt_policy();
        let error = crate::remote_commit_write(
            &policy,
            &gate,
            "nz",
            OPERATION,
            &json!({"instance_id":"nz", "phase":"commit", "idempotency_key":"k1",
                    "confirmation_token":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"}),
        )
        .await
        .unwrap_err();
        assert!(error.contains("confirmation"));
        assert_eq!(policy.status().instances[0].requests_used, 0);
    }

    #[test]
    fn admission_completed_result_replays_after_final_dispatch_allowance() {
        let gate = RemoteWriteGate::default();
        let policy = single_attempt_policy();
        let confirmation = gate.prepare(&policy, "nz", OPERATION, &args("k1")).unwrap();
        let prepared = gate
            .take_for(
                confirmation["confirmation_token"].as_str().unwrap(),
                "nz",
                OPERATION,
                "k1",
            )
            .unwrap();
        let permit = policy.begin_request("nz", RemoteCapability::Write).unwrap();
        permit.finish(10).unwrap();
        gate.remember_result(&prepared, json!({"id":7})).unwrap();
        assert!(policy.authorize("nz", RemoteCapability::Write).is_err());
        let replay = gate.prepare(&policy, "nz", OPERATION, &args("k1")).unwrap();
        assert_eq!(
            replay,
            json!({"status":"already_committed", "idempotency_key":"k1", "result":{"id":7}})
        );
        assert_eq!(policy.status().instances[0].requests_used, 1);
    }
    #[test]
    fn invalid_or_mismatched_confirmation_does_not_reserve_policy() {
        let gate = RemoteWriteGate::default();
        let policy = single_attempt_policy();
        let confirmation = gate.prepare(&policy, "nz", OPERATION, &args("k1")).unwrap();
        let token = confirmation["confirmation_token"].as_str().unwrap();
        for (candidate, instance, operation, key) in [
            ("bad", "nz", OPERATION, "k1"),
            ("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA", "nz", OPERATION, "k1"),
            (token, "au", OPERATION, "k1"),
            (token, "nz", "remote_update_state", "k1"),
            (token, "nz", OPERATION, "other-key"),
        ] {
            assert!(gate
                .reserve_for(&policy, candidate, instance, operation, key)
                .is_err());
            assert_eq!(policy.status().instances[0].requests_used, 0);
        }
        let (_, permit) = gate
            .reserve_for(&policy, token, "nz", OPERATION, "k1")
            .unwrap();
        assert_eq!(policy.status().instances[0].requests_used, 1);
        drop(permit);
        assert!(gate
            .reserve_for(&policy, token, "nz", OPERATION, "k1")
            .is_err());
        assert_eq!(policy.status().instances[0].requests_used, 1);
    }

    #[test]
    fn concurrency_denial_preserves_pending_confirmation() {
        let gate = RemoteWriteGate::default();
        let policy = policy();
        let first = gate
            .prepare(&policy, "nz", OPERATION, &args("first"))
            .unwrap();
        let second = gate
            .prepare(&policy, "nz", OPERATION, &args("second"))
            .unwrap();
        let (_, permit) = gate
            .reserve_for(
                &policy,
                first["confirmation_token"].as_str().unwrap(),
                "nz",
                OPERATION,
                "first",
            )
            .unwrap();
        let error = gate
            .reserve_for(
                &policy,
                second["confirmation_token"].as_str().unwrap(),
                "nz",
                OPERATION,
                "second",
            )
            .unwrap_err();
        assert!(error.contains("concurrency"));
        assert_eq!(
            policy
                .status()
                .instances
                .iter()
                .find(|instance| instance.instance_id == "nz")
                .unwrap()
                .requests_used,
            1
        );
        drop(permit);
        let (_, second_permit) = gate
            .reserve_for(
                &policy,
                second["confirmation_token"].as_str().unwrap(),
                "nz",
                OPERATION,
                "second",
            )
            .unwrap();
        second_permit.finish(0).unwrap();
        assert_eq!(
            policy
                .status()
                .instances
                .iter()
                .find(|instance| instance.instance_id == "nz")
                .unwrap()
                .requests_used,
            2
        );
    }

    #[test]
    fn exhausted_budget_preserves_token_and_blocks_new_or_pending_prepare() {
        let gate = RemoteWriteGate::default();
        let policy = single_attempt_policy();
        let first = gate
            .prepare(&policy, "nz", OPERATION, &args("first"))
            .unwrap();
        let second = gate
            .prepare(&policy, "nz", OPERATION, &args("second"))
            .unwrap();
        let (prepared, permit) = gate
            .reserve_for(
                &policy,
                first["confirmation_token"].as_str().unwrap(),
                "nz",
                OPERATION,
                "first",
            )
            .unwrap();
        permit.finish(0).unwrap();
        gate.remember_result(&prepared, json!({"id":7})).unwrap();
        assert!(gate
            .reserve_for(
                &policy,
                second["confirmation_token"].as_str().unwrap(),
                "nz",
                OPERATION,
                "second"
            )
            .is_err());
        assert!(gate
            .prepare(&policy, "nz", OPERATION, &args("second"))
            .is_err());
        assert!(gate
            .prepare(&policy, "nz", OPERATION, &args("third"))
            .is_err());
        assert!(gate
            .prepare(&policy, "nz", OPERATION, &args("first"))
            .is_ok());
        // This test-only path verifies that policy denial did not consume the token.
        assert!(gate
            .take_for(
                second["confirmation_token"].as_str().unwrap(),
                "nz",
                OPERATION,
                "second"
            )
            .is_ok());
        assert_eq!(policy.status().instances[0].requests_used, 1);
    }

    #[test]
    fn expired_confirmation_never_invokes_dispatch_admission() {
        let gate = RemoteWriteGate::default();
        let policy = single_attempt_policy();
        let now = Instant::now();
        let confirmation = gate
            .prepare_at(&policy, "nz", OPERATION, &args("k1"), now)
            .unwrap();
        let result: Result<(PreparedWrite, ()), String> = gate.consume_pending(
            confirmation["confirmation_token"].as_str().unwrap(),
            "nz",
            OPERATION,
            "k1",
            || now + CONFIRMATION_TTL,
            |_| panic!("expired confirmations must not enter policy admission"),
        );
        assert!(result.is_err());
        assert_eq!(policy.status().instances[0].requests_used, 0);
    }

    #[test]
    fn cache_replay_obeys_current_controls_after_budget_exhaustion() {
        let gate = RemoteWriteGate::default();
        let policy = single_attempt_policy();
        let confirmation = gate.prepare(&policy, "nz", OPERATION, &args("k1")).unwrap();
        let (prepared, permit) = gate
            .reserve_for(
                &policy,
                confirmation["confirmation_token"].as_str().unwrap(),
                "nz",
                OPERATION,
                "k1",
            )
            .unwrap();
        permit.finish(0).unwrap();
        gate.remember_result(&prepared, json!({"id":7})).unwrap();
        policy.set_kill_switch(true);
        assert!(gate.prepare(&policy, "nz", OPERATION, &args("k1")).is_err());
        policy.set_kill_switch(false);
        policy.set_degraded(true);
        assert!(gate.prepare(&policy, "nz", OPERATION, &args("k1")).is_err());
        policy.set_degraded(false);
        for _ in 0..3 {
            policy.record_failure();
        }
        assert!(gate.prepare(&policy, "nz", OPERATION, &args("k1")).is_err());
        policy.record_success();
        assert_eq!(
            gate.prepare(&policy, "nz", OPERATION, &args("k1")).unwrap()["status"],
            "already_committed"
        );
        let mut instance = policy.instance("nz").unwrap();
        instance.write_enabled = false;
        let read_only = RemoteMcpPolicy::new(vec![instance]).unwrap();
        assert!(gate
            .prepare(&read_only, "nz", OPERATION, &args("k1"))
            .is_err());
        assert_eq!(policy.status().instances[0].requests_used, 1);
    }

    #[test]
    fn cached_result_is_available_after_byte_allowance_is_exhausted() {
        let gate = RemoteWriteGate::default();
        let mut instance = single_attempt_policy().instance("nz").unwrap();
        instance.budget.max_requests = 2;
        instance.budget.max_bytes = 10;
        let policy = RemoteMcpPolicy::new(vec![instance]).unwrap();
        let confirmation = gate.prepare(&policy, "nz", OPERATION, &args("k1")).unwrap();
        let (prepared, permit) = gate
            .reserve_for(
                &policy,
                confirmation["confirmation_token"].as_str().unwrap(),
                "nz",
                OPERATION,
                "k1",
            )
            .unwrap();
        permit.finish(10).unwrap();
        gate.remember_result(&prepared, json!({"id":7})).unwrap();
        assert!(policy.authorize("nz", RemoteCapability::Write).is_err());
        assert_eq!(
            gate.prepare(&policy, "nz", OPERATION, &args("k1")).unwrap()["status"],
            "already_committed"
        );
        assert_eq!(policy.status().instances[0].bytes_used, 10);
    }
}
