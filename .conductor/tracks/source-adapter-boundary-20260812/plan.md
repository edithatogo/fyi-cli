# Plan

## Acquisition evidence integrity — 2026-11-17 (#310)

Receiving baseline: `1f0ae6d0b00663d52ccb65904152ca4e933b3486`.
No open pull requests were present at intake. The existing receipt, replay and
single-list sync pull implementations are retained and extended in this track.

- [x] Capture a fresh, bounded request response without automatic redirects,
  cache reuse or content decoding; persist complete HTTP/parse/identity failures
  as failed v1 receipts, and refuse to fabricate receipts for incomplete exchanges.
  [implementation checkpoint: a26298e5; PR #488]
- [x] Apply normalized, non-overlapping evidence/database/sidecar preflight to
  `fetch-request-page` before creating a pacing database or contacting a source.
  [implementation checkpoint: a26298e5; PR #488]
- [x] Bind incremental sync receipts to redacted before/after boundary snapshots
  and parsed numeric rate-limit observations, including partial application failures.
  [implementation checkpoint: a26298e5; PR #488]
- [x] Run socket-free red/green regressions, independent review, formatting,
  strict Clippy, workspace tests and the existing hosted acceptance checks.
  [source checkpoint: 90129b7; CI 44054377597; MSRV 44054377789;
  Linux/macOS/Windows portability 44054380472]
- [x] Record the exact implementation checkpoint and current network-command
  coverage without closing the wider #310 or archive-cutover gates prematurely.
  [source checkpoint: 90129b7; PR #488]

Evidence and limits: [acquisition-integrity-20261117.md](acquisition-integrity-20261117.md).

No live source request or write operation is authorised by this implementation.
Feed/push receipts, retained Python entrypoints, archive linkage and downstream
shadow parity remain separate unfinished acceptance work.

## Replay qualification follow-up — 2026-11-17

Baseline: `1f0ae6d`, including the Rust replay engine and transport from #481/#483.
This #312 increment retains the existing #309/#310 contract and owners.

- [x] Implement checkpoint-size validation before payload publication and atomic
  checkpoint replacement. The regression reproduced the defect on all three
  platforms; the correction passed independent source review. [checkpoint: d7b577e]
- [x] Disable the Wayback HTTP client's implicit protocol retries. The socket-free
  regression observed three attempts instead of one before the correction;
  independent source review accepted the scoped fix. [checkpoint: d7b577e]
- [x] Execute the frozen replay corpus against the Python reference, retaining
  the two documented legacy divergences; 67 focused cases passed locally, and
  the full Python suite passed on tests-first head `7805d42`.
- [x] Protect replay fixture bytes across checkouts and add focused Rust replay
  and CLI evidence cases to the existing Linux/macOS/Windows portability gate.
- [x] Reconcile the replay guides and record source review and tests-first
  evidence in [replay qualification](./replay-qualification-20261117.md).
  Implementation PR: [#489](https://github.com/edithatogo/fyi-cli/pull/489).
- [x] Verify final PR head `e1e0a6a` through hosted CI and Linux, macOS and
  Windows receipt portability. PR #489 merged into `master`.
  [checkpoint: `428ad00`] [Final CI](https://github.com/edithatogo/fyi-cli/actions/runs/44389636962) · [Portability run](https://github.com/edithatogo/fyi-cli/actions/runs/44389636938)

Live source smoke, release pinning, downstream shadow parity and archive cutover
remain separate gates; `archive_cutover_allowed=false`.

## Bounded Rust replay work — 2026-11-16

- [x] Develop #312 replay from the frozen source checkpoint `5885469` on branch
  `codex/fyi-wayback-replay-312`. No live source or authentication scope was
  assigned. The replay engine merged through PR #481.
- [x] Reuse the frozen approval, bounded replay and offline recovery contracts;
  establish their dependencies and perform receiving-head preflight before
  integrating source files. Twelve focused offline replay tests pass, including
  the frozen synthetic oracle, input-size bounds and an object-name preflight.
  The independently
  owned #310 receipt-integrity work has priority
  over shared receipt and CLI paths; replay must consume its pinned contract.
- [x] Integrate the guarded replay transport, response receipts and Rust CLI
  after receipt-integrity integration. [PR #483; merge checkpoint: 71bf805]
- [x] Develop the transport increment on branch `codex/fyi-replay-transport-312`,
  based on PR #481. Its scope covered Rust replay, transport and CLI code,
  focused tests and the CLI parity manifest. No live source or authentication
  scope was assigned. The bounded increment merged through PR #483.
- [x] Implement the bounded transport, CLI, ordered response-body evidence and
  v1 replay receipts, with fail-closed handling for incomplete exchanges.
  Focused offline tests cover redirects, compressed entities, response and
  result tampering, path collisions, checkpoints and runtime limits. [source
  checkpoint: e6e1052]
- [x] Develop the bounded #310 sync pull receipt follow-up on branch
  `codex/fyi-sync-pull-receipt-310`, initially based on PR #483. Its scope was
  Rust sync pull evidence, CLI preflight and offline tests. No live source or
  authentication scope was assigned. The increment merged through PR #484 after
  its protected checks and MSRV check passed.
- [x] Implement a fresh, redirect-free single-list sync pull with immutable raw,
  result and receipt files. Offline tests cover a successful pull, HTTP failure,
  redirect, incomplete response and evidence-path preflight. Feed and push
  receipts remain gated. [source checkpoint: 868a334]
- [x] Verify offline source/CLI integration and saved response receipts across
  Linux, macOS and Windows. [checkpoint: `428ad00`, PR #489]
- [ ] Verify the pinned downstream adapter, separately authorized hosted smoke
  and NZ shadow parity before closing #312. See Integration below for the
  release and cutover gates.

## Independent workstreams

- [~] #310: extend the existing versioned receipt schema and atomic writer to
  every active network command.
- [~] #311: implement resumable Internet Archive CDX discovery with parity
  fixtures derived from the current `fyi-archive` behavior.
- [~] #312: implement bounded, allowlisted Wayback replay with byte receipts.
  Offline implementation and adversarial parity fixtures are present; hosted
  smoke, downstream old/new parity, pinned release integration, NZ shadow
  parity, and archive cutover remain open acceptance gates.
- [x] #313: add source probing, shared capture pacing, `Retry-After`, and
  checkpointed numeric-ID discovery. [PR #410]
- [x] #314: add narrowly allowlisted external historical-index retrieval.
  [PR #410]

These workstreams may proceed in parallel because they share only the receipt
contract. Merge #310 first or use its fixture branch as the temporary base.

## Offline Wayback archive handoff

- [x] Replay one synthetic fixture in process, verify its receipt with the
  pinned Rust CLI, resume the completed checkpoint with zero requests and
  verify the resulting `fyi-archive` package and its indexes.
  [Source checkpoints: `c94f3ed` and `0691a9a`]
- [x] Verify that the packaged WACZ preserves the fixture's encoded response
  and binds its indexed WARC record to the original receipt. Retain the
  same-length byte-change and incorrect-record-ID rejection results.
  [Source checkpoints: `c94f3ed` and `0691a9a`]

Both runs used the same fixture with network access denied.
[Offline handoff evidence](replay-qualification-20261117.md#offline-wayback-archive-handoff)

## Integration

`archive_cutover_allowed=false`. The following gates remain open.

- [ ] Complete #310 receipt coverage for all active source commands, including
  feed and push paths and retained legacy entrypoints. Verify production
  response, redirect and attachment evidence through WARC/WACZ capture and
  downstream retention.
- [ ] Qualify and pin a deployable Rust `fyi-cli` release in `fyi-archive`.
  Convert remaining source-facing scripts to thin wrappers and enforce the
  source HTTP boundary with static and behavioral checks that allow approved
  archive and publication APIs.
- [ ] Verify old and new downstream outputs across the offline parity fixtures.
  The one synthetic snapshot provides an input for that comparison.
- [ ] Record authorized, bounded hosted smoke for live Alaveteli discovery
  and capture and one Internet Archive CDX/replay record.
- [ ] Verify NZ shadow parity for queues, captures, events, attachments,
  revisions, receipts and checkpoints. Qualify rollback before removing
  legacy transports. [Downstream parity gate](https://github.com/edithatogo/fyi-archive/issues/378)

## 2026-11-13 Rust receipt verification tranche

- [x] Check in Python-origin v1 receipt fixture at `conformance/acquisition/receipt-v1-python.json` with a Python-side schema and digest test.
- [~] Implement Rust offline canonical JSON and receipt verifier at `crates/fyi-core/src/acquisition_receipts.rs` with cross-language golden, tampering, unsafe URL, total consistency and ordering invariants; hosted CI remains acceptance evidence, not yet complete.
- [ ] Expand Rust verification to full JSON Schema 2020-12 equivalence, including unknown-field rejection, numeric/UTC semantics, and Unicode/float/escaping cross-runtime corpus.
- [ ] Add fail-closed, durable Rust atomic receipt writer that never silently overwrites valid immutable evidence; add power-loss and concurrency tests.
- [ ] Integrate receipts with all Rust source-network commands, and pin client & downstream `fyi-archive` consumers after shadow parity; existing #310 remains **open**.
- [ ] Verify production WARC/WACZ raw-byte capture linkage and publish independently reproducible signed manifests.

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

## Socket-free source regressions — 2026-11-16

- [x] Develop socket-free replay transport and typed source failure fixtures,
  test-only dependencies and evidence documentation on branch
  `codex/fyi-replay-socket-free-tests`. No native or authentication scope was
  assigned. The follow-up merged through PR #485.
- [x] Replace the new replay/source socket fixtures with in-process responses
  and a stalled connector; preserve the typed 503/401 and timeout regressions.
  Focused tests also passed with network access explicitly denied. [source
  checkpoint: 75d5fd1]
- [x] Preserve the Python SDK 503-versus-401 acceptance test through the real
  MCP stdio dispatcher and source response path using an explicitly selected,
  in-process transport fixture. The production entrypoint cannot select that
  fixture. The SDK and guardrail-boundary regressions passed with network
  access denied. [source checkpoint: 2cd7b1b]

## Result-byte verification tranche (PR #406 merged, 2026-11-13)

- [x] Rust `verify_receipt_result_bytes` independently checks stored canonical result projection length and SHA-256 after verifying the receipt's own integrity (PR #406, merge `5c5bf30d`).
- [x] `fyi-cli verify-acquisition-receipt <receipt> --result-file <path>` supports optional offline byte verification, retaining metadata-only compatibility. Golden `conformance/acquisition/result-v1.json`, negative tests and Rust 1.99.1 CLI smoke passed on PR #406 final-head CI.
- [ ] Extend the verified fixture linkage to production raw HTTP responses and WARC/WACZ payloads, with independent archival verification.
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

### Network redirect boundary hardening — 13 November 2026
- [~] The shared Rust agent HTTP client now refuses cross-origin or credential-bearing redirect targets and caps same-origin redirect hops; mocked tests cover a refused cross-port redirect and an accepted relative same-origin redirect. This is part of the source-adapter threat boundary, not a substitute for DNS rebinding protection or authorised upstream policies.
- [ ] Validate this change against real but bounded approved provider smoke fixtures and retain error/deny evidence without storing response bodies in logs.

### Parent/child GitHub issue state reconciliation (2026-11-13)

- `fyi-cli` #311 (CDX discovery adapter) is **closed** as of 2026-08-13. That represents issue-level implementation status, not evidence that Rust has replaced its Python command or that `fyi-archive` has passed operational shadow parity.
- `fyi-archive` #370 (generic instance acquisition controller) is **closed** but still describes the source ownership split and downstream shadow-parity acceptance. A closed parent issue is not an archive-cutover authorisation.
- `fyi-cli` #309, #310, #312, #313 and #314 remain **open**. Do not relabel those as complete or set `archive_cutover_allowed=true` without verified end-to-end evidence.

## Python receipt immutability hardening — #310

- [~] Align Python `write_receipt_atomic` with the merged Rust no-clobber evidence contract: private same-directory temporary file, fsync, hard-link publication, byte-identical idempotency, immutable different receipts, and symlink rejection.
- [~] Add Python regression tests for idempotency, non-replacement and symlink safety. Hosted PR checks are required for verification.
- [ ] Audit all network entry points and add missing `--receipt` support, including Rust adapters, before closing #310.
- [ ] Finish full cross-language JSON Schema parity, crash/Windows filesystem verification, raw response/WARC linkage and downstream shadow parity. Do not claim archival cutover on the basis of this hardening PR.

## Rust shared pacing increment — #313

- [x] Add the Python-compatible SQLite reservation and status contract to
  `fyi-core`, with an immediate transaction for state and event writes, rollback
  on event failures and a frozen Python 3.15.1 conformance fixture. [checkpoint: 7dc6ef9]
- [x] Integrate optional shared pacing and HTTP 429/503 `Retry-After` feedback
  into the guarded Rust client and record limiter configuration in receipts. [checkpoint: 7dc6ef9]
- [x] Add read-only `rate-limit-status` and explicit `fetch-request-page` pacing
  options. Validate command inventory, CLI errors and output-file preservation. [checkpoint: 7dc6ef9]
- Local Rust 1.99.1 verification passed 331 tests across 15 suites, all-target and
  all-feature workspace Clippy, formatting and the required repository contract
  checks. Hosted PR verification follows this source checkpoint.
- All 12 required hosted checks passed at `b50bbec`. After incorporating main's
  receipt scalar fixes from #420, source checkpoint `60e240c` passed 333 Rust
  tests and 42 focused Python 3.15.1 schema/repository contract tests. Updated
  hosted checks remain required for final integration.
- [ ] Finish Rust probing/capture, queued-reservation backoff handling, downstream
  archive parity and the remaining source acceptance evidence before retiring
  the Python source adapters.

## Execution sequencing and acceptance clarification (2026-11-14)

Canonical ordered roadmap: [docs/implementation-execution-order-20261114.md](../../../docs/implementation-execution-order-20261114.md). This is the next implementation track, **not** a new parallel track. Begin by verifying the latest-head checks and merge/prune status for PR #430. Then deliver #310 and #312 in incremental, reproducibly verified feature PRs; reuse the existing #311/#313/#314 evidence only where it proves current Rust behaviour. Keep source-network inventory, cross-runtime receipt schema/canonicalization, immutable publication, raw entity/attachment/WARC linkage, fail-closed redirect and SSRF controls, pacing, bounded replay, independent-process fault tests, and Linux/macOS/Windows acceptance evidence in scope. Produce a command-to-contract-to-consumer ledger with exact SHA and CI evidence.

Do not set `archive_cutover_allowed=true` or remove Python network transports as part of this track; release-pinned `fyi-archive` shadow parity and rollback belong to the subsequent cutover milestone. Reconcile Conductor and issues against evidence, not unchecked checklist claims.
