# Source receipt coverage — 16 November 2026

This is a bounded source-boundary handoff for #309, #310 and #312. The audited
baseline is `047cc13156d8a9ae1cd9fd2d55678bd17a715f2c`, including PR #477.
The track and its archive-cutover gates remain open.

## Scope and ownership

- Chat `01a1e3da-6e02-7012-8e4a-1e4f2034d019` owns the sync receipt-integrity
  increment on branch `codex/fyi-sync-receipt-integrity-310` in managed
  worktree `/Users/doughnut/.codex/worktrees/fyi-receipt-integrity-310/fyi-cli`.
- Owned implementation paths: `crates/fyi-cli/src/main.rs`,
  `crates/fyi-core/src/acquisition_receipts.rs` and their scoped new tests.
  This note is the terminal evidence handoff. No native object, authentication,
  tenant or live source operation is assigned.
- The parallel MCP lane retains `crates/fyi-core/src/sync.rs`, MCP sources,
  shared handover and track files, repository integration, and the preserved
  Rust replay branch. The worktrees do not share a writer or build directory.

## Audited coverage at the baseline

| Surface | Receipt and evidence at `047cc131` | Remaining boundary |
| --- | --- | --- |
| Rust `fetch-request-page` | Optional receipt, raw response and canonical result are requested together. The guarded GET binds a single response's status, URL, bytes and digest; raw and result files are written before the receipt. | One request only; this is not full WARC/WACZ, redirect-chain, attachment, cross-runtime or downstream parity. |
| Rust `sync pull --receipt` | #477 constructs a sync receipt after the operation, but its builder always records an empty response list and zero requests, bytes and retries. Its canonical projection is hashed but not persisted. | The receipt cannot establish the transport operations that occurred. Failure can occur after database or network activity. |
| Rust `sync push --receipt` | The same builder records zero response evidence. A push report with failed items is still an `Ok` report and #477 labels the receipt succeeded. | Submission is a side effect; a receipt error after dispatch cannot undo it. Explicit live-write authorization remains required. |
| Rust remote MCP tools | Use the guarded `SyncClient` for remote operations; these tool calls do not expose a #310 acquisition-receipt contract. | Source-owned transport evidence, response and budget coverage remain open under the existing MCP and source tracks. |
| Rust MuckRock and FragDenStaat provider libraries | Their read-only `json_get` paths call a supplied `reqwest::Client` directly and do not emit acquisition receipts. | These are separate non-Alaveteli pilot surfaces; their boundary and consumer coverage need an explicit inventory before any claim of exclusive source-network ownership. |
| Python authority import, body discovery, request fetch, discovery, historical index and capture | The existing CLI offers optional receipts and passes a recorder into these source paths. | An optional flag or successful receipt alone does not prove complete raw-byte, attachment or archive linkage. |
| Python `source-probe` and `ingest-feed` | Both can access a source, but neither CLI command exposes a receipt. `source-probe` has a recorder-capable lower-level function; `ingest-feed` invokes `feedparser.parse`. | They remain gaps in the all-network-command inventory and require their own scoped remediation and fixtures. |
| Python `internet-archive-cdx` and `internet-archive-replay` | Their existing CLI routes require a receipt and use the acquisition recorder to observe responses; replay also requires a selection, result and checkpoint. | Rust replay implementation, independent old/new parity, authorized bounded hosted smoke and downstream shadow verification remain separate gates. |
| Rust CDX and Wayback replay | Neither command is present on the audited Rust CLI. A local replay branch exists and remains with its owner. | Do not infer Rust availability or archive cutover from the Python routes or the unmerged local branch. |

The receipt verifier checks structural and digest consistency. A schema-valid
empty response list by itself does not prove that a sync operation made no
requests, and a result digest does not stand in for captured response bytes.
The Rust verifier's conservative rejection of empty `alaveteli-sync` receipts
is stricter than the generic v1 JSON schema and the Python structural validator.
Cross-runtime semantic parity for this case remains an open, blocking follow-up;
the Python validator must not be treated as proof of observed sync transport.

## Bounded integrity increment

The intended correction is to reject ambiguous empty-response sync receipts and
stop receipt-requesting sync commands before database or source I/O until the
transport can provide real response evidence and a recoverable projection.
This is a fail-closed safety gate, not completion of #310 or #312. Existing
receipts must not be silently rewritten; consumers need to re-acquire evidence
under an authorized future workflow when it becomes available.

The source checkpoint is `16f77ca763b7d2f46246e389ce9482606046e133` on
`codex/fyi-sync-receipt-integrity-310`, based on `047cc131`. The CLI accepts the
existing option for compatibility but rejects `sync pull` or `sync push` with
`--receipt` before runtime, database or source operations. Without that option,
the prior sync path is retained. The legacy Rust sync builder now errors, and
the Rust verifier rejects the ambiguous #477 sync receipt shape; existing files
are never rewritten. This intentionally rejects even a genuinely zero-response
sync receipt because the old representation cannot distinguish that case from
missing observations.

## Local verification

The isolated worktree used Homebrew `rustc 1.100.0` and `cargo 1.100.0` on
macOS. All Cargo commands below used `--locked --offline` with
`CARGO_BUILD_JOBS=2`, `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0` and
`CARGO_PROFILE_TEST_DEBUG=0`.

- Red at the baseline: `cargo test -p fyi-core --test
  sync_receipt_integrity_tests` produced the expected two failures for the
  fabricated and accepted empty-response receipts; the observed-response
  control passed. The CLI gate test also failed before the correction because
  sync reached ordinary base-URL validation instead of the evidence preflight.
- Green: `cargo test -p fyi-core --test sync_receipt_integrity_tests --test
  acquisition_receipts` passed 18 tests. `cargo test -p fyi-cli --test
  sync_receipt_gate_tests --test receipt_wire_tests` passed 12 tests. `cargo
  test -p fyi-cli --bin fyi-cli test_parse_sync` passed five selected tests.
  The CLI tests use malformed URLs that cannot initiate a source request and
  check preservation of existing receipt bytes and absence of a new database.
- `cargo clippy -p fyi-core --lib --test acquisition_receipts --test
  sync_receipt_integrity_tests -- -D warnings` and `cargo clippy -p fyi-cli
  --bin fyi-cli --test sync_receipt_gate_tests --test receipt_wire_tests -- -D
  warnings` passed. `cargo fmt --all -- --check` and `git diff --check` passed.

These are targeted local results, not full-workspace, measured-coverage,
cross-platform or hosted acceptance. The repository coordinator must review
and integrate the exact receiving head, update the shared track and handover,
and retain all required CI/security/coverage gates. Independent transport-backed
receipt construction, cross-runtime semantics, release pinning,
raw/attachment/WARC linkage and `fyi-archive` shadow parity remain open. The
preserved Rust replay branch and its dependencies stay with the #312 owner.
