//! Offline contract tests for the local remote-operation status resource.

use super::*;
use policy::{RemoteBudget, RemoteCapability, RemoteInstancePolicy};

const STATUS_URI: &str = "fyi://remote/status";

#[derive(Clone, Default)]
struct Capture(Arc<std::sync::Mutex<Vec<u8>>>);

impl std::io::Write for Capture {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn policy(bytes: u64) -> RemoteMcpPolicy {
    RemoteMcpPolicy::new(vec![RemoteInstancePolicy {
        instance_id: "synthetic".into(),
        base_url: "https://synthetic-private-host.example".into(),
        read_enabled: true,
        write_enabled: true,
        budget: RemoteBudget {
            max_requests: 20,
            max_bytes: bytes,
            max_duration_seconds: 1,
        },
    }])
    .unwrap()
}

async fn call(policy: &RemoteMcpPolicy, method: &str, params: Option<Value>) -> JsonRpcResponse {
    let db = DbPool::new_in_memory().await.unwrap();
    db.run_migrations().await.unwrap();
    handle_jsonrpc_request_with_policy(
        &db,
        JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!("synthetic-private-caller-id")),
            method: method.into(),
            params,
        },
        policy,
    )
    .await
    .unwrap()
}

async fn status(policy: &RemoteMcpPolicy) -> Value {
    let response = call(policy, "resources/read", Some(json!({"uri":STATUS_URI}))).await;
    assert!(
        response.error.is_none(),
        "status resource is unavailable: {response:?}"
    );
    let result = response.result.unwrap();
    assert_eq!(result["contents"][0]["uri"], STATUS_URI);
    assert_eq!(result["contents"][0]["mimeType"], "application/json");
    serde_json::from_str(result["contents"][0]["text"].as_str().unwrap()).unwrap()
}

