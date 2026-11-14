//! Synthetic remote-operation boundary tests; never contact a remote endpoint.

use super::*;
use policy::{RemoteBudget, RemoteCapability, RemoteInstancePolicy};

fn policy(read: bool, write: bool, bytes: u64) -> RemoteMcpPolicy {
    RemoteMcpPolicy::new(vec![RemoteInstancePolicy {
        instance_id: "synthetic".into(),
        base_url: "https://foi-repository.example".into(),
        read_enabled: read,
        write_enabled: write,
        budget: RemoteBudget {
            max_requests: 5,
            max_bytes: bytes,
            max_duration_seconds: 1,
        },
    }])
    .unwrap()
}

#[tokio::test]
async fn invalid_read_arguments_never_spend_allowance_or_open_the_circuit() {
    let policy = policy(true, true, 1024);
    for _ in 0..3 {
        for (operation, arguments) in [
            ("remote_search_requests", json!({"query":"   "})),
            ("remote_search_requests", json!({"query":"x".repeat(201)})),
            ("remote_get_request", json!({"request_id":0})),
            (
                "remote_get_request",
                json!({"request_id":"synthetic-private"}),
            ),
            ("remote_list_authorities", json!({"filter":"x".repeat(101)})),
        ] {
            assert!(remote_read(
                &policy,
                "synthetic",
                operation,
                &arguments,
                "synthetic-correlation"
            )
            .await
            .is_err());
        }
    }
    let status = policy.status();
    assert_eq!(status.instances[0].requests_used, 0);
    assert_eq!(status.instances[0].bytes_used, 0);
    assert!(!status.circuit_open);
    assert!(policy
        .authorize("synthetic", RemoteCapability::Write)
        .is_ok());
}

#[tokio::test]
async fn successful_remote_result_is_counted_before_publication() {
    let policy = policy(true, false, 64);
    policy.record_failure();
    let payload = json!({"instance_id":"synthetic","version":"1"});
    let permit = policy
        .begin_request("synthetic", RemoteCapability::Read)
        .unwrap();
    let result = execute_remote_operation(&policy, permit, async { Ok(payload.clone()) })
        .await
        .unwrap();
    assert_eq!(result, payload);
    let status = policy.status();
    assert_eq!(status.instances[0].requests_used, 1);
    assert_eq!(
        status.instances[0].bytes_used,
        serde_json::to_vec(&payload).unwrap().len() as u64
    );
    assert!(!status.circuit_open);
}

#[tokio::test]
async fn oversized_remote_result_is_never_returned_or_refunded() {
    let policy = policy(true, false, 8);
    let permit = policy
        .begin_request("synthetic", RemoteCapability::Read)
        .unwrap();
    let error = execute_remote_operation(&policy, permit, async {
        Ok(json!({"private":"synthetic-disclosure-marker"}))
    })
    .await
    .unwrap_err();
    assert!(!error.contains("synthetic-disclosure-marker"));
    assert_eq!(policy.status().instances[0].requests_used, 1);
    assert!(policy.status().instances[0].bytes_used > 8);
    assert!(policy
        .begin_request("synthetic", RemoteCapability::Read)
        .is_err());
}

#[tokio::test]
async fn failed_remote_future_has_a_private_error_and_releases_concurrency() {
    let policy = policy(true, false, 1024);
    for _ in 0..3 {
        let permit = policy
            .begin_request("synthetic", RemoteCapability::Read)
            .unwrap();
        let error = execute_remote_operation(&policy, permit, async {
            Err("synthetic-secret upstream response /private/path".into())
        })
        .await
        .unwrap_err();
        assert_eq!(
            error,
            "remote operation failed; see operator audit telemetry"
        );
    }
    assert!(policy.status().circuit_open);
    assert_eq!(policy.status().instances[0].requests_used, 3);
}

#[tokio::test]
async fn timeout_drops_the_remote_future_and_preserves_consumed_request() {
    let policy = policy(true, false, 1024);
    let permit = policy
        .begin_request("synthetic", RemoteCapability::Read)
        .unwrap();
    let result = execute_remote_operation(&policy, permit, std::future::pending()).await;
    assert_eq!(
        result.unwrap_err(),
        "remote operation timed out; see operator audit telemetry"
    );
    assert_eq!(policy.status().instances[0].requests_used, 1);
    assert!(policy
        .begin_request("synthetic", RemoteCapability::Read)
        .is_ok());
}

#[tokio::test]
async fn policy_change_before_result_publication_fails_closed() {
    let policy = policy(true, true, 1024);
    let permit = policy
        .begin_request("synthetic", RemoteCapability::Write)
        .unwrap();
    let result = execute_remote_operation(&policy, permit, async {
        policy.set_degraded(true);
        Ok(json!({"ok":true}))
    })
    .await;
    assert!(result.is_err());
    assert_eq!(policy.status().instances[0].requests_used, 1);
    assert!(policy
        .begin_request("synthetic", RemoteCapability::Read)
        .is_ok());
}

