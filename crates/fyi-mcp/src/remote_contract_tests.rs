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

#[test]
fn read_catalogue_includes_bounded_authority_feed_contracts() {
    let enabled = policy(true, false, 4096);
    let catalogue = registered_tool_catalogue(&enabled);
    for name in [
        "remote_discover_authority_feeds",
        "remote_plan_authority_feed",
    ] {
        let tool = catalogue["tools"]
            .as_array()
            .unwrap()
            .iter()
            .find(|tool| tool["name"] == name)
            .expect("the complete read surface must include authority feeds");
        assert_eq!(tool["annotations"]["readOnlyHint"], true);
        assert_eq!(tool["annotations"]["destructiveHint"], false);
        assert_eq!(
            tool["inputSchema"]["properties"]["authority_slug"]["maxLength"],
            128
        );
        assert_eq!(tool["inputSchema"]["additionalProperties"], false);
    }
    let disabled = registered_tool_catalogue(&RemoteMcpPolicy::new(vec![]).unwrap());
    assert!(disabled["tools"]
        .as_array()
        .unwrap()
        .iter()
        .all(|tool| !tool["name"].as_str().unwrap().starts_with("remote_")));
}

#[tokio::test]
async fn denied_read_has_a_versioned_safe_error_before_source_io() {
    let db = DbPool::new_in_memory().await.unwrap();
    let disabled = policy(false, false, 4096);
    let response = handle_jsonrpc_request_with_policy(
        &db,
        JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(170)),
            method: "tools/call".into(),
            params: Some(json!({"name":"remote_version","arguments":{"instance_id":"synthetic"}})),
        },
        &disabled,
    )
    .await
    .unwrap();
    assert!(response.error.is_none());
    let result = response.result.unwrap();
    assert_eq!(result["isError"], true);
    let error = &result["structuredContent"]["error"];
    assert_eq!(error["schema_version"], 1);
    assert_eq!(error["kind"], "disabled");
    assert_eq!(error["operation"], "remote_version");
    assert_eq!(error["retryable"], false);
    assert!(error["status"].is_null());
    assert!(error["correlation_id"].is_null());
    assert_eq!(disabled.status().instances[0].requests_used, 0);
}

#[test]
fn configured_instances_do_not_advertise_ungranted_read_capability() {
    for (read, write) in [(false, false), (false, true)] {
        let catalogue = registered_tool_catalogue(&policy(read, write, 4096));
        let remote = catalogue["tools"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|tool| tool["name"].as_str())
            .filter(|name| name.starts_with("remote_"))
            .collect::<Vec<_>>();
        assert_eq!(remote.len(), if write { 3 } else { 0 });
        assert!(remote.into_iter().all(is_remote_write));
    }
}

#[tokio::test]
async fn unsafe_authority_slugs_are_rejected_without_admission() {
    let policy = policy(true, false, 4096);
    for slug in [
        "",
        "../secret",
        "body/authority",
        "https://private.example",
        "authority?token=secret",
        "authority#fragment",
        "é",
        "a%2Fb",
    ] {
        for operation in [
            "remote_discover_authority_feeds",
            "remote_plan_authority_feed",
        ] {
            let error = remote_read(
                &policy,
                "synthetic",
                operation,
                &json!({"authority_slug":slug}),
            )
            .await
            .unwrap_err();
            assert_eq!(serde_json::to_value(&error).unwrap()["kind"], "validation");
            assert!(error.correlation_id.is_none());
            assert!(!error.to_string().contains("secret"));
        }
    }
    assert_eq!(policy.status().instances[0].requests_used, 0);
    assert!(!policy.status().circuit_open);
}

