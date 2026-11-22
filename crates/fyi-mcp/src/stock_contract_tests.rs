//! Pinned stock authority fixture through real MCP handlers, without sockets.

use super::*;
use crate::policy::{RemoteBudget, RemoteInstancePolicy};
use crate::write_journal::WriteJournal;
use std::collections::{BTreeSet, VecDeque};
use std::sync::{Arc, Mutex};

const KEY: &str = "<injected-at-runtime>";

async fn call(
    db: &DbPool,
    policy: &RemoteMcpPolicy,
    gate: &RemoteWriteGate,
    name: &str,
    arguments: Value,
) -> Value {
    let response = handle_jsonrpc_request_with_policy_and_gate(
        db,
        JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "tools/call".into(),
            params: Some(json!({"name":name, "arguments":arguments})),
        },
        policy,
        gate,
    )
    .await
    .unwrap();
    assert!(response.error.is_none(), "unexpected JSON-RPC error");
    response.result.unwrap()
}

fn instance() -> RemoteInstancePolicy {
    RemoteInstancePolicy {
        instance_id: "synthetic".into(),
        base_url: "https://source.invalid".into(),
        read_enabled: true,
        write_enabled: true,
        budget: RemoteBudget {
            max_requests: 20,
            max_bytes: 1024 * 1024,
            max_duration_seconds: 30,
        },
    }
}

#[test]
fn stock_profile_is_explicit_and_rejects_unknown_instances() {
    let legacy = RemoteMcpPolicy::new(vec![instance()]).unwrap();
    assert!(!legacy.is_stock_alaveteli("synthetic"));
    let unknown = BTreeSet::from(["missing".to_string()]);
    assert!(legacy.clone().with_stock_alaveteli(unknown).is_err());
    let stock = legacy
        .with_stock_alaveteli(BTreeSet::from(["synthetic".to_string()]))
        .unwrap();
    assert!(stock.is_stock_alaveteli("synthetic"));
}