#[tokio::test]
async fn disabled_writes_and_invalid_confirmation_never_start_remote_io() {
    let disabled = policy(true, false, 1024);
    let gate = RemoteWriteGate::default();
    let args = json!({"instance_id":"synthetic", "phase":"commit", "idempotency_key":"fixture", "confirmation_token":"SYNTHETIC-INVALID"});
    assert!(remote_commit_write(
        &disabled,
        &gate,
        "synthetic",
        "remote_create_request",
        &args
    )
    .await
    .is_err());
    assert_eq!(disabled.status().instances[0].requests_used, 0);
    let enabled = policy(true, true, 1024);
    let error = remote_commit_write(&enabled, &gate, "synthetic", "remote_create_request", &args)
        .await
        .unwrap_err();
    assert_eq!(error, "confirmation_token is invalid");
    assert!(!error.contains("SYNTHETIC-INVALID"));
    assert_eq!(enabled.status().instances[0].requests_used, 0);
    assert_eq!(enabled.status().instances[0].bytes_used, 0);
}

#[tokio::test]
async fn missing_confirmation_and_wrong_scope_are_rejected_before_client_construction() {
    let policy = policy(true, true, 1024);
    let gate = RemoteWriteGate::default();
    let prepared = gate
        .prepare(
            &policy,
            "synthetic",
            "remote_create_request",
            &json!({"idempotency_key":"scope-fixture"}),
        )
        .unwrap();
    for arguments in [
        json!({}),
        json!({"confirmation_token":prepared["confirmation_token"]}),
    ] {
        assert!(remote_commit_write(
            &policy,
            &gate,
            "synthetic",
            "remote_create_request",
            &arguments
        )
        .await
        .is_err());
    }
    assert_eq!(policy.status().instances[0].requests_used, 0);
    let wrong = json!({"confirmation_token":prepared["confirmation_token"], "idempotency_key":"scope-fixture"});
    assert!(
        remote_commit_write(&policy, &gate, "synthetic", "remote_update_state", &wrong)
            .await
            .is_err()
    );
    assert!(gate
        .take_for(
            prepared["confirmation_token"].as_str().unwrap(),
            "synthetic",
            "remote_create_request",
            "scope-fixture"
        )
        .is_ok());
}

#[tokio::test]
async fn changed_commit_payload_is_rejected_without_consuming_confirmation() {
    let policy = policy(true, true, 1024);
    let gate = RemoteWriteGate::default();
    let prepared = gate
        .prepare(
            &policy,
            "synthetic",
            "remote_create_request",
            &json!({"idempotency_key":"immutable", "body":"approved synthetic payload"}),
        )
        .unwrap();
    let arguments = json!({"instance_id":"synthetic", "phase":"commit", "idempotency_key":"immutable", "confirmation_token":prepared["confirmation_token"], "body":"changed synthetic payload"});
    assert!(remote_commit_write(
        &policy,
        &gate,
        "synthetic",
        "remote_create_request",
        &arguments
    )
    .await
    .is_err());
    assert_eq!(policy.status().instances[0].requests_used, 0);
    assert!(gate
        .take_for(
            prepared["confirmation_token"].as_str().unwrap(),
            "synthetic",
            "remote_create_request",
            "immutable"
        )
        .is_ok());
}

#[test]
fn correlation_ids_are_server_generated_and_json_budget_counts_utf8_bytes() {
    let first = next_correlation_id();
    let second = next_correlation_id();
    assert_ne!(first, second);
    assert!(first.strip_prefix("mcp-").unwrap().parse::<u64>().is_ok());
    let payload = json!({"unicode":"Māori \"\\\n", "items":[true, 17, null]});
    assert_eq!(
        serialized_payload_bytes(&payload).unwrap(),
        serde_json::to_vec(&payload).unwrap().len() as u64
    );
}

#[tokio::test]
async fn attachment_path_checks_reject_escape_and_in_root_symlinks() {
    let directory = std::env::temp_dir().join(format!(
        "fyi-mcp-attachments-{}-{}",
        std::process::id(),
        next_correlation_id()
    ));
    tokio::fs::create_dir_all(directory.join("nested"))
        .await
        .unwrap();
    tokio::fs::write(
        directory.join("nested/synthetic.txt"),
        b"synthetic fixture only",
    )
    .await
    .unwrap();
    let root = tokio::fs::canonicalize(&directory).await.unwrap();
    let paths = attachment_paths_under_root(&root, &[json!("nested/synthetic.txt")])
        .await
        .unwrap();
    assert_eq!(paths, vec![root.join("nested/synthetic.txt")]);
    for path in [
        "../private-marker",
        "/private-marker",
        "nested/./synthetic.txt",
        "C:\\private-marker",
        "nested\\..\\private-marker",
    ] {
        let error = attachment_paths_under_root(&root, &[json!(path)])
            .await
            .unwrap_err();
        assert!(!error.contains("private-marker"));
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(root.join("nested/synthetic.txt"), root.join("link.txt"))
            .unwrap();
        assert_eq!(
            attachment_paths_under_root(&root, &[json!("link.txt")])
                .await
                .unwrap_err(),
            "attachment symlinks are not allowed"
        );
        std::os::unix::fs::symlink(root.join("nested"), root.join("linkdir")).unwrap();
        assert_eq!(
            attachment_paths_under_root(&root, &[json!("linkdir/synthetic.txt")])
                .await
                .unwrap_err(),
            "attachment symlinks are not allowed"
        );
    }
    tokio::fs::remove_dir_all(directory).await.unwrap();
}
