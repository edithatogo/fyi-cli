//! Signed-ish provenance for archived payloads via a SHA-256 hash chain.
//!
//! This is intentionally lighter than full sigstore/cosign: each record hashes
//! its payload together with the previous record hash, producing a tamper-evident
//! chain suitable for local archive integrity checks.

use data_encoding::HEXLOWER;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Genesis previous-hash used for the first record in a chain.
pub const GENESIS_PREV_HASH: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

/// One link in a provenance hash chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvenanceRecord {
    /// Monotonic sequence number (0-based).
    pub sequence: u64,
    /// ISO-8601 or free-form timestamp string supplied by the archiver.
    pub recorded_at: String,
    /// Logical archive path or content identifier.
    pub payload_id: String,
    /// SHA-256 hex digest of the raw payload bytes.
    pub payload_hash: String,
    /// Hex SHA-256 of the previous record's `record_hash` (or genesis).
    pub prev_hash: String,
    /// Hex SHA-256 over the canonical fields of this record.
    pub record_hash: String,
}

/// Context fields required to emit RIOPA-compatible provenance events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RiopaEmissionContext {
    pub generated_at: String,
    pub source: String,
    pub input_ref: String,
    pub output_ref: String,
    pub agent: String,
    pub rights: String,
    pub schema_refs: Vec<String>,
}

/// One emitted RIOPA-compatible artifact/capture event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RiopaArtifactEvent {
    pub event_id: String,
    pub event_type: String,
    pub recorded_at: String,
    pub source: String,
    pub input: RiopaIoRef,
    pub output: RiopaIoRef,
    pub agent: String,
    pub rights: String,
    pub schema_refs: Vec<String>,
    pub integrity: RiopaIntegrity,
    pub chain: RiopaChainLink,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RiopaIoRef {
    pub reference: String,
    pub hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RiopaIntegrity {
    pub chain_algorithm: String,
    pub prev_hash: String,
    pub payload_hash: String,
    pub record_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RiopaChainLink {
    pub sequence: u64,
    pub payload_id: String,
}

/// Captured RIOPA emission payload including semantic-loss notes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RiopaEventStream {
    pub profile: String,
    pub generated_at: String,
    pub source: String,
    pub input_ref: String,
    pub output_ref: String,
    pub agent: String,
    pub rights: String,
    pub schema_refs: Vec<String>,
    pub semantic_gaps: Vec<String>,
    pub unsupported_fields: Vec<String>,
    pub events: Vec<RiopaArtifactEvent>,
}

/// Compute SHA-256 hex digest of arbitrary bytes.
pub fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    HEXLOWER.encode(&hasher.finalize())
}

/// Hash payload bytes for storage in [`ProvenanceRecord::payload_hash`].
pub fn hash_payload(payload: &[u8]) -> String {
    sha256_hex(payload)
}

fn canonical_record_bytes(
    sequence: u64,
    recorded_at: &str,
    payload_id: &str,
    payload_hash: &str,
    prev_hash: &str,
) -> Vec<u8> {
    // Fixed field order for stable hashing across versions.
    format!("v1|{sequence}|{recorded_at}|{payload_id}|{payload_hash}|{prev_hash}").into_bytes()
}

/// Append a new provenance record for `payload`.
pub fn append_record(
    chain: &[ProvenanceRecord],
    recorded_at: &str,
    payload_id: &str,
    payload: &[u8],
) -> ProvenanceRecord {
    let sequence = chain.len() as u64;
    let prev_hash = chain
        .last()
        .map(|r| r.record_hash.clone())
        .unwrap_or_else(|| GENESIS_PREV_HASH.to_string());
    let payload_hash = hash_payload(payload);
    let record_hash = sha256_hex(&canonical_record_bytes(
        sequence,
        recorded_at,
        payload_id,
        &payload_hash,
        &prev_hash,
    ));
    ProvenanceRecord {
        sequence,
        recorded_at: recorded_at.to_string(),
        payload_id: payload_id.to_string(),
        payload_hash,
        prev_hash,
        record_hash,
    }
}

/// Errors produced when verifying a provenance chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProvenanceError {
    EmptyChain,
    SequenceGap {
        index: usize,
        expected: u64,
        found: u64,
    },
    PrevHashMismatch {
        index: usize,
    },
    RecordHashMismatch {
        index: usize,
    },
    PayloadHashMismatch {
        index: usize,
    },
}

