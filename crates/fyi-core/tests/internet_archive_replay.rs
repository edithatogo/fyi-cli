use fyi_core::{acquisition_receipts, internet_archive_replay as adapter, provenance};

use acquisition_receipts::canonical_json_bytes;
use adapter::{
    replay_approved_rows, replay_approved_rows_with_runtime, validate_selection_bytes,
    write_replay_result, ReplayConfig, ReplayFetchError, ReplayObservation, ReplayResponse,
    ReplayRuntime, CHECKPOINT_SCHEMA, MAX_PAYLOAD_BYTES, REPLAY_ADAPTER_ID, REPLAY_ORIGIN,
    RESULT_SCHEMA, SELECTION_SCHEMA,
};
use provenance::sha256_hex;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const ORACLE: &str = include_str!("../../../conformance/internet-archive-replay/oracle-v1.json");
const SCHEMA: &[u8] =
    include_bytes!("../../../conformance/internet-archive-replay/selection-v1.schema.json");

fn oracle() -> Value {
    serde_json::from_str(ORACLE).unwrap()
}

fn case(name: &str) -> Value {
    oracle()["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["name"] == name)
        .unwrap()
        .clone()
}

fn test_directory() -> tempfile::TempDir {
    let parent = std::fs::canonicalize(std::env::temp_dir()).unwrap();
    tempfile::Builder::new()
        .prefix("fyi-replay-contract-")
        .tempdir_in(parent)
        .unwrap()
}

fn bytes(hex: &str) -> Vec<u8> {
    let (pairs, remainder) = hex.as_bytes().as_chunks::<2>();
    assert!(remainder.is_empty());
    pairs
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn response(event: &Value, requested_url: &str) -> ReplayResponse {
    let mut headers = reqwest::header::HeaderMap::new();
    for (name, value) in event["headers"].as_object().unwrap() {
        headers.insert(
            name.parse::<reqwest::header::HeaderName>().unwrap(),
            value.as_str().unwrap().parse().unwrap(),
        );
    }
    ReplayResponse {
        url: event["url"]
            .as_str()
            .unwrap_or(requested_url)
            .parse()
            .unwrap(),
        status: reqwest::StatusCode::from_u16(event["status"].as_u64().unwrap() as u16).unwrap(),
        version: reqwest::Version::HTTP_11,
        headers,
        body: bytes(event["body_hex"].as_str().unwrap()),
        captured_at: "2026-11-14T00:00:00Z".parse().unwrap(),
    }
}

fn observation(value: &ReplayObservation<'_>) -> Option<Value> {
    assert!(value.requested_url.starts_with(REPLAY_ORIGIN));
    assert_eq!(value.attempts, 1);
    assert!(value.retry_delays_seconds.is_empty());
    value.response.map(|response| {
        json!({
            "url": response.url.as_str(), "status": response.status.as_u16(),
            "bytes": response.body.len(), "sha256": sha256_hex(&response.body),
            "attempts": value.attempts, "retry_delays_seconds": value.retry_delays_seconds
        })
    })
}

#[derive(Default)]
struct FakeRuntime(Arc<Mutex<Duration>>);

impl ReplayRuntime for FakeRuntime {
    fn now(&self) -> Duration {
        *self.0.lock().unwrap()
    }

    async fn sleep(&mut self, _duration: Duration) {
        panic!("replay must never retry or sleep")
    }
}

fn forbidden_fetch(
    _: String,
    _: Duration,
    _: usize,
) -> std::future::Ready<Result<ReplayResponse, ReplayFetchError>> {
    panic!("invalid replay state must fail before HTTP")
}

fn initialize(case: &Value, root: &Path) -> (PathBuf, PathBuf) {
    let output = root.join("payloads");
    let checkpoint = root.join("checkpoint.json");
    if let Some(initial) = case["initial"].as_object() {
        std::fs::create_dir(&output).unwrap();
        for (name, value) in initial["files"].as_object().unwrap() {
            std::fs::write(output.join(name), bytes(value.as_str().unwrap())).unwrap();
        }
        if let Some(contents) = initial.get("checkpoint_json").and_then(Value::as_str) {
            std::fs::write(&checkpoint, contents).unwrap();
        }
    }
    (output, checkpoint)
}

fn files(path: &Path) -> BTreeMap<String, String> {
    if !path.exists() {
        return BTreeMap::new();
    }
    std::fs::read_dir(path)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            assert!(entry.file_type().unwrap().is_file());
            (
                entry.file_name().to_str().unwrap().to_string(),
                hex(&std::fs::read(entry.path()).unwrap()),
            )
        })
        .collect()
}

