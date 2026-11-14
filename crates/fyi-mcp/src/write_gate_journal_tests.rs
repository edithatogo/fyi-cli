//! Confirmation and durable journal integration; synthetic futures only.

use super::*;
use crate::policy::{RemoteBudget, RemoteInstancePolicy};
use std::path::PathBuf;

struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!(
            "fyi-mcp-gate-journal-{}-{}",
            std::process::id(),
            crate::next_correlation_id()
        )))
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn policy() -> RemoteMcpPolicy {
    RemoteMcpPolicy::new(vec![RemoteInstancePolicy {
        instance_id: "synthetic".into(),
        base_url: "https://example.invalid".into(),
        read_enabled: true,
        write_enabled: true,
        budget: RemoteBudget {
            max_requests: 1,
            max_bytes: 1024,
            max_duration_seconds: 30,
        },
    }])
    .unwrap()
}

fn arguments() -> Value {
    json!({"idempotency_key":"synthetic-private-key", "body":"synthetic-private-body Café 東京"})
}

#[tokio::test]
async fn durable_success_preserves_cached_replay_after_budget_exhaustion_but_not_restart() {
    let directory = Directory::new();
    let journal = WriteJournal::open(&directory.0).await.unwrap();
    let gate = RemoteWriteGate::with_journal(journal.clone());
    let policy = policy();
    let arguments = arguments();
    let response = gate
        .prepare_durable(&policy, "synthetic", "remote_create_request", &arguments)
        .await
        .unwrap();
    let token = response["confirmation_token"].as_str().unwrap();
    let (prepared, permit) = gate
        .reserve_durable(
            &policy,
            token,
            "synthetic",
            "remote_create_request",
            "synthetic-private-key",
        )
        .await
        .unwrap();
    permit.finish(1).unwrap();
    let result = json!({"body":"synthetic-private-result"});
    assert_eq!(
        gate.finish_durable(&prepared, Ok(result.clone()))
            .await
            .unwrap(),
        result
    );
    let cached = gate
        .prepare_durable(&policy, "synthetic", "remote_create_request", &arguments)
        .await
        .unwrap();
    assert_eq!(cached["status"], "already_committed");
    assert_eq!(cached["result"], result);
    assert_eq!(policy.status().instances[0].requests_used, 1);
    assert!(policy
        .begin_request("synthetic", RemoteCapability::Write)
        .is_err());
    journal.close().await;
    let bytes = std::fs::read(directory.0.join("remote-write-v1.sqlite3")).unwrap();
    for marker in ["synthetic-private", token] {
        assert!(!bytes
            .windows(marker.len())
            .any(|part| part == marker.as_bytes()));
    }
    let restarted_journal = WriteJournal::open(&directory.0).await.unwrap();
    let restarted = RemoteWriteGate::with_journal(restarted_journal.clone());
    assert!(restarted
        .prepare_durable(
            &self::policy(),
            "synthetic",
            "remote_create_request",
            &arguments
        )
        .await
        .is_err());
    assert!(restarted.entries.lock().unwrap().is_empty());
    restarted_journal.close().await;
}

#[tokio::test]
async fn invalid_confirmations_are_audited_without_spending_network_allowance() {
    let directory = Directory::new();
    let journal = WriteJournal::open(&directory.0).await.unwrap();
    let gate = RemoteWriteGate::with_journal(journal.clone());
    let policy = policy();
    let response = gate
        .prepare_durable(&policy, "synthetic", "remote_create_request", &arguments())
        .await
        .unwrap();
    assert!(gate
        .reserve_durable(
            &policy,
            "synthetic-private-invalid-token",
            "synthetic",
            "remote_create_request",
            "synthetic-private-key"
        )
        .await
        .is_err());
    assert_eq!(policy.status().instances[0].requests_used, 0);
    let (prepared, permit) = gate
        .reserve_durable(
            &policy,
            response["confirmation_token"].as_str().unwrap(),
            "synthetic",
            "remote_create_request",
            "synthetic-private-key",
        )
        .await
        .unwrap();
    drop(permit);
    assert_eq!(
        gate.finish_durable(&prepared, Err("fixed safe failure".into()))
            .await
            .unwrap_err(),
        "fixed safe failure"
    );
    assert!(gate
        .prepare_durable(&policy, "synthetic", "remote_create_request", &arguments())
        .await
        .is_err());
    journal.close().await;
}

#[tokio::test]
async fn audit_storage_failure_prevents_preparation_and_dispatch_and_result_publication() {
    let directory = Directory::new();
    let journal = WriteJournal::open(&directory.0).await.unwrap();
    let gate = RemoteWriteGate::with_journal(journal.clone());
    let policy = policy();
    let response = gate
        .prepare_durable(&policy, "synthetic", "remote_create_request", &arguments())
        .await
        .unwrap();
    journal.close().await;
    let error = gate
        .reserve_durable(
            &policy,
            response["confirmation_token"].as_str().unwrap(),
            "synthetic",
            "remote_create_request",
            "synthetic-private-key",
        )
        .await
        .unwrap_err();
    assert!(error.contains("journal is unavailable"));
    assert!(gate
        .prepare_durable(&policy, "synthetic", "remote_create_request", &arguments())
        .await
        .is_err());
    assert!(policy.status().instances[0].requests_used == 1);

    let fresh = RemoteWriteGate::with_journal(journal.clone());
    assert!(fresh
        .prepare_durable(
            &self::policy(),
            "synthetic",
            "remote_create_request",
            &arguments()
        )
        .await
        .is_err());
    assert!(fresh.entries.lock().unwrap().is_empty());
    assert!(fresh
        .prepare_durable(
            &self::policy(),
            "synthetic",
            "remote_create_request",
            &json!({"idempotency_key":"bad/key"})
        )
        .await
        .is_err());
    let fabricated = PreparedWrite {
        instance_id: "synthetic".into(),
        operation: "remote_create_request".into(),
        arguments: arguments(),
        idempotency_key: "synthetic-private-key".into(),
    };
    assert!(fresh
        .finish_durable(&fabricated, Ok(json!({"result":"private"})))
        .await
        .is_err());
}

#[tokio::test]
async fn cancellation_after_reservation_keeps_a_restart_tombstone_and_releases_permit() {
    let directory = Directory::new();
    let journal = WriteJournal::open(&directory.0).await.unwrap();
    let gate = RemoteWriteGate::with_journal(journal.clone());
    let policy = policy();
    let response = gate
        .prepare_durable(&policy, "synthetic", "remote_create_request", &arguments())
        .await
        .unwrap();
    let token = response["confirmation_token"].as_str().unwrap().to_string();
    let dispatch_gate = gate.clone();
    let dispatch_policy = policy.clone();
    let (ready, reserved) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(async move {
        let (_prepared, _permit) = dispatch_gate
            .reserve_durable(
                &dispatch_policy,
                &token,
                "synthetic",
                "remote_create_request",
                "synthetic-private-key",
            )
            .await
            .unwrap();
        ready.send(()).unwrap();
        std::future::pending::<()>().await;
    });
    reserved.await.unwrap();
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    assert_eq!(policy.status().instances[0].requests_used, 1);
    journal.close().await;
    let restarted_journal = WriteJournal::open(&directory.0).await.unwrap();
    let restarted = RemoteWriteGate::with_journal(restarted_journal.clone());
    assert!(restarted
        .prepare_durable(
            &self::policy(),
            "synthetic",
            "remote_create_request",
            &arguments()
        )
        .await
        .is_err());
    restarted_journal.close().await;
}
