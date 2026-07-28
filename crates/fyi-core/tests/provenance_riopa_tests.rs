use fyi_core::provenance::{
    append_record, emit_riopa_event_stream, RiopaEmissionContext, RiopaEventStream,
};
use serde_json::Value;

fn synthetic_chain() -> Vec<fyi_core::provenance::ProvenanceRecord> {
    let first = append_record(
        &[],
        "2026-07-01T00:00:00Z",
        "capture/request-42.json",
        b"payload-a",
    );
    let second = append_record(
        std::slice::from_ref(&first),
        "2026-07-01T00:01:00Z",
        "capture/request-42-attachment-1.bin",
        b"payload-b",
    );
    vec![first, second]
}

fn synthetic_context() -> RiopaEmissionContext {
    RiopaEmissionContext {
        generated_at: "2026-07-01T00:02:00Z".to_string(),
        source: "fyi-cli://capture".to_string(),
        input_ref: "urn:fyi:capture-input:synthetic".to_string(),
        output_ref: "urn:fyi:capture-output:synthetic".to_string(),
        agent: "fyi-cli/0.1.0".to_string(),
        rights: "operator-authorized-read-only".to_string(),
        schema_refs: vec![
            "https://example.invalid/riopa/provenance/v1".to_string(),
            "https://github.com/edithatogo/fyi-cli/crates/fyi-core/src/provenance.rs".to_string(),
        ],
    }
}

#[test]
fn emits_expected_golden_riopa_stream_for_synthetic_capture() {
    let chain = synthetic_chain();
    let context = synthetic_context();
    let stream = emit_riopa_event_stream(&chain, &context).expect("emit RIOPA stream");
    let actual = serde_json::to_value(stream).expect("serialize stream");

    let fixture_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/riopa_provenance_stream_golden.json");
    let expected: Value =
        serde_json::from_str(&std::fs::read_to_string(fixture_path).expect("read fixture"))
            .expect("parse fixture");

    assert_eq!(actual, expected);
}

#[test]
fn riopa_stream_roundtrip_preserves_mapping_and_semantic_gap_annotations() {
    let chain = synthetic_chain();
    let context = synthetic_context();
    let stream = emit_riopa_event_stream(&chain, &context).expect("emit RIOPA stream");
    let encoded = serde_json::to_string(&stream).expect("encode stream");
    let restored: RiopaEventStream = serde_json::from_str(&encoded).expect("decode stream");

    assert_eq!(restored.events.len(), 2);
    assert_eq!(restored.semantic_gaps.len(), 3);
    assert!(restored
        .unsupported_fields
        .contains(&"signature".to_string()));
    assert_eq!(restored.events[0].integrity.prev_hash, "0".repeat(64));
}