#[test]
fn replay_oracle_provenance_is_frozen() {
    let mut value = oracle();
    assert_eq!(value["oracle"]["python"], "3.15.1");
    assert_eq!(value["oracle"]["network"], "mocked-only");
    assert_eq!(value["oracle"]["schema_sha256"], sha256_hex(SCHEMA));
    let digest = value
        .as_object_mut()
        .unwrap()
        .remove("fixture_sha256")
        .unwrap();
    assert_eq!(digest, sha256_hex(&canonical_json_bytes(&value).unwrap()));
    assert_eq!(value["cases"].as_array().unwrap().len(), 30);
    assert_eq!(value["invalid_selections"].as_array().unwrap().len(), 24);
    assert_eq!(value["invalid_configs"].as_array().unwrap().len(), 10);
    assert_eq!(REPLAY_ADAPTER_ID, "internet-archive-replay");
    assert_eq!(MAX_PAYLOAD_BYTES, 16 * 1024 * 1024);
    assert_eq!(
        SELECTION_SCHEMA,
        "urn:fyi-cli:internet-archive-replay-selection:1"
    );
    assert_eq!(
        CHECKPOINT_SCHEMA,
        "urn:fyi-cli:internet-archive-replay-checkpoint:1"
    );
    assert_eq!(
        RESULT_SCHEMA,
        "urn:fyi-cli:internet-archive-replay-result:1"
    );
}

#[test]
fn replay_rejects_oversized_selection_before_parsing() {
    let bytes = vec![b' '; 16 * 1024 * 1024 + 1];
    let error = validate_selection_bytes(&bytes).unwrap_err();
    assert!(format!("{error:#}").contains("replay selection exceeds byte limit"));
}

#[tokio::test]
async fn replay_rejects_oversized_checkpoint_before_source_io() {
    let baseline = case("baseline");
    let selection =
        validate_selection_bytes(baseline["selection_json"].as_str().unwrap().as_bytes()).unwrap();
    let config: ReplayConfig = serde_json::from_value(baseline["config"].clone()).unwrap();
    let directory = test_directory();
    let checkpoint = directory.path().join("cp.json");
    std::fs::write(&checkpoint, vec![b' '; 16 * 1024 * 1024 + 1]).unwrap();
    let error = replay_approved_rows(
        &selection,
        &config,
        &directory.path().join("payloads"),
        &checkpoint,
        forbidden_fetch,
        |_| Ok(()),
    )
    .await
    .unwrap_err();
    assert!(format!("{error:#}").contains("replay checkpoint exceeds byte limit"));
}

