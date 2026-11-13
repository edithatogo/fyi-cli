# Plan

## Independent workstreams

- [ ] #310: define the acquisition-receipt schema and atomic writer, then add it
  to every existing network command.
- [~] #311: implement resumable Internet Archive CDX discovery with parity
  fixtures derived from the current `fyi-archive` behavior.
- [~] #312: implement bounded, allowlisted Wayback replay with byte receipts.
  Offline implementation and adversarial parity fixtures are present; hosted
  smoke, downstream old/new parity, pinned release integration, NZ shadow
  parity, and archive cutover remain open acceptance gates.
- [ ] #313: add source probing, shared capture pacing, `Retry-After`, and
  checkpointed numeric-ID discovery.
- [ ] #314: add narrowly allowlisted external historical-index retrieval.

These workstreams may proceed in parallel because they share only the receipt
contract. Merge #310 first or use its fixture branch as the temporary base.

## Integration

- [ ] Publish a pinned `fyi-cli` release/commit containing the adapters.
- [ ] Convert `fyi-archive` source-facing scripts into thin command wrappers.
- [ ] Add a static boundary test rejecting new source HTTP clients outside
  `fyi-cli` while allowing archive/publication provider APIs.
- [ ] Run offline output parity and hosted bounded smoke tests.
- [ ] Run NZ shadow parity for queues, captures, events, attachments, revisions,
  receipts, and checkpoints before removing legacy transports.

## 2026-11-13 Rust receipt verification tranche

- [x] Check in Python-origin v1 receipt fixture at `conformance/acquisition/receipt-v1-python.json` with a Python-side schema and digest test.
- [~] Implement Rust offline canonical JSON and receipt verifier at `crates/fyi-core/src/acquisition_receipts.rs` with cross-language golden, tampering, unsafe URL, total consistency and ordering invariants; hosted CI remains acceptance evidence, not yet complete.
- [ ] Expand Rust verification to full JSON Schema 2020-12 equivalence, including unknown-field rejection, numeric/UTC semantics, and Unicode/float/escaping cross-runtime corpus.
- [ ] Add fail-closed, durable Rust atomic receipt writer that never silently overwrites valid immutable evidence; add power-loss and concurrency tests.
- [ ] Integrate receipts with all Rust source-network commands, and pin client & downstream `fyi-archive` consumers after shadow parity; existing #310 remains **open**.
- [ ] Verify WARC/WACZ raw byte capture linkage and publish independently reproducible signed manifests.

The new Rust verifier is read-only and does not claim write-path or network acquisition parity. Keep #310 active and do not alter archive cutover gate.

### 2026-11-13 receipt command and compatibility status

- [~] A Rust read-only `verify-acquisition-receipt` CLI command has been added and is smoke-tested on Rust 1.99.1 in dedicated CI. The verifier checks Python v1 canonical self-digest, timestamps, URLs, retry/byte totals and shape invariants. This does **not** validate the captured response payload bytes or replace the Python acquisition writer. Full schema parity, atomic Rust writer, and source-network integration remain open.
- [x] Machine-readable Python/Rust command-name parity manifest and CI drift check added. Rust now exposes 44 top-level commands versus 46 Python; 34 names overlap, but meaningful behaviour is not asserted.
- [~] Python 3.15 security compatibility: Ruff and BasedPyright succeed; `pip-audit` crashes on removed `sre_constants` in vendored `pip-api`/pyparsing, and Atheris 3.1.0 provides cp312–cp314 wheels but no cp315. The isolated Python 3.12 `pip-audit` successfully audited the actual Python 3.15 installed set with **no known vulnerabilities** on run 43372190314 (audit=0); Atheris 3.12 remains enforced. Do not increase Python minimum until security parity or a documented independent scanner/fuzz replacement is implemented.

## Incremental implementation — immutable Rust writer (PR #405 merged, 2026-11-13)

- [~] #310 Rust `write_receipt_atomic` added to `fyi-core`: verifies the Python-compatible v1 receipt, writes private same-directory temporary bytes, calls `sync_all`, uses `hard_link` for no-overwrite atomic publish, permits only byte-identical idempotent repeats, rejects a divergent or corrupted pre-existing receipt, and cleans temporary evidence. On filesystems without hard-link support it fails closed instead of replacing evidence.
- [~] Rust verifier now rejects unknown fields in the receipt and nested typed structures, matching the v1 `additionalProperties:false` policy (full Schema 2020-12 parity remains open).
- [~] Added negative and concurrency tests for divergent valid receipts, malformed existing evidence, unknown schema properties, identical concurrent writes and temporary-file cleanup. Verified by PR #405 hosted Rust 1.99 CI, coverage, security audit and mutation-adjacent regression checks; merge commit `593fedb6`.
- [ ] Integrate writer with source-network Rust adapters and add offline input/output parity; the existing Python acquisition recorder and its tests remain authoritative until cutover.
- [ ] Cross-platform Windows publication/flush verification, filesystem fault injection, process crash/corruption recovery, real bytes WARC linkage and downstream `fyi-archive` NZ shadow parity.
- [ ] Sign release evidence and pin upstream/downstream client versions before archive cutover.

