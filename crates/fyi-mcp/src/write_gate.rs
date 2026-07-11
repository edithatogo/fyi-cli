//! Two-step confirmation and replay protection for remote writes.

use crate::policy::{RemoteCapability, RemoteMcpPolicy};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

const MAX_PREPARE_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone)]
pub struct PreparedWrite {
    pub instance_id: String,
    pub operation: String,
    pub arguments: Value,
    pub idempotency_key: String,
}

#[derive(Debug, Clone)]
pub struct RemoteWriteGate {
    pending: std::sync::Arc<Mutex<HashMap<String, PreparedWrite>>>,
    completed: std::sync::Arc<Mutex<HashMap<String, Value>>>,
    next_token: std::sync::Arc<AtomicU64>,
}

impl Default for RemoteWriteGate {
    fn default() -> Self {
        Self {
            pending: std::sync::Arc::new(Mutex::new(HashMap::new())),
            completed: std::sync::Arc::new(Mutex::new(HashMap::new())),
            next_token: std::sync::Arc::new(AtomicU64::new(1)),
        }
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
        policy
            .authorize(instance_id, RemoteCapability::Write)
            .map_err(|error| error.to_string())?;
        if !matches!(
            operation,
            "remote_create_request" | "remote_add_correspondence" | "remote_update_state"
        ) {
            return Err("unsupported remote write operation".into());
        }
        let idempotency_key = arguments
            .get("idempotency_key")
            .and_then(Value::as_str)
            .ok_or_else(|| "idempotency_key is required".to_string())?;
        if idempotency_key.is_empty()
            || idempotency_key.len() > 128
            || !idempotency_key
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        {
            return Err("idempotency_key is invalid".into());
        }
        let encoded =
            serde_json::to_vec(arguments).map_err(|_| "write arguments are invalid".to_string())?;
        if encoded.len() > MAX_PREPARE_BYTES {
            return Err("write arguments exceed the 64 KiB safety limit".into());
        }
        if let Some(attachments) = arguments.get("attachments") {
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
                    || path.contains('\0')
                    || path.contains("..")
                {
                    return Err("attachment path is invalid".into());
                }
            }
        }
        if let Some(result) = self
            .completed
            .lock()
            .expect("write gate mutex poisoned")
            .get(idempotency_key)
        {
            return Ok(
                json!({"status": "already_committed", "idempotency_key": idempotency_key, "result": result}),
            );
        }
        let token = format!(
            "confirm-{}",
            self.next_token.fetch_add(1, Ordering::Relaxed)
        );
        self.pending
            .lock()
            .expect("write gate mutex poisoned")
            .insert(
                token.clone(),
                PreparedWrite {
                    instance_id: instance_id.to_string(),
                    operation: operation.to_string(),
                    arguments: arguments.clone(),
                    idempotency_key: idempotency_key.to_string(),
                },
            );
        Ok(json!({
            "status": "prepared",
            "confirmation_token": token,
            "idempotency_key": idempotency_key,
            "expires_in_seconds": 300,
            "requires_commit": true
        }))
    }

    pub fn take(&self, token: &str) -> Result<PreparedWrite, String> {
        if token.is_empty() || token.len() > 80 {
            return Err("confirmation_token is invalid".into());
        }
        self.pending
            .lock()
            .expect("write gate mutex poisoned")
            .remove(token)
            .ok_or_else(|| "confirmation_token is missing, expired, or already used".into())
    }

    pub fn remember_result(&self, idempotency_key: String, result: Value) {
        self.completed
            .lock()
            .expect("write gate mutex poisoned")
            .insert(idempotency_key, result);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::{RemoteBudget, RemoteInstancePolicy};

    fn policy() -> RemoteMcpPolicy {
        RemoteMcpPolicy::new(vec![RemoteInstancePolicy {
            instance_id: "nz".into(),
            base_url: "https://foi-repository.example".into(),
            read_enabled: true,
            write_enabled: true,
            budget: RemoteBudget::default(),
        }])
        .unwrap()
    }

    #[test]
    fn prepare_requires_explicit_confirmation_and_replay_key() {
        let gate = RemoteWriteGate::default();
        let error = gate
            .prepare(&policy(), "nz", "remote_create_request", &json!({}))
            .unwrap_err();
        assert!(error.contains("idempotency_key"));
        let prepared = gate
            .prepare(
                &policy(),
                "nz",
                "remote_create_request",
                &json!({"idempotency_key": "k1"}),
            )
            .unwrap();
        let token = prepared["confirmation_token"].as_str().unwrap();
        assert!(gate.take(token).is_ok());
        assert!(gate.take(token).is_err());
    }

    #[test]
    fn completed_idempotency_key_is_replayed_without_new_token() {
        let gate = RemoteWriteGate::default();
        let args = json!({"idempotency_key": "k1"});
        gate.remember_result("k1".into(), json!({"id": 7}));
        let result = gate
            .prepare(&policy(), "nz", "remote_create_request", &args)
            .unwrap();
        assert_eq!(result["status"], "already_committed");
        assert!(result.get("confirmation_token").is_none());
    }
}