impl std::fmt::Display for ProvenanceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyChain => write!(f, "provenance chain is empty"),
            Self::SequenceGap {
                index,
                expected,
                found,
            } => write!(
                f,
                "sequence gap at index {index}: expected {expected}, found {found}"
            ),
            Self::PrevHashMismatch { index } => {
                write!(f, "previous hash mismatch at index {index}")
            }
            Self::RecordHashMismatch { index } => {
                write!(f, "record hash mismatch at index {index}")
            }
            Self::PayloadHashMismatch { index } => {
                write!(f, "payload hash mismatch at index {index}")
            }
        }
    }
}

impl std::error::Error for ProvenanceError {}

/// Verify structural integrity of a hash chain (without re-hashing payloads).
pub fn verify_chain(chain: &[ProvenanceRecord]) -> Result<(), ProvenanceError> {
    if chain.is_empty() {
        return Err(ProvenanceError::EmptyChain);
    }
    for (index, record) in chain.iter().enumerate() {
        let expected_seq = index as u64;
        if record.sequence != expected_seq {
            return Err(ProvenanceError::SequenceGap {
                index,
                expected: expected_seq,
                found: record.sequence,
            });
        }
        let expected_prev = if index == 0 {
            GENESIS_PREV_HASH.to_string()
        } else {
            chain[index - 1].record_hash.clone()
        };
        if record.prev_hash != expected_prev {
            return Err(ProvenanceError::PrevHashMismatch { index });
        }
        let expected_hash = sha256_hex(&canonical_record_bytes(
            record.sequence,
            &record.recorded_at,
            &record.payload_id,
            &record.payload_hash,
            &record.prev_hash,
        ));
        if record.record_hash != expected_hash {
            return Err(ProvenanceError::RecordHashMismatch { index });
        }
    }
    Ok(())
}

/// Verify the chain and that each record's payload_hash matches provided bytes.
///
/// `payloads` must be aligned with `chain` (same length and order).
pub fn verify_chain_with_payloads(
    chain: &[ProvenanceRecord],
    payloads: &[&[u8]],
) -> Result<(), ProvenanceError> {
    verify_chain(chain)?;
    if payloads.len() != chain.len() {
        return Err(ProvenanceError::PayloadHashMismatch { index: 0 });
    }
    for (index, (record, payload)) in chain.iter().zip(payloads.iter()).enumerate() {
        if record.payload_hash != hash_payload(payload) {
            return Err(ProvenanceError::PayloadHashMismatch { index });
        }
    }
    Ok(())
}