#[tokio::test]
async fn stock_fixture_exercises_five_mcp_cases_and_write_guards_without_network() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/remote_mcp/upstream-stock-v1.json"
    ))
    .unwrap();
    let cases = fixture["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 5);
    for case in cases {
        let id = case["id"].as_str().unwrap();
        let directory = std::env::temp_dir().join(format!(
            "fyi-stock-{}-{}",
            std::process::id(),
            next_correlation_id()
        ));
        std::fs::create_dir(&directory).unwrap();
        std::fs::write(directory.join("report.txt"), "Synthetic attachment").unwrap();
        let exchanges = Arc::new(Mutex::new(VecDeque::from(
            case["exchanges"].as_array().unwrap().clone(),
        )));
        let queued = exchanges.clone();
        let identity = fyi_core::agent_runtime::ClientIdentity::default_identity(None).unwrap();
        let client =
            SyncClient::with_guardrails("https://source.invalid", &identity, Default::default())
                .unwrap()
                .with_sdk_fixture_transport(move |mut request| {
                    let expected = queued
                        .lock()
                        .unwrap()
                        .pop_front()
                        .expect("unexpected source request");
                    async move {
                        assert_eq!(request.method().as_str(), expected["method"]);
                        assert!(request.headers().get("x-fyi-bot-token").is_none());
                        let url = request.url().clone();
                        let path = match url.query() {
                            Some(query) => format!("{}?{query}", url.path()),
                            None => url.path().to_string(),
                        };
                        assert_eq!(path, expected["path"].as_str().unwrap());
                        if let Some(parts) = expected.get("request_parts") {
                            let content_type = request
                                .headers()
                                .get("content-type")
                                .unwrap()
                                .to_str()
                                .unwrap()
                                .to_string();
                            let body = request.body_mut().take().expect("form body");
                            let bytes =
                                axum::body::to_bytes(axum::body::Body::new(body), 1024 * 1024)
                                    .await?;
                            let text = std::str::from_utf8(&bytes)?;
                            if content_type.starts_with("multipart/form-data;") {
                                let boundary = content_type.split("boundary=").nth(1).unwrap();
                                let segments: Vec<_> = text
                                    .split(&format!("--{boundary}"))
                                    .filter(|segment| {
                                        segment.contains("Content-Disposition:")
                                            || segment.contains("content-disposition:")
                                    })
                                    .collect();
                                assert_eq!(segments.len(), parts.as_object().unwrap().len());
                                for (name, value) in parts.as_object().unwrap() {
                                    let segment = segments
                                        .iter()
                                        .find(|segment| {
                                            segment.contains(&format!("name=\"{name}\""))
                                        })
                                        .unwrap();
                                    let actual = segment
                                        .split_once("\r\n\r\n")
                                        .unwrap()
                                        .1
                                        .trim_end_matches("\r\n");
                                    if let Some(value) = value.as_str() {
                                        if name.ends_with("_json") {
                                            assert_eq!(
                                                serde_json::from_str::<Value>(actual)?,
                                                serde_json::from_str::<Value>(value)?
                                            );
                                        } else {
                                            assert_eq!(actual, value);
                                        }
                                    } else {
                                        assert_eq!(actual, "Synthetic attachment");
                                    }
                                }
                            } else {
                                assert!(
                                    content_type.starts_with("application/x-www-form-urlencoded")
                                );
                                let mut query = url.clone();
                                query.set_query(Some(text));
                                let actual: std::collections::BTreeMap<_, _> =
                                    query.query_pairs().into_owned().collect();
                                assert_eq!(actual.len(), parts.as_object().unwrap().len());
                                for (name, value) in parts.as_object().unwrap() {
                                    let value = value.as_str().unwrap();
                                    if name.ends_with("_json") {
                                        assert_eq!(
                                            serde_json::from_str::<Value>(&actual[name])?,
                                            serde_json::from_str::<Value>(value)?
                                        );
                                    } else {
                                        assert_eq!(actual[name], value);
                                    }
                                }
                            }
                        }
                        let response = axum::http::Response::builder()
                            .status(expected["status"].as_u64().unwrap() as u16)
                            .body(expected["json"].to_string())?;
                        Ok((response.into(), url))
                    }
                });
        let mut policy = RemoteMcpPolicy::new(vec![instance()])
            .unwrap()
            .with_stock_alaveteli(BTreeSet::from(["synthetic".to_string()]))
            .unwrap();
        policy.use_sdk_stock_fixture("synthetic", client, directory.clone());
        let journal = WriteJournal::open(&directory.join("journal"))
            .await
            .unwrap();
        let gate = RemoteWriteGate::with_journal(journal.clone());
        let db = DbPool::new_in_memory().await.unwrap();
        db.run_migrations().await.unwrap();

        if id == "request_complete" {
            let result = call(
                &db,
                &policy,
                &gate,
                "remote_get_request",
                json!({"instance_id":"synthetic", "request_id":7001}),
            )
            .await;
            assert_eq!(
                result["structuredContent"]["request"]["body"],
                "Synthetic request"
            );
            assert!(!result.to_string().contains(KEY));
        } else {
            let (tool, mut args) = match id {
                "create" => (
                    "remote_create_request",
                    serde_json::from_str::<Value>(
                        case["exchanges"][0]["request_parts"]["request_json"]
                            .as_str()
                            .unwrap(),
                    )
                    .unwrap(),
                ),
                "correspondence_json" | "correspondence_multipart" => {
                    let mut args: Value = serde_json::from_str(
                        case["exchanges"][0]["request_parts"]["correspondence_json"]
                            .as_str()
                            .unwrap(),
                    )
                    .unwrap();
                    args["request_id"] = json!(7001);
                    args["state"] = json!("successful");
                    if id == "correspondence_multipart" {
                        args["attachments"] = json!(["report.txt"]);
                    }
                    ("remote_add_correspondence", args)
                }
                "state_preflight" => (
                    "remote_update_state",
                    json!({"request_id":7001, "expected_current_state":"waiting_response", "state":"successful"}),
                ),
                _ => panic!("unexpected stock case"),
            };
            args["instance_id"] = json!("synthetic");
            args["phase"] = json!("prepare");
            args["idempotency_key"] = json!("fixture-operation");
            let prepared = call(&db, &policy, &gate, tool, args.clone()).await;
            assert!(
                !prepared["isError"].as_bool().unwrap_or(false),
                "prepare failed: {id}"
            );
            let token = prepared["structuredContent"]["confirmation_token"]
                .as_str()
                .unwrap();
            let commit = json!({"instance_id":"synthetic", "phase":"commit", "idempotency_key":"fixture-operation", "confirmation_token":token});
            let result = call(&db, &policy, &gate, tool, commit.clone()).await;
            assert!(
                !result["isError"].as_bool().unwrap_or(false),
                "commit failed: {id}"
            );
            assert!(!result.to_string().contains(KEY));
            match id {
                "create" => assert_eq!(result["structuredContent"]["request"]["id"], 7001),
                "state_preflight" => {
                    assert_eq!(result["structuredContent"]["state"]["updated"], true)
                }
                _ => assert_eq!(
                    result["structuredContent"]["correspondence"]["success"],
                    true
                ),
            }
            let replay = call(&db, &policy, &gate, tool, commit).await;
            assert_eq!(replay["isError"], true);
            let cached = call(&db, &policy, &gate, tool, args).await;
            assert_eq!(cached["structuredContent"]["status"], "already_committed");
        }
        assert!(
            exchanges.lock().unwrap().is_empty(),
            "fixture case was not fully exercised: {id}"
        );
        journal.close().await;
        std::fs::remove_dir_all(&directory).unwrap();
    }
}

