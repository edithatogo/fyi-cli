# Guarded Rust Alaveteli acquisition with v1 receipts

Status: incremental read-only integration, 13 November 2026. Canonical Conductor owner: `.conductor/tracks/source-adapter-boundary-20260812/` (#309, #310). **Not a fyi-archive cutover contract.**

## Behaviour

The native Rust `fyi-cli fetch-request-page <id>` now uses `fyi-core::SyncClient::fetch_request_evidence` and the existing agent-aware guarded network client. It performs a single GET of `/api/v2/request/<id>.json`, validates a positive requested ID, the parsed request ID, the HTTP response status and the bounded source response. It produces both original network entity bytes and a normalized canonical JSON projection.

The receipt v1 stores **two distinct digests**:
- `responses[0].sha256`: SHA-256 of the exact returned HTTP entity bytes after HTTP transfer decoding, before JSON parsing
- `result.sha256`: SHA-256 of the canonical typed `AlaveteliRequest` JSON projection with a trailing LF

These bytes are **not a WARC HTTP exchange**; headers, redirect chain, original compressed wire encoding, attestation and signing are not yet captured. A 304 conditional response backed by local cache is rejected as fresh acquisition evidence.

The response URL stored in receipts has any username, password, fragment and query parameter values removed or redacted. Authentication headers and response bodies are never included in error messages. The existing network guardrails constrain request concurrency, rate limits, bytes and time.

## CLI

```bash
# Read-only guarded fetch (no files written)
fyi-cli fetch-request-page 42 --base-url https://fyi.org.nz

# Read-only fetch with three distinct, explicitly requested private files
fyi-cli fetch-request-page 42 --base-url https://fyi.org.nz \
  --raw-file evidence/42-response.bin \
  --result-file evidence/42-result.json \
  --receipt evidence/42-receipt.json

# Recheck without contacting a source
fyi-cli verify-acquisition-receipt evidence/42-receipt.json \
  --raw-file evidence/42-response.bin \
  --result-file evidence/42-result.json
```

Evidence paths must be supplied together or not at all. Writes create private same-directory temporary files, flush them and atomically publish with no clobber; same-byte repeats are idempotent, conflicting files and symlinks fail closed. The raw and normalized files are published **before** the receipt, to avoid leaving a falsely complete receipt after a failed blob write. They are not a three-file transaction: interrupted operations may leave orphan blobs for operator reconciliation.

The `--db` option remains syntactically accepted in Rust but this new read-only path does not mutate SQLite. Python's `fetch-request-page` has different request path, database side effects and output shape; this is **partial**, not demonstrated behavioural parity.

## Offline conformance and limitations

The Rust 1.99.1 CI checks raw and normalized content against Python-origin v1 fixtures and rejects deliberate mutations without any network traffic. Mocked `SyncClient` tests cover the outbound GET, response digest, normalized result, denied responses, malformed IDs and 304 cache refusal.

The following remain open under #309/#310–#314: complete Alaveteli/Python command semantic parity; integration of receipts into all Rust network commands and external indexes; repeatable fyi-archive old/new NZ shadow comparison; WARC/WACZ original HTTP and attachment byte linkage; cross-platform filesystem fault injection; legal/ethical upstream coordination; signed release/provenance manifests and archive cutover authorisation.

Do not treat this read-only single-request workflow as approval for unattended scraping, bulk retrieval, writes, or access to restricted data.
