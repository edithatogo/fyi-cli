//! Sync receipt regressions use only in-memory, synthetic evidence.
use fyi_core::acquisition_receipts::{
    build_sync_command_receipt, canonical_json_bytes, verify_receipt, SyncReceiptInput,
};
use serde_json::{json, Value};

fn legacy_empty_sync_receipt(command: &str, status: &str) -> Value {
    let mut receipt: Value = serde_json::from_str(include_str!(
        "../../../conformance/acquisition/receipt-v1-python.json"
    ))
    .unwrap();
    receipt["command"] = json!(command);
    receipt["adapter"]["id"] = json!("alaveteli-sync");
    receipt["status"] = json!(status);
    receipt["responses"] = json!([]);
    receipt["totals"] = json!({"requests": 0, "bytes": 0, "retries": 0});
    if status == "failed" {
        receipt["failure"] = json!({"type": "SyncError"});
    } else {
        receipt.as_object_mut().unwrap().remove("failure");
    }
    let mut unsigned = receipt.as_object().unwrap().clone();
    unsigned.remove("receipt_sha256");
    receipt["receipt_sha256"] = json!(fyi_core::provenance::sha256_hex(
        &canonical_json_bytes(&Value::Object(unsigned)).unwrap()
    ));
    receipt
}

#[test]
fn sync_builder_rejects_a_projection_without_transport_evidence() {
    for command in ["sync-pull", "sync-push"] {
        let result = build_sync_command_receipt(SyncReceiptInput {
            command,
            source_url: "https://example.invalid",
            request_bounds: json!({}),
            started_at: "2026-11-16T00:00:00Z",
            completed_at: "2026-11-16T00:00:01Z",
            result_projection: b"{}\n",
            failure_type: None,
        });
        assert!(
            result.is_err(),
            "{command} fabricated a zero-response receipt"
        );
    }
}

#[test]
fn verifier_rejects_preexisting_ambiguous_empty_sync_receipts() {
    for command in ["sync-pull", "sync-push"] {
        for status in ["succeeded", "failed"] {
            let receipt = legacy_empty_sync_receipt(command, status);
            assert!(
                verify_receipt(&receipt).is_err(),
                "accepted {status} {command} without observed transport responses"
            );
        }
    }
}

#[test]
fn verifier_retains_observed_response_receipts() {
    let mut receipt: Value = serde_json::from_str(include_str!(
        "../../../conformance/acquisition/receipt-v1-python.json"
    ))
    .unwrap();
    receipt["command"] = json!("sync-pull");
    receipt["adapter"]["id"] = json!("alaveteli-sync");
    let mut unsigned = receipt.as_object().unwrap().clone();
    unsigned.remove("receipt_sha256");
    receipt["receipt_sha256"] = json!(fyi_core::provenance::sha256_hex(
        &canonical_json_bytes(&Value::Object(unsigned)).unwrap()
    ));
    verify_receipt(&receipt).unwrap();
}

#[test]
fn verifier_still_allows_empty_failure_receipts_from_other_adapters() {
    let mut receipt = legacy_empty_sync_receipt("discover", "failed");
    receipt["adapter"]["id"] = json!("alaveteli-search-feed");
    let mut unsigned = receipt.as_object().unwrap().clone();
    unsigned.remove("receipt_sha256");
    receipt["receipt_sha256"] = json!(fyi_core::provenance::sha256_hex(
        &canonical_json_bytes(&Value::Object(unsigned)).unwrap()
    ));
    verify_receipt(&receipt).unwrap();
}