#[tokio::test]
async fn stock_source_fails_closed_on_stale_state_errors_and_invalid_writes() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let sends = Arc::new(AtomicUsize::new(0));
    let count = sends.clone();
    let identity = fyi_core::agent_runtime::ClientIdentity::default_identity(None).unwrap();
    let client = SyncClient::with_guardrails("https://source.invalid", &identity, Default::default())
        .unwrap()
        .with_sdk_fixture_transport(move |request| {
            count.fetch_add(1, Ordering::SeqCst);
            async move {
                let url = request.url().clone();
                let body = if request.method() == axum::http::Method::GET {
                    json!({"id":7001,"title":"Synthetic","request_text":"Synthetic request","url":"https://source.invalid/request/example","status":"waiting_response"})
                } else {
                    // The pinned controller can return an error with HTTP 200.
                    json!({"errors":["synthetic private error"]})
                };
                let response = axum::http::Response::builder().status(200).body(body.to_string())?;
                Ok((response.into(), url))
            }
        });
    let stale = client
        .stock_update_state_if_current(
            KEY,
            7001,
            "successful",
            &UpdateRequestStatePayload {
                state: "rejected".into(),
            },
        )
        .await;
    assert!(stale.is_err());
    assert_eq!(
        sends.load(Ordering::SeqCst),
        1,
        "stale preflight must not send the update"
    );
    let mut create = CreateRequestPayload {
        title: "Synthetic".into(),
        body: "Synthetic".into(),
        external_user_name: "Synthetic".into(),
        external_url: "https://source.invalid/external".into(),
        tags: None,
    };
    let error = client
        .stock_create_request(KEY, &create)
        .await
        .unwrap_err()
        .to_string();
    assert!(!error.contains(KEY));
    assert!(!error.contains("synthetic private error"));
    assert_eq!(sends.load(Ordering::SeqCst), 2);
    create.tags = Some("unsupported".into());
    assert!(client.stock_create_request(KEY, &create).await.is_err());
    let correspondence = AddCorrespondencePayload {
        direction: CorrespondenceDirection::Request,
        body: "Synthetic".into(),
        sent_at: "2026-11-22T00:00:00Z".into(),
        state: None,
    };
    assert!(client
        .stock_add_correspondence(
            KEY,
            7001,
            &correspondence,
            &[PathBuf::from("synthetic.txt")]
        )
        .await
        .is_err());
    let response = AddCorrespondencePayload {
        direction: CorrespondenceDirection::Response,
        ..correspondence
    };
    let too_many = vec![PathBuf::from("synthetic.txt"); 9];
    assert!(client
        .stock_add_correspondence(KEY, 7001, &response, &too_many)
        .await
        .is_err());
    let directory = std::env::temp_dir().join(format!(
        "fyi-stock-attachments-{}-{}",
        std::process::id(),
        next_correlation_id()
    ));
    std::fs::create_dir(&directory).unwrap();
    let unsupported = directory.join("attachment.bin");
    std::fs::write(&unsupported, "synthetic").unwrap();
    assert!(client
        .stock_add_correspondence(KEY, 7001, &response, &[unsupported])
        .await
        .is_err());
    let oversized = directory.join("oversized.txt");
    std::fs::File::create(&oversized)
        .unwrap()
        .set_len(50 * 1024 * 1024 + 1)
        .unwrap();
    assert!(client
        .stock_add_correspondence(KEY, 7001, &response, std::slice::from_ref(&oversized))
        .await
        .is_err());
    #[cfg(unix)]
    {
        let link = directory.join("link.txt");
        std::os::unix::fs::symlink(&oversized, &link).unwrap();
        assert!(client
            .stock_add_correspondence(KEY, 7001, &response, &[link])
            .await
            .is_err());
    }
    std::fs::remove_dir_all(directory).unwrap();
    assert_eq!(
        sends.load(Ordering::SeqCst),
        2,
        "unsupported writes must be rejected before dispatch"
    );
}