#[tokio::test]
async fn replay_checkpoint_overflow_preserves_resumable_progress() {
    for completed_before_failure in [0, 1] {
        let baseline = case("baseline");
        let mut document: Value =
            serde_json::from_str(baseline["selection_json"].as_str().unwrap()).unwrap();
        let mut second = document["rows"][0].clone();
        second["row_index"] = json!(1);
        second["original"] = json!("https://example.test/request/2.json");
        document["rows"].as_array_mut().unwrap().push(second);
        document["rows"][completed_before_failure]
            .as_object_mut()
            .unwrap()
            .remove("expected_media_type");
        document.as_object_mut().unwrap().remove("selection_sha256");
        document["selection_sha256"] = json!(sha256_hex(&canonical_json_bytes(&document).unwrap()));
        let selection =
            validate_selection_bytes(&canonical_json_bytes(&document).unwrap()).unwrap();
        let config: ReplayConfig = serde_json::from_value(baseline["config"].clone()).unwrap();
        let directory = test_directory();
        let output = directory.path().join("payloads");
        let checkpoint = directory.path().join("checkpoint.json");
        let prior_checkpoint = Arc::new(Mutex::new(None));
        let captured_checkpoint = prior_checkpoint.clone();
        let calls = AtomicUsize::new(0);
        let event = &baseline["events"][0];

        let outcome = replay_approved_rows(
            &selection,
            &config,
            &output,
            &checkpoint,
            |url, _, _| {
                let position = calls.fetch_add(1, Ordering::SeqCst);
                let mut reply = response(event, &url);
                if position == completed_before_failure {
                    *captured_checkpoint.lock().unwrap() = std::fs::read(&checkpoint).ok();
                    // Valid, unapproved response metadata must not make the next
                    // checkpoint larger than the adapter can reopen on resume.
                    reply.headers.insert(
                        reqwest::header::CONTENT_TYPE,
                        format!("application/{}", "x".repeat(16 * 1024 * 1024))
                            .parse()
                            .unwrap(),
                    );
                }
                std::future::ready(Ok(reply))
            },
            |_| Ok(()),
        )
        .await;

        assert!(
            outcome.is_err(),
            "oversized checkpoint must not be published"
        );
        let error = outcome.unwrap_err();
        assert!(format!("{error:#}").contains("replay checkpoint exceeds byte limit"));
        assert_eq!(calls.load(Ordering::SeqCst), completed_before_failure + 1);
        assert_eq!(
            std::fs::read(&checkpoint).ok(),
            *prior_checkpoint.lock().unwrap()
        );
        assert_eq!(files(&output).len(), completed_before_failure);

        let resumed_calls = AtomicUsize::new(0);
        let result = replay_approved_rows(
            &selection,
            &config,
            &output,
            &checkpoint,
            |url, _, _| {
                resumed_calls.fetch_add(1, Ordering::SeqCst);
                std::future::ready(Ok(response(event, &url)))
            },
            |_| Ok(()),
        )
        .await
        .unwrap();
        assert_eq!(
            resumed_calls.load(Ordering::SeqCst),
            2 - completed_before_failure
        );
        assert_eq!(files(&output).len(), 2);
        let result: Value = serde_json::from_slice(&result.canonical_bytes().unwrap()).unwrap();
        assert_eq!(result["objects"].as_array().unwrap().len(), 2);
    }
}