No existing #310–#314 issue is closed by this incremental Rust implementation.

## Result-byte verification tranche (PR #406 merged, 2026-11-13)

- [x] Rust `verify_receipt_result_bytes` independently checks stored canonical result projection length and SHA-256 after verifying the receipt's own integrity (PR #406, merge `5c5bf30d`).
- [x] `fyi-cli verify-acquisition-receipt <receipt> --result-file <path>` supports optional offline byte verification, retaining metadata-only compatibility. Golden `conformance/acquisition/result-v1.json`, negative tests and Rust 1.99.1 CLI smoke passed on PR #406 final-head CI.
- [ ] Add raw HTTP response/WARC/WACZ payload byte linkage and independent archival verification. Result projection bytes are **not** the raw network response and must not be described as equivalent.
- [ ] Full cross-language JSON canonicalization and schema fixture corpus, Rust network writer integration and downstream shadow parity remain open; do not close #310 or enable `fyi-archive` cutover.

Merged evidence: PR #403 (Rust verifier baseline and migration contracts), PR #405 (Rust immutable writer and tests). New work stays in this existing source-adapter track.

### Merged acceptance evidence (2026-11-13)

- PR #403 — architecture/Conductor contracts, Rust 1.99.1 and Python 3.15 compatibility diagnostics; merged.
- PR #405 — immutable Rust receipt writer, strict field validation, concurrency/corruption/symlink tests; merged commit `593fedb6b768526d8e45204f1b23171167826b33` after full hosted CI including security, coverage and MSRV.
- PR #406 — offline result projection byte-length/hash verification and CLI/fixture smoke; merged commit `5c5bf30d646540ec39835559100a7375e40b4688` after full hosted CI including security, coverage and MSRV.
- No corresponding automatic close-out of #310: remaining work is raw-response/WARC/WACZ byte linkage, Rust source-adapter writer integration, external release pinning, parity with `fyi-archive`, comprehensive fault/replay testing and signed provenance.

## Guarded Alaveteli network acquisition increment (PR pending, 13 Nov 2026)

- [~] Extend the existing `SyncClient::send_guarded` boundary with `fetch_request_evidence` for an explicit single-request read. It retains original HTTP entity body bytes, captures actual source status/final URL and a canonical typed projection, then produces a Python-compatible acquisition receipt v1. Reject cache-replayed 304 responses rather than misreporting them as fresh network evidence.
- [~] Add `build_single_get_receipt` and `verify_receipt_raw_bytes`: source and normalized result hashes are kept separate, and the receipt self-digest covers both. Tests require successful hash and length checks and rejection of modified raw/normalized bodies.
- [~] Share private no-clobber atomic publication for receipt and raw/normalized blobs. The user-invoked Rust `fetch-request-page` no longer only prints a placeholder: it uses the guarded client and supports explicit `--raw-file`, `--result-file` and `--receipt` together. Existing `verify-acquisition-receipt` adds optional `--raw-file` verification.
- [~] Run offline Rust MSRV 1.99.1 conformance with Python-origin `raw-v1.bin`, `result-v1.json`, `receipt-v1-python.json`, including tampering rejection. Dedicated mocked GET, HTTP 403 and cached 304 tests. Hosted PR checks are required before marking this increment verified.
- [ ] Extend original HTTP/WARC/WACZ linkage to archive headers, attachments, redirect chain and preservation format. The network entity bytes captured here are **not** the complete WARC response bytes.
- [ ] Migrate the other Rust source commands behind the same boundary and finish downstream fyi-archive pinned release/shadow parity; do not close #310 or enable archive cutover.
- [ ] Compare Python and Rust request endpoint, SQLite effects, output/error codes and reconcilers; Rust fetch-request-page is not yet functionally identical to the Python command.

See `docs/rust-alaveteli-receipt-acquisition.md`. Retain prior merged PR evidence and `archive_cutover_allowed=false`.