#[tokio::test]
async fn stock_missing_key_does_not_consume_confirmation_or_dispatch() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let mut config = instance();
    config.instance_id = "unconfigured-test".into();
    assert!(std::env::var_os(config.authority_key_env_name()).is_none());
    let mut policy = RemoteMcpPolicy::new(vec![config])
        .unwrap()
        .with_stock_alaveteli(BTreeSet::from(["unconfigured-test".to_owned()]))
        .unwrap();
    let directory = std::env::temp_dir().join(format!(
        "fyi-stock-missing-key-{}-{}",
        std::process::id(),
        next_correlation_id()
    ));
    std::fs::create_dir(&directory).unwrap();
    let journal = WriteJournal::open(&directory.join("journal"))
        .await
        .unwrap();
    let gate = RemoteWriteGate::with_journal(journal.clone());
    let db = DbPool::new_in_memory().await.unwrap();
    db.run_migrations().await.unwrap();
    let read = call(
        &db,
        &policy,
        &gate,
        "remote_get_request",
        json!({"instance_id":"unconfigured-test", "request_id":7001}),
    )
    .await;
    assert_eq!(read["isError"], true);
    assert_eq!(policy.status().instances[0].requests_used, 0);
    let prepare = call(&db, &policy, &gate, "remote_create_request", json!({"instance_id":"unconfigured-test", "phase":"prepare", "idempotency_key":"missing-key", "title":"Synthetic", "body":"Synthetic", "external_user_name":"Synthetic", "external_url":"https://source.invalid/external"})).await;
    let token = prepare["structuredContent"]["confirmation_token"]
        .as_str()
        .unwrap();
    let commit = json!({"instance_id":"unconfigured-test", "phase":"commit", "confirmation_token":token, "idempotency_key":"missing-key"});
    let denied = call(&db, &policy, &gate, "remote_create_request", commit.clone()).await;
    assert_eq!(denied["isError"], true);
    assert_eq!(policy.status().instances[0].requests_used, 0);
    let count = Arc::new(AtomicUsize::new(0));
    let sends = count.clone();
    let identity = fyi_core::agent_runtime::ClientIdentity::default_identity(None).unwrap();
    let client =
        SyncClient::with_guardrails("https://source.invalid", &identity, Default::default())
            .unwrap()
            .with_sdk_fixture_transport(move |request| {
                sends.fetch_add(1, Ordering::SeqCst);
                async move {
                    let url = request.url().clone();
                    let response = axum::http::Response::builder().status(200).body(
                        json!({"id":7001,"url":"https://source.invalid/request/example"})
                            .to_string(),
                    )?;
                    Ok((response.into(), url))
                }
            });
    policy.use_sdk_stock_fixture("unconfigured-test", client, directory.clone());
    let result = call(&db, &policy, &gate, "remote_create_request", commit).await;
    assert!(!result["isError"].as_bool().unwrap_or(false));
    assert_eq!(count.load(Ordering::SeqCst), 1);
    journal.close().await;
    std::fs::remove_dir_all(directory).unwrap();
}