/// Emit a RIOPA-compatible event stream from a validated provenance chain.
pub fn emit_riopa_event_stream(
    chain: &[ProvenanceRecord],
    context: &RiopaEmissionContext,
) -> Result<RiopaEventStream, ProvenanceError> {
    verify_chain(chain)?;
    let events = chain
        .iter()
        .map(|record| RiopaArtifactEvent {
            event_id: format!("riopa:{}:{}", record.sequence, record.record_hash),
            event_type: "capture.artifact.recorded".to_string(),
            recorded_at: record.recorded_at.clone(),
            source: context.source.clone(),
            input: RiopaIoRef {
                reference: context.input_ref.clone(),
                hash: record.payload_hash.clone(),
            },
            output: RiopaIoRef {
                reference: context.output_ref.clone(),
                hash: record.record_hash.clone(),
            },
            agent: context.agent.clone(),
            rights: context.rights.clone(),
            schema_refs: context.schema_refs.clone(),
            integrity: RiopaIntegrity {
                chain_algorithm: "sha256-prev-hash-v1".to_string(),
                prev_hash: record.prev_hash.clone(),
                payload_hash: record.payload_hash.clone(),
                record_hash: record.record_hash.clone(),
            },
            chain: RiopaChainLink {
                sequence: record.sequence,
                payload_id: record.payload_id.clone(),
            },
        })
        .collect();

    Ok(RiopaEventStream {
        profile: "riopa.provenance.capture.v1".to_string(),
        generated_at: context.generated_at.clone(),
        source: context.source.clone(),
        input_ref: context.input_ref.clone(),
        output_ref: context.output_ref.clone(),
        agent: context.agent.clone(),
        rights: context.rights.clone(),
        schema_refs: context.schema_refs.clone(),
        semantic_gaps: vec![
            "payload bytes are not embedded; only content hashes are emitted".to_string(),
            "no external timestamp authority attestation is captured".to_string(),
            "no stream-level cryptographic signature is currently emitted".to_string(),
        ],
        unsupported_fields: vec![
            "signature".to_string(),
            "witness_log".to_string(),
            "retention_policy_id".to_string(),
        ],
        events,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_payload_is_stable() {
        let h1 = hash_payload(b"hello archive");
        let h2 = hash_payload(b"hello archive");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
        assert_ne!(h1, hash_payload(b"hello Archive"));
    }

    #[test]
    fn append_and_verify_chain() {
        let mut chain = Vec::new();
        let r0 = append_record(&chain, "2026-07-01T00:00:00Z", "doc/a.pdf", b"payload-a");
        chain.push(r0);
        let r1 = append_record(&chain, "2026-07-02T00:00:00Z", "doc/b.pdf", b"payload-b");
        chain.push(r1);

        assert_eq!(chain[0].sequence, 0);
        assert_eq!(chain[0].prev_hash, GENESIS_PREV_HASH);
        assert_eq!(chain[1].prev_hash, chain[0].record_hash);
        assert!(verify_chain(&chain).is_ok());
        assert!(verify_chain_with_payloads(&chain, &[b"payload-a", b"payload-b"]).is_ok());
    }

    #[test]
    fn tampered_payload_hash_fails() {
        let chain = vec![append_record(
            &[],
            "2026-07-01T00:00:00Z",
            "doc/a.pdf",
            b"payload-a",
        )];
        assert!(verify_chain_with_payloads(&chain, &[b"payload-TAMPERED"]).is_err());
    }

    #[test]
    fn tampered_record_breaks_chain() {
        let mut chain = vec![append_record(
            &[],
            "2026-07-01T00:00:00Z",
            "doc/a.pdf",
            b"payload-a",
        )];
        chain.push(append_record(
            &chain,
            "2026-07-02T00:00:00Z",
            "doc/b.pdf",
            b"payload-b",
        ));
        chain[1].payload_id = "doc/evil.pdf".into();
        assert!(matches!(
            verify_chain(&chain),
            Err(ProvenanceError::RecordHashMismatch { index: 1 })
        ));
    }

    #[test]
    fn empty_chain_errors() {
        assert_eq!(verify_chain(&[]), Err(ProvenanceError::EmptyChain));
    }

    #[test]
    fn serde_roundtrip() {
        let chain = vec![append_record(&[], "t0", "id0", b"p0")];
        let json = serde_json::to_string(&chain).unwrap();
        let restored: Vec<ProvenanceRecord> = serde_json::from_str(&json).unwrap();
        assert_eq!(chain, restored);
        assert!(verify_chain(&restored).is_ok());
    }

    #[test]
    fn emit_riopa_stream_preserves_chain_integrity_fields() {
        let mut chain = Vec::new();
        chain.push(append_record(
            &chain,
            "2026-07-01T00:00:00Z",
            "capture/request-42.json",
            b"payload-a",
        ));
        chain.push(append_record(
            &chain,
            "2026-07-01T00:01:00Z",
            "capture/request-42-attachment-1.bin",
            b"payload-b",
        ));
        let context = RiopaEmissionContext {
            generated_at: "2026-07-01T00:02:00Z".to_string(),
            source: "fyi-cli://capture".to_string(),
            input_ref: "urn:fyi:capture-input:synthetic".to_string(),
            output_ref: "urn:fyi:capture-output:synthetic".to_string(),
            agent: "fyi-cli/0.1.0".to_string(),
            rights: "operator-authorized-read-only".to_string(),
            schema_refs: vec![
                "https://example.invalid/riopa/provenance/v1".to_string(),
                "https://github.com/edithatogo/fyi-cli/crates/fyi-core/src/provenance.rs"
                    .to_string(),
            ],
        };

        let stream = emit_riopa_event_stream(&chain, &context).unwrap();
        assert_eq!(stream.events.len(), 2);
        assert_eq!(stream.events[0].integrity.prev_hash, GENESIS_PREV_HASH);
        assert_eq!(
            stream.events[1].integrity.prev_hash,
            stream.events[0].integrity.record_hash
        );
        assert_eq!(
            stream.events[1].chain.payload_id,
            "capture/request-42-attachment-1.bin"
        );
    }
}