#[tokio::test]
async fn feed_discovery_and_read_contract_are_offline_and_leave_database_empty() {
    let db = DbPool::new_in_memory().await.unwrap();
    db.run_migrations().await.unwrap();
    let policy = policy(true, false, 8192);
    let payload = remote_read(
        &policy,
        "synthetic",
        "remote_discover_authority_feeds",
        &json!({"authority_slug":"synthetic-authority"}),
    )
    .await
    .unwrap();
    assert_eq!(
        payload["feeds"],
        json!([{
            "url":"https://foi-repository.example/feed/body/synthetic-authority",
            "availability":"not_checked"
        }])
    );
    assert_eq!(payload["provenance"]["cache_status"], "not_applicable");
    assert_eq!(payload["provenance"]["response_bytes"], 0);
    assert!(payload["provenance"]["source_status"].is_null());
    assert!(payload["provenance"]["correlation_id"]
        .as_str()
        .unwrap()
        .starts_with("mcp-"));
    assert!(chrono::DateTime::parse_from_rfc3339(
        payload["provenance"]["retrieved_at"].as_str().unwrap()
    )
    .is_ok());
    let mut result = payload.clone();
    assert_eq!(
        arguments::validate_arguments(
            &remote_output_schema("remote_discover_authority_feeds"),
            &mut result
        ),
        Ok(())
    );
    assert_eq!(
        policy.status().instances[0].bytes_used,
        serialized_payload_bytes(&payload).unwrap()
    );
    let before = policy.status();
    let resource = read_mcp_resource(&db, "fyi://remote/read-contract")
        .await
        .unwrap();
    let contract: Value = serde_json::from_str(resource[0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(contract["disabled_by_default"], true);
    assert_eq!(contract["tools"].as_array().unwrap().len(), 7);
    assert_eq!(policy.status(), before);
    assert!(db.list_requests(10).await.unwrap().is_empty());
}

#[tokio::test]
async fn typed_source_failure_retains_status_but_never_private_diagnostics() {
    let policy = policy(true, false, 4096);
    let permit = policy
        .begin_request("synthetic", RemoteCapability::Read)
        .unwrap();
    let error = execute_remote_operation_typed(&policy, permit, RemoteOperation::Version, async {
        Err(
            anyhow::Error::new(fyi_core::sync::SourceHttpError::new(429))
                .context("synthetic-private-token=/private/path body=secret"),
        )
    })
    .await
    .unwrap_err();
    let response = tool_read_error(Some(json!(170)), error);
    let mut structured = response.result.as_ref().unwrap()["structuredContent"].clone();
    assert_eq!(
        arguments::validate_arguments(&remote_output_schema("remote_version"), &mut structured),
        Ok(())
    );
    assert_eq!(structured["error"]["status"], 429);
    assert_eq!(structured["error"]["retryable"], true);
    assert!(structured["error"]["correlation_id"]
        .as_str()
        .unwrap()
        .starts_with("mcp-"));
    assert!(!serde_json::to_string(&response).unwrap().contains("secret"));
    assert!(!serde_json::to_string(&response)
        .unwrap()
        .contains("/private/path"));
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
            assert!(remote_read(&policy, "synthetic", operation, &arguments)
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
    let result = execute_remote_operation(&policy, permit, RemoteOperation::Version, async {
        Ok(payload.clone())
    })
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
    let error = execute_remote_operation(&policy, permit, RemoteOperation::Version, async {
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
        let error = execute_remote_operation(&policy, permit, RemoteOperation::Version, async {
            Err(anyhow::anyhow!(
                "synthetic-secret upstream response /private/path"
            ))
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
    let result = execute_remote_operation(
        &policy,
        permit,
        RemoteOperation::Version,
        std::future::pending(),
    )
    .await;
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
    let result = execute_remote_operation(&policy, permit, RemoteOperation::Version, async {
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

#[tokio::test]
async fn attachment_validation_enforces_count_limit_and_string_items() {
    let directory = std::env::temp_dir().join(format!(
        "fyi-mcp-attach-bounds-{}-{}",
        std::process::id(),
        next_correlation_id()
    ));
    tokio::fs::create_dir_all(&directory).await.unwrap();
    let root = tokio::fs::canonicalize(&directory).await.unwrap();

    let too_many: Vec<Value> = (0..9).map(|i| json!(format!("file-{i}.txt"))).collect();
    assert_eq!(
        validated_attachment_paths(&json!({"attachments": too_many}))
            .await
            .unwrap_err(),
        "attachments exceed the 8 file limit"
    );
    assert_eq!(
        attachment_paths_under_root(&root, &[json!(123)])
            .await
            .unwrap_err(),
        "attachment paths must be strings"
    );
    assert_eq!(
        validated_attachment_paths(&json!({"attachments": "not-an-array"}))
            .await
            .unwrap_err(),
        "attachments must be an array"
    );

    tokio::fs::remove_dir_all(directory).await.unwrap();
}

#[tokio::test]
async fn write_argument_validation_fails_closed_before_network_io() {
    let policy = policy(true, true, 1024);
    let gate = RemoteWriteGate::default();

    let prepared_state = gate
        .prepare(
            &policy,
            "synthetic",
            "remote_update_state",
            &json!({"idempotency_key": "missing-expected", "request_id": 42, "state": "successful"}),
        )
        .unwrap();
    let state_commit = json!({
        "instance_id": "synthetic",
        "phase": "commit",
        "idempotency_key": "missing-expected",
        "confirmation_token": prepared_state["confirmation_token"],
    });
    assert_eq!(
        remote_commit_write(
            &policy,
            &gate,
            "synthetic",
            "remote_update_state",
            &state_commit
        )
        .await
        .unwrap_err(),
        "remote operation failed; see operator audit telemetry"
    );

    let prepared_corr = gate
        .prepare(
            &policy,
            "synthetic",
            "remote_add_correspondence",
            &json!({
                "idempotency_key": "bad-direction",
                "request_id": 42,
                "direction": "sideways",
                "body": "synthetic body",
                "sent_at": "2026-11-16T00:00:00Z"
            }),
        )
        .unwrap();
    let corr_commit = json!({
        "instance_id": "synthetic",
        "phase": "commit",
        "idempotency_key": "bad-direction",
        "confirmation_token": prepared_corr["confirmation_token"],
    });
    assert_eq!(
        remote_commit_write(
            &policy,
            &gate,
            "synthetic",
            "remote_add_correspondence",
            &corr_commit
        )
        .await
        .unwrap_err(),
        "remote operation failed; see operator audit telemetry"
    );
    let status = policy.status();
    assert_eq!(status.instances[0].bytes_used, 0);
    assert!(!status.circuit_open);
}