#[tokio::test]
async fn stock_writes_reject_source_errors_and_never_reveal_the_key() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    for (status, body) in [
        (403, json!({"error":"private"})),
        (302, json!({"url":"https://source.invalid/request/example"})),
        (200, json!({"id":7001})),
        (
            200,
            json!({"id":7001,"url":format!("https://source.invalid/request/example?k={KEY}")}),
        ),
    ] {
        let count = Arc::new(AtomicUsize::new(0));
        let sends = count.clone();
        let identity = fyi_core::agent_runtime::ClientIdentity::default_identity(None).unwrap();
        let client =
            SyncClient::with_guardrails("https://source.invalid", &identity, Default::default())
                .unwrap()
                .with_sdk_fixture_transport(move |request| {
                    sends.fetch_add(1, Ordering::SeqCst);
                    let body = body.clone();
                    async move {
                        let url = request.url().clone();
                        let response = axum::http::Response::builder()
                            .status(status)
                            .header("location", "https://elsewhere.invalid/")
                            .body(body.to_string())?;
                        Ok((response.into(), url))
                    }
                });
        let create = CreateRequestPayload {
            title: "Synthetic".into(),
            body: "Synthetic".into(),
            external_user_name: "Synthetic".into(),
            external_url: "https://source.invalid/external".into(),
            tags: None,
        };
        let error = client
            .stock_create_request(KEY, &create)
            .await
            .unwrap_err()
            .to_string();
        assert!(!error.contains(KEY));
        assert!(!error.contains("private"));
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }
}