#[tokio::test]
async fn replay_rejects_an_unrepresentable_object_name_before_source_io() {
    let baseline = case("baseline");
    let original = baseline["selection_json"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(original).unwrap();
    let old_hash = parsed["selection_sha256"].as_str().unwrap();
    let altered = original.replace(
        "\"row_index\":0",
        &format!("\"row_index\":{}", "9".repeat(200)),
    );
    let unsigned = altered.replace(&format!("\"selection_sha256\":\"{old_hash}\","), "");
    let signed = altered.replace(old_hash, &sha256_hex(unsigned.as_bytes()));
    let selection = validate_selection_bytes(signed.as_bytes()).unwrap();
    let config: ReplayConfig = serde_json::from_value(baseline["config"].clone()).unwrap();
    let directory = test_directory();
    let error = replay_approved_rows(
        &selection,
        &config,
        &directory.path().join("payloads"),
        &directory.path().join("cp.json"),
        forbidden_fetch,
        |_| Ok(()),
    )
    .await
    .unwrap_err();
    assert!(format!("{error:#}").contains("replay object name exceeds limit"));
}

#[test]
fn replay_selection_and_config_identities_match_frozen_python() {
    let fixture = oracle();
    for case in fixture["cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let selection_json = case["selection_json"].as_str().unwrap();
        let selection = validate_selection_bytes(selection_json.as_bytes()).unwrap();
        let expected = selection_json.replace("\"row_index\":-0", "\"row_index\":0");
        assert_eq!(
            selection.canonical_bytes().unwrap(),
            expected.as_bytes(),
            "{name}"
        );
        assert!(!selection.is_empty());
        assert_ne!(selection.len(), 0);
        let config: ReplayConfig = serde_json::from_value(case["config"].clone()).unwrap();
        config.validate().unwrap();
        let identity = json!({
            "replay_origin": REPLAY_ORIGIN, "allowed_target_host": config.allowed_target_host,
            "selection_sha256": selection.selection_sha256(),
            "source_cdx_sha256": selection.source_cdx_sha256(),
            "max_payload_bytes": config.max_payload_bytes
        });
        assert_eq!(config.identity(&selection), identity, "{name}");
        let mut bounds = identity;
        bounds["max_rows"] = json!(config.max_rows);
        bounds["max_redirects"] = json!(config.max_redirects);
        bounds["max_runtime_seconds"] = json!(config.max_runtime_seconds);
        bounds["request_timeout_seconds"] = json!(config.request_timeout_seconds);
        assert_eq!(config.request_bounds(&selection), bounds, "{name}");
    }
    for case in fixture["invalid_selections"].as_array().unwrap() {
        assert!(
            validate_selection_bytes(case["selection_json"].as_str().unwrap().as_bytes()).is_err(),
            "{}",
            case["name"]
        );
    }
    for case in fixture["invalid_configs"].as_array().unwrap() {
        if let Ok(config) = serde_json::from_value::<ReplayConfig>(case["config"].clone()) {
            assert!(config.validate().is_err(), "{case}");
        }
    }
    let mut config: ReplayConfig =
        serde_json::from_value(case("baseline")["config"].clone()).unwrap();
    for value in [f64::INFINITY, f64::NAN, f64::MAX] {
        config.max_runtime_seconds = value;
        assert!(config.validate().is_err());
    }
}

#[tokio::test]
async fn replay_matches_frozen_python_results_checkpoints_and_recovery() {
    for case in oracle()["cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let directory = test_directory();
        let (output, checkpoint) = initialize(case, directory.path());
        let selection =
            validate_selection_bytes(case["selection_json"].as_str().unwrap().as_bytes()).unwrap();
        let config: ReplayConfig = serde_json::from_value(case["config"].clone()).unwrap();
        let events = case["events"].as_array().unwrap().clone();
        let calls = Arc::new(Mutex::new(Vec::new()));
        let observed = Arc::new(Mutex::new(Vec::new()));
        let called = calls.clone();
        let seen = observed.clone();
        let mut runtime = FakeRuntime::default();
        let result = replay_approved_rows_with_runtime(
            &selection,
            &config,
            &output,
            &checkpoint,
            move |url, timeout, cap| {
                let mut calls = called.lock().unwrap();
                let event = events
                    .get(calls.len())
                    .expect("unexpected request beyond frozen exchange budget")
                    .clone();
                calls.push(
                    json!({"url":url,"timeout_seconds":timeout.as_secs_f64(),"byte_cap":cap}),
                );
                std::future::ready(if event["error"].is_string() {
                    Err(ReplayFetchError::Retryable(anyhow::anyhow!(
                        "synthetic offline transport failure"
                    )))
                } else {
                    Ok(response(&event, &url))
                })
            },
            move |value| {
                if let Some(value) = observation(value) {
                    seen.lock().unwrap().push(value);
                }
                Ok(())
            },
            &mut runtime,
        )
        .await;
        let expected = &case["expected"];
        if let Some(expected) = expected["result_json"].as_str() {
            let result = result.unwrap_or_else(|error| panic!("{name}: {error:#}"));
            assert_eq!(
                result.canonical_bytes().unwrap(),
                expected.as_bytes(),
                "{name}"
            );
            assert_eq!(
                serde_json::to_vec(&result).unwrap(),
                expected.trim_end().as_bytes(),
                "{name}"
            );
            let result_path = directory.path().join("result.json");
            write_replay_result(&result_path, &result).unwrap();
            assert_eq!(
                std::fs::read(result_path).unwrap(),
                expected.as_bytes(),
                "{name}"
            );
        } else {
            assert!(result.is_err(), "{name}");
        }
        assert_eq!(
            *calls.lock().unwrap(),
            expected["calls"].as_array().unwrap().clone(),
            "{name}"
        );
        assert_eq!(
            *observed.lock().unwrap(),
            expected["observations"].as_array().unwrap().clone(),
            "{name}"
        );
        assert_eq!(
            std::fs::read_to_string(&checkpoint).ok().as_deref(),
            expected["checkpoint_json"].as_str(),
            "{name}"
        );
        assert_eq!(
            serde_json::to_value(files(&output)).unwrap(),
            expected["files"],
            "{name}"
        );
    }
}

#[tokio::test]
async fn replay_compressed_entities_preserve_raw_evidence_and_both_byte_caps() {
    let case = oracle()["legacy_negative_evidence"][0].clone();
    assert_eq!(
        case["expected"]["error"]["message"],
        "Wayback declared length does not match payload"
    );
    let selection =
        validate_selection_bytes(case["selection_json"].as_str().unwrap().as_bytes()).unwrap();
    let config: ReplayConfig = serde_json::from_value(case["config"].clone()).unwrap();
    let event = &case["events"][0];
    let directory = test_directory();
    let output = directory.path().join("payloads");
    let checkpoint = directory.path().join("checkpoint.json");
    let observed = Arc::new(Mutex::new(Vec::new()));
    let seen = observed.clone();
    let result = replay_approved_rows(
        &selection,
        &config,
        &output,
        &checkpoint,
        |url, _, cap| {
            assert_eq!(cap, config.max_payload_bytes);
            std::future::ready(Ok(response(event, &url)))
        },
        move |value| {
            seen.lock().unwrap().push(observation(value).unwrap());
            Ok(())
        },
    )
    .await
    .unwrap();
    assert_eq!(
        result.canonical_bytes().unwrap(),
        case["native_expected_result_json"]
            .as_str()
            .unwrap()
            .as_bytes()
    );
    assert_eq!(
        std::fs::read(&checkpoint).unwrap(),
        case["native_expected_checkpoint_json"]
            .as_str()
            .unwrap()
            .as_bytes()
    );
    assert_eq!(
        observed.lock().unwrap()[0]["sha256"],
        sha256_hex(&bytes(event["body_hex"].as_str().unwrap()))
    );
    assert_ne!(observed.lock().unwrap()[0]["bytes"], 50);

    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(&vec![b'x'; 4096]).unwrap();
    let expanded = encoder.finish().unwrap();
    for mode in ["encoded_cap", "decoded_cap", "encoded_length"] {
        let directory = test_directory();
        let mut config = config.clone();
        let mut response = response(
            event,
            &format!("{REPLAY_ORIGIN}/web/20250101000000id_/https://example.test/request/1.json"),
        );
        if mode == "encoded_cap" {
            config.max_payload_bytes = 50;
        }
        if mode == "decoded_cap" {
            response.body = expanded.clone();
            response.headers.insert(
                reqwest::header::CONTENT_LENGTH,
                expanded.len().to_string().parse().unwrap(),
            );
        }
        if mode == "encoded_length" {
            response
                .headers
                .insert(reqwest::header::CONTENT_LENGTH, "1".parse().unwrap());
        }
        let mut response = Some(response);
        assert!(
            replay_approved_rows(
                &selection,
                &config,
                &directory.path().join("payloads"),
                &directory.path().join("cp.json"),
                |_, _, _| std::future::ready(Ok(response.take().unwrap())),
                |_| Ok(())
            )
            .await
            .is_err(),
            "{mode}"
        );
        assert!(
            files(&directory.path().join("payloads")).is_empty(),
            "{mode}"
        );
        assert!(!directory.path().join("cp.json").exists(), "{mode}");
    }
}

#[tokio::test]
async fn replay_checkpoint_recovery_rebinds_every_object_to_approval() {
    let forged = oracle()["legacy_negative_evidence"][1].clone();
    assert!(forged["expected"]["error"].is_null());
    let directory = test_directory();
    let (output, checkpoint) = initialize(&forged, directory.path());
    let selection =
        validate_selection_bytes(forged["selection_json"].as_str().unwrap().as_bytes()).unwrap();
    let config: ReplayConfig = serde_json::from_value(forged["config"].clone()).unwrap();
    assert!(replay_approved_rows(
        &selection,
        &config,
        &output,
        &checkpoint,
        forbidden_fetch,
        |_| Ok(())
    )
    .await
    .is_err());
    for field in [
        "original",
        "timestamp",
        "cdx_digest",
        "status",
        "media_type",
        "final_url",
        "path",
    ] {
        let baseline = case("resume_complete");
        let directory = test_directory();
        let (output, checkpoint) = initialize(&baseline, directory.path());
        let mut value: Value =
            serde_json::from_str(&std::fs::read_to_string(&checkpoint).unwrap()).unwrap();
        value["completed"][0][field] = if field == "status" {
            json!(201)
        } else {
            json!("changed")
        };
        value.as_object_mut().unwrap().remove("checkpoint_sha256");
        value["checkpoint_sha256"] = json!(sha256_hex(&canonical_json_bytes(&value).unwrap()));
        std::fs::write(&checkpoint, canonical_json_bytes(&value).unwrap()).unwrap();
        assert!(
            replay_approved_rows(
                &selection,
                &config,
                &output,
                &checkpoint,
                forbidden_fetch,
                |_| Ok(())
            )
            .await
            .is_err(),
            "{field}"
        );
    }
}

#[tokio::test]
async fn replay_observer_failure_aborts_before_payload_or_checkpoint_progress() {
    let case = case("baseline");
    let directory = test_directory();
    let selection =
        validate_selection_bytes(case["selection_json"].as_str().unwrap().as_bytes()).unwrap();
    let config: ReplayConfig = serde_json::from_value(case["config"].clone()).unwrap();
    let calls = AtomicUsize::new(0);
    let result = replay_approved_rows(
        &selection,
        &config,
        &directory.path().join("payloads"),
        &directory.path().join("cp.json"),
        |url, _, _| {
            calls.fetch_add(1, Ordering::SeqCst);
            std::future::ready(Ok(response(&case["events"][0], &url)))
        },
        |_| anyhow::bail!("synthetic evidence storage failure"),
    )
    .await;
    assert!(format!("{:#}", result.unwrap_err()).contains("synthetic evidence storage failure"));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(files(&directory.path().join("payloads")).is_empty());
    assert!(!directory.path().join("cp.json").exists());
}

#[tokio::test]
async fn replay_runtime_deadlines_bound_each_exchange_without_retries() {
    let case = case("baseline");
    let selection =
        validate_selection_bytes(case["selection_json"].as_str().unwrap().as_bytes()).unwrap();
    let mut config: ReplayConfig = serde_json::from_value(case["config"].clone()).unwrap();
    config.max_runtime_seconds = 1.0;
    let directory = test_directory();
    let mut runtime = FakeRuntime::default();
    *runtime.0.lock().unwrap() = Duration::from_secs(4);
    let clock = runtime.0.clone();
    let seen = AtomicUsize::new(0);
    assert!(replay_approved_rows_with_runtime(
        &selection,
        &config,
        &directory.path().join("payloads"),
        &directory.path().join("cp.json"),
        |url, timeout, _| {
            assert_eq!(timeout, Duration::from_secs(1));
            *clock.lock().unwrap() += Duration::from_secs(2);
            std::future::ready(Ok(response(&case["events"][0], &url)))
        },
        |_| {
            seen.fetch_add(1, Ordering::SeqCst);
            Ok(())
        },
        &mut runtime
    )
    .await
    .is_err());
    assert_eq!(seen.load(Ordering::SeqCst), 1);
    assert!(files(&directory.path().join("payloads")).is_empty());

    config.request_timeout_seconds = 0.01;
    let directory = test_directory();
    let called = AtomicUsize::new(0);
    let failed = AtomicUsize::new(0);
    assert!(replay_approved_rows(
        &selection,
        &config,
        &directory.path().join("payloads"),
        &directory.path().join("cp.json"),
        |_, _, _| {
            called.fetch_add(1, Ordering::SeqCst);
            std::future::pending::<Result<ReplayResponse, ReplayFetchError>>()
        },
        |value| {
            assert!(value.response.is_none());
            failed.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    )
    .await
    .is_err());
    assert_eq!(called.load(Ordering::SeqCst), 1);
    assert_eq!(failed.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn replay_paths_reject_checkpoint_overlap_and_transport_normalization() {
    let case = case("baseline");
    let selection =
        validate_selection_bytes(case["selection_json"].as_str().unwrap().as_bytes()).unwrap();
    let config: ReplayConfig = serde_json::from_value(case["config"].clone()).unwrap();
    let directory = test_directory();
    let output = directory.path().join("payloads");
    assert!(replay_approved_rows(
        &selection,
        &config,
        &output,
        &output.join("cp.json"),
        forbidden_fetch,
        |_| Ok(())
    )
    .await
    .is_err());
    let mut raw: Value = serde_json::from_str(case["selection_json"].as_str().unwrap()).unwrap();
    raw["rows"][0]["original"] = json!("https://example.test/a/%2e%2e/request/1.json");
    raw.as_object_mut().unwrap().remove("selection_sha256");
    raw["selection_sha256"] = json!(sha256_hex(&canonical_json_bytes(&raw).unwrap()));
    let selection = validate_selection_bytes(&canonical_json_bytes(&raw).unwrap()).unwrap();
    assert!(replay_approved_rows(
        &selection,
        &config,
        &output,
        &directory.path().join("cp.json"),
        forbidden_fetch,
        |_| Ok(())
    )
    .await
    .is_err());
}

#[cfg(unix)]
#[tokio::test]
async fn replay_paths_reject_symlink_outputs_checkpoints_and_objects() {
    use std::os::unix::fs::symlink;
    let baseline = case("baseline");
    let selection =
        validate_selection_bytes(baseline["selection_json"].as_str().unwrap().as_bytes()).unwrap();
    let config: ReplayConfig = serde_json::from_value(baseline["config"].clone()).unwrap();
    for mode in [
        "output",
        "output_parent",
        "checkpoint",
        "checkpoint_parent",
        "object",
    ] {
        let directory = test_directory();
        let target = directory.path().join("target");
        std::fs::create_dir(&target).unwrap();
        let output = directory.path().join("payloads");
        let mut checkpoint = directory.path().join("cp.json");
        match mode {
            "output" => symlink(&target, &output).unwrap(),
            "output_parent" => {
                symlink(&target, &output).unwrap();
            }
            "checkpoint" => {
                std::fs::write(target.join("cp.json"), "{}").unwrap();
                symlink(target.join("cp.json"), &checkpoint).unwrap();
            }
            "checkpoint_parent" => {
                let link = directory.path().join("link");
                symlink(&target, &link).unwrap();
                checkpoint = link.join("cp.json");
            }
            "object" => {
                std::fs::create_dir(&output).unwrap();
                let name = baseline["expected"]["files"]
                    .as_object()
                    .unwrap()
                    .keys()
                    .next()
                    .unwrap();
                let payload = target.join("payload.raw");
                std::fs::write(&payload, bytes(oracle()["payload_hex"].as_str().unwrap())).unwrap();
                symlink(payload, output.join(name)).unwrap();
            }
            _ => unreachable!(),
        }
        let result = replay_approved_rows(
            &selection,
            &config,
            &output,
            &checkpoint,
            |url, _, _| std::future::ready(Ok(response(&baseline["events"][0], &url))),
            |_| Ok(()),
        )
        .await;
        assert!(result.is_err(), "{mode}");
        assert!(!target.join("000000.raw").exists());
    }
}