fn row(status: &Value, operation: &str) -> Value {
    status["metrics"]["operations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["operation"] == operation)
        .unwrap()
        .clone()
}

#[tokio::test]
async fn remote_status_is_discoverable_even_with_no_remote_capabilities() {
    let policy = RemoteMcpPolicy::new(Vec::new()).unwrap();
    let listed = call(&policy, "resources/list", None).await.result.unwrap();
    assert!(listed["resources"]
        .as_array()
        .unwrap()
        .iter()
        .any(|resource| {
            resource["uri"] == STATUS_URI && resource["mimeType"] == "application/json"
        }));
    let snapshot = status(&policy).await;
    assert_eq!(snapshot["schema_version"], 1);
    assert_eq!(snapshot["policy"]["remote_enabled"], false);
    assert_eq!(snapshot["metrics"]["schema_version"], 1);
    assert_eq!(snapshot["metrics"]["scope"], "process");
    assert_eq!(
        snapshot["metrics"]["measurement"],
        "mcp_admitted_operations"
    );
    assert_eq!(
        snapshot["metrics"]["operations"].as_array().unwrap().len(),
        8
    );
}

#[tokio::test]
async fn status_is_private_read_only_and_available_under_operator_controls() {
    let policy = policy(1024);
    policy.set_kill_switch(true);
    policy.set_degraded(true);
    for _ in 0..3 {
        policy.record_failure();
    }
    let first = status(&policy).await;
    assert_eq!(first, status(&policy.clone()).await);
    assert_eq!(first["policy"]["kill_switch"], true);
    assert_eq!(first["policy"]["degraded"], true);
    assert_eq!(first["policy"]["circuit_open"], true);
    assert_eq!(first["policy"]["instances"][0]["requests_used"], 0);
    let text = first.to_string();
    assert!(!text.contains("synthetic-private"));
    assert!(!text.contains("base_url"));
    assert!(!text.contains("correlation_id"));
}

#[tokio::test]
async fn successful_dispatch_metrics_count_serialized_utf8_once() {
    let policy = policy(1024);
    let payload = json!({"private":"Māori 🦀 synthetic-private-body"});
    let permit = policy
        .begin_request("synthetic", RemoteCapability::Read)
        .unwrap();
    let result = execute_remote_operation(&policy, permit, RemoteOperation::Version, async {
        Ok(payload.clone())
    })
    .await
    .unwrap();
    assert_eq!(result, payload);
    let snapshot = status(&policy).await;
    let metric = row(&snapshot, "remote_version");
    assert_eq!(metric["started"], 1);
    assert_eq!(metric["in_flight"], 0);
    assert_eq!(metric["outcomes"]["success"], 1);
    assert_eq!(
        metric["serialized_result_bytes"],
        serde_json::to_vec(&payload).unwrap().len()
    );
    assert_eq!(metric["duration_bucket_counts"][6], 1);
    assert!(!snapshot.to_string().contains("synthetic-private-body"));
    assert_eq!(snapshot, status(&policy.clone()).await);
}

#[tokio::test]
async fn failed_and_rejected_results_have_fixed_outcomes_without_private_errors() {
    for (bytes, outcome) in [(1024, "failed"), (1, "result_rejected")] {
        let policy = policy(bytes);
        let permit = policy
            .begin_request("synthetic", RemoteCapability::Read)
            .unwrap();
        let result = execute_remote_operation(&policy, permit, RemoteOperation::Version, async {
            if bytes == 1 {
                Ok(json!({"private":"synthetic-private-result"}))
            } else {
                Err("synthetic-private-error /private/path?credential=secret".into())
            }
        })
        .await;
        assert!(result.is_err());
        let snapshot = status(&policy).await;
        let metric = row(&snapshot, "remote_version");
        assert_eq!(metric["outcomes"][outcome], 1);
        assert_eq!(metric["in_flight"], 0);
        assert_eq!(metric["outcomes"]["success"], 0);
        assert_eq!(
            metric["serialized_result_bytes"],
            policy.status().instances[0].bytes_used
        );
        assert!(!snapshot.to_string().contains("synthetic-private"));
    }
}

#[tokio::test]
async fn cancelled_dispatch_releases_activity_and_records_one_cancellation() {
    let policy = policy(1024);
    let permit = policy
        .begin_request("synthetic", RemoteCapability::Read)
        .unwrap();
    {
        let operation = execute_remote_operation(
            &policy,
            permit,
            RemoteOperation::Version,
            std::future::pending(),
        );
        tokio::pin!(operation);
        tokio::select! {
            _ = &mut operation => panic!("pending operation completed"),
            _ = tokio::task::yield_now() => {}
        }
        let during = row(&status(&policy).await, "remote_version");
        assert_eq!(during["started"], 1);
        assert_eq!(during["in_flight"], 1);
    }
    let metric = row(&status(&policy).await, "remote_version");
    assert_eq!(metric["in_flight"], 0);
    assert_eq!(metric["outcomes"]["cancelled"], 1);
    assert_eq!(metric["outcomes"]["success"], 0);
    assert_eq!(metric["duration_bucket_counts"][6], 1);
    assert_eq!(policy.status().instances[0].requests_used, 1);
    assert!(policy
        .begin_request("synthetic", RemoteCapability::Read)
        .is_ok());
}

#[tokio::test]
async fn dispatch_timeout_is_distinct_from_failure_and_cancellation() {
    let policy = policy(1024);
    let permit = policy
        .begin_request("synthetic", RemoteCapability::Read)
        .unwrap();
    assert!(execute_remote_operation(
        &policy,
        permit,
        RemoteOperation::Version,
        std::future::pending()
    )
    .await
    .is_err());
    let metric = row(&status(&policy).await, "remote_version");
    assert_eq!(metric["outcomes"]["timed_out"], 1);
    assert_eq!(metric["outcomes"]["failed"], 0);
    assert_eq!(metric["outcomes"]["cancelled"], 0);
    assert_eq!(metric["in_flight"], 0);
}

#[tokio::test]
async fn async_source_future_inherits_server_correlation_without_private_values() {
    // Tracing callsite interest is process-wide. Other parallel tests exercise
    // these same callsites without a subscriber; isolate the capture and use
    // the process-wide subscriber that production startup also installs.
    if std::env::var_os("FYI_MCP_TRACE_CAPTURE_CHILD").as_deref() != Some(std::ffi::OsStr::new("1"))
    {
        let output = tokio::time::timeout(
            std::time::Duration::from_secs(30),
            tokio::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "observability_tests::async_source_future_inherits_server_correlation_without_private_values",
                    "--nocapture",
                ])
                .env("FYI_MCP_TRACE_CAPTURE_CHILD", "1")
                .kill_on_drop(true)
                .output(),
        )
        .await
        .expect("isolated tracing contract timed out")
        .unwrap();
        assert!(
            output.status.success(),
            "isolated tracing contract failed:\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    let capture = Capture::default();
    let writer = capture.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .without_time()
        .with_writer(move || writer.clone())
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();
    let policy = policy(4096);
    for operation in [RemoteOperation::Version, RemoteOperation::CreateRequest] {
        let permit = policy
            .begin_request("synthetic", RemoteCapability::Read)
            .unwrap();
        execute_remote_operation(&policy, permit, operation, async {
            tokio::task::yield_now().await;
            tracing::info!(target: "synthetic_source", "synthetic source event");
            Ok(json!({"private":"synthetic-private-result"}))
        })
        .await
        .unwrap();
    }
    let logs = String::from_utf8(capture.0.lock().unwrap().clone()).unwrap();
    let lines: Vec<_> = logs.lines().collect();
    assert_eq!(lines.len(), 6, "{logs}");
    let mut correlations = Vec::new();
    for (events, operation) in lines
        .as_chunks::<3>()
        .0
        .iter()
        .zip(["remote_version", "remote_create_request"])
    {
        let correlation = events[0]
            .split("correlation_id=")
            .nth(1)
            .unwrap()
            .split_whitespace()
            .next()
            .unwrap();
        assert!(correlation.trim_matches('"').starts_with("mcp-"));
        for event in events {
            assert!(event.contains("mcp_remote_dispatch"), "{event}");
            assert!(event.contains("schema_version=1"), "{event}");
            assert!(event.contains(correlation), "{event}");
            assert!(event.contains(operation), "{event}");
        }
        assert!(events[0].contains("event=\"started\""));
        assert!(events[1].contains("synthetic source event"));
        assert!(events[2].contains("event=\"completed\""));
        assert!(events[2].contains("outcome=\"success\""));
        correlations.push(correlation);
    }
    assert_ne!(correlations[0], correlations[1]);
    assert!(!logs.contains("synthetic-private"));
    assert!(!logs.contains("instance_id"));
}

#[tokio::test]
async fn invalid_or_disallowed_calls_cannot_allocate_metric_dimensions() {
    let policy = policy(4096);
    let original = status(&policy).await;
    for index in 0..100 {
        for arguments in [
            json!({"instance_id":format!("synthetic-private-{index}")}),
            json!({"instance_id":"synthetic", "query":format!("private-{index}")}),
        ] {
            let response = call(
                &policy,
                "tools/call",
                Some(json!({
                    "name":"remote_version", "arguments":arguments
                })),
            )
            .await;
            assert!(response.error.is_some() || response.result.unwrap()["isError"] == true);
        }
    }
    assert_eq!(original, status(&policy).await);
    assert!(RemoteOperation::from_tool("private-unknown-operation").is_none());
    assert!(remote_read(&policy, "synthetic", "unknown", &json!({}))
        .await
        .is_err());
    assert!(remote_commit_write(
        &policy,
        &RemoteWriteGate::default(),
        "synthetic",
        "unknown",
        &json!({})
    )
    .await
    .is_err());
    assert_eq!(original, status(&policy).await);
}