#[tokio::test]
async fn stock_state_preflight_forwards_etag_when_present() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let count = Arc::new(AtomicUsize::new(0));
    let sends = count.clone();
    let identity = fyi_core::agent_runtime::ClientIdentity::default_identity(None).unwrap();
    let client = SyncClient::with_guardrails("https://source.invalid", &identity, Default::default()).unwrap()
        .with_sdk_fixture_transport(move |request| {
            let sequence = sends.fetch_add(1, Ordering::SeqCst);
            async move {
                let url = request.url().clone();
                let response = if sequence == 0 {
                    assert_eq!(request.method(), axum::http::Method::GET);
                    axum::http::Response::builder().status(200).header("etag", "\"synthetic-v1\"").body(json!({"id":7001,"title":"Synthetic","request_text":"Synthetic","url":"https://source.invalid/request/example","status":"waiting_response"}).to_string())?
                } else {
                    assert_eq!(sequence, 1);
                    assert_eq!(request.method(), axum::http::Method::POST);
                    assert_eq!(request.headers().get("if-match").unwrap(), "\"synthetic-v1\"");
                    axum::http::Response::builder().status(200).body(json!({"url":"https://source.invalid/request/example"}).to_string())?
                };
                Ok((response.into(), url))
            }
        });
    let updated = client
        .stock_update_state_if_current(
            KEY,
            7001,
            "waiting_response",
            &UpdateRequestStatePayload {
                state: "successful".into(),
            },
        )
        .await
        .unwrap();
    assert!(updated.updated);
    assert_eq!(count.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn legacy_provider_five_cases_remain_unchanged_without_network() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/remote_mcp/upstream-v2.json"
    ))
    .unwrap();
    for id in [
        "request_complete",
        "create",
        "correspondence_json",
        "correspondence_multipart",
        "state_preflight",
    ] {
        let case = fixture["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|case| case["id"] == id)
            .unwrap();
        let directory = std::env::temp_dir().join(format!(
            "fyi-legacy-{}-{}",
            std::process::id(),
            next_correlation_id()
        ));
        std::fs::create_dir(&directory).unwrap();
        std::fs::write(
            directory.join("report.txt"),
            "Synthetic attachment Café 東京",
        )
        .unwrap();
        let exchanges = Arc::new(Mutex::new(VecDeque::from(
            case["exchanges"].as_array().unwrap().clone(),
        )));
        let queued = exchanges.clone();
        let identity = fyi_core::agent_runtime::ClientIdentity::default_identity(None).unwrap();
        let client =
            SyncClient::with_guardrails("https://source.invalid", &identity, Default::default())
                .unwrap()
                .with_sdk_fixture_transport(move |mut request| {
                    let expected = queued
                        .lock()
                        .unwrap()
                        .pop_front()
                        .expect("unexpected legacy source request");
                    async move {
                        assert_eq!(request.method().as_str(), expected["method"]);
                        let url = request.url().clone();
                        assert_eq!(url.path(), expected["path"]);
                        assert!(url.query().is_none());
                        if let Some(expected_json) = expected.get("request_json") {
                            let body = request.body_mut().take().unwrap();
                            let bytes =
                                axum::body::to_bytes(axum::body::Body::new(body), 1024 * 1024)
                                    .await?;
                            assert_eq!(serde_json::from_slice::<Value>(&bytes)?, *expected_json);
                        }
                        if let Some(parts) = expected.get("request_parts") {
                            let body = request.body_mut().take().unwrap();
                            let bytes =
                                axum::body::to_bytes(axum::body::Body::new(body), 1024 * 1024)
                                    .await?;
                            let text = std::str::from_utf8(&bytes)?;
                            for (name, value) in parts.as_object().unwrap() {
                                assert!(text.contains(&format!("name=\"{name}\"")));
                                assert!(text.contains(value.as_str().unwrap()));
                            }
                        }
                        let response = axum::http::Response::builder()
                            .status(expected["status"].as_u64().unwrap() as u16)
                            .body(expected["json"].to_string())?;
                        Ok((response.into(), url))
                    }
                });
        let mut policy = RemoteMcpPolicy::new(vec![instance()]).unwrap();
        assert!(!policy.is_stock_alaveteli("synthetic"));
        policy.use_sdk_stock_fixture("synthetic", client, directory.clone());
        let journal = WriteJournal::open(&directory.join("journal"))
            .await
            .unwrap();
        let gate = RemoteWriteGate::with_journal(journal.clone());
        let db = DbPool::new_in_memory().await.unwrap();
        db.run_migrations().await.unwrap();
        let tool = case["tool"].as_str().unwrap();
        let args = case["arguments"].clone();
        let result = if case["write"].as_bool().unwrap_or(false) {
            let prepared = call(&db, &policy, &gate, tool, args.clone()).await;
            let token = prepared["structuredContent"]["confirmation_token"]
                .as_str()
                .unwrap();
            let commit = json!({"instance_id":"synthetic", "phase":"commit", "idempotency_key":args["idempotency_key"], "confirmation_token":token});
            call(&db, &policy, &gate, tool, commit).await
        } else {
            call(&db, &policy, &gate, tool, args).await
        };
        assert!(
            !result["isError"].as_bool().unwrap_or(false),
            "legacy case failed: {id}"
        );
        let actual = &result["structuredContent"];
        for (name, expected) in case["expected"].as_object().unwrap() {
            assert_eq!(&actual[name], expected, "legacy case: {id}");
        }
        assert!(exchanges.lock().unwrap().is_empty());
        journal.close().await;
        std::fs::remove_dir_all(directory).unwrap();
    }
}
