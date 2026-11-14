# Acceptance evidence

## Reconciled baseline (2026-11-14)

Commit `a816dcd` had a pending plan and metadata despite the registry's
implemented label. Its Python contract sensor checked a hand-maintained
inventory without exercising registration, schemas or transports.

Reproduced with Rust 1.99.1: four executable stdio regression tests failed.
The unmodified official Python MCP SDK 1.29.0 failed all four initial client
scenarios, including protocol negotiation and HTTP Origin rejection. These
were local synthetic tests, not live service calls.

## Slice 1: wire contract

Implements supported-version negotiation, ping, strict JSON-RPC envelopes,
silent notifications, bounded legacy batches, private parse/dispatch errors,
authentication before HTTP parsing, browser-origin rejection, HTTP body bounds,
and bounded stdio framing. Existing application handlers and source transports
are unchanged.

Run `cargo test -p fyi-mcp --locked` for the unit and executable sensors.
The PR records final local and hosted-head evidence; no unobserved check is
marked complete here.

PR #438 implements the wire slice at head
`71ec8aa093b9f05265d737e70c46964eb38b1bbd`; 44 local tests passed with
Rust 1.99.1, and Clippy, formatting and the existing verifiers passed. All required final-head checks passed before squash merge
`19f3cf376101500036951f9a3d3733da49f0df8b`. The optional self-hosted
AI security run was queued and is not claimed as passing.

## Slice 2: confirmation and operation budgets

The original confirmation gate failed four regressions for token opacity,
duplicate preparation, changed payloads and bounded capacity. The original
policy failed oversized-response accounting. Additional review regressions
showed invalid confirmations consuming the final operation allowance and
completed replay failing after budget exhaustion.

The repaired gate uses OS-generated opaque tokens, monotonic expiry, immutable
scope/payload binding, atomic admission and bounded retained state. Policy
permits enforce per-instance concurrency, count attempts before I/O, account
rejected response bytes, enforce duration and release on cancellation. Synthetic
future tests cover success, private upstream failure, timeout, oversized output,
policy changes, invalid confirmation and changed commit payload. No live write
is part of this verification. Rust 1.99.1 verification passed 98 MCP tests
(93 unit/boundary and five executable), Clippy with warnings denied, formatting,
and the Conductor contract check.

Review additionally identified invalid read parameters consuming remote allowance
and opening the shared upstream circuit before I/O. Query, ID and filter
validation now precedes admission; repeated invalid calls preserve zero usage
and a closed circuit.

PR #441 passed all 12 required checks on final head
`bc88340807a475170dea49222090e63adcd4f33a` while current with master:
[CI](https://github.com/edithatogo/fyi-cli/actions/runs/43558786976) and
[CodeQL](https://github.com/edithatogo/fyi-cli/actions/runs/43558786984).
It was squash-merged as `6c89bcdc11b0b5e70b7e840c4d6dfc8aae6998b1`.
The final base refreshes preserved all nine owned files byte for byte;
the source implementation was not changed to accommodate concurrent workstreams.

Attachment tests reject traversal and observed in-root file/directory symlinks.
Process restart durability, attachment byte immutability and source raw-byte/
retry/fault contracts remain separate unverified dependencies.

## Slice 3: executable schemas, current exchanges and official clients (PR #443)

The catalogue now exposes all 23 complete tool definitions. Remote writes have
mutating annotations and separate prepare/commit input branches. Output
schemas reflect the source model shapes; `sync_status`'s
previously unsatisfiable output schema is repaired. Argument validation occurs
before dispatch and rejects unsupported schema assertions, unknown fields,
wrong types and excessive bounds without reflecting rejected values.

The existing verifier retains historical v1 and validates current v2 exchanges
plus the catalogue captured from the executable. Rust equality and an actual
SDK `tools/list` comparison fail on drift. Twenty-three v2 cases include eight
executed handler negatives and clearly labelled synthetic success/error outputs.
No synthetic remote success example is represented as a live source call.

Review also identified semantic read errors consuming allowance and tripping
the upstream circuit before I/O. Read query/ID/filter checks now precede
admission; repeated invalid inputs preserve zero request/byte usage and a
closed circuit. The SDK verifies valid preparation remains available after
three whitespace-only searches. The admission fix is included in PR #441; PR #443 verifies it through
the real SDK and completes the phase-specific schema review finding.

The official Python MCP SDK exercises stdio and Streamable HTTP using default,
2025-06-18 and 2025-03-26 initialization. It checks local results, output schemas,
private errors, capability denial, read-only registration, all three governed
write preparations, mismatched confirmations and unchanged state after invalid
calls. Twenty deterministic Hypothesis examples send malformed arguments through
that real client. Inputs never contact a live authority.

Local verification on Rust 1.99.1 passed 127 MCP tests (122 unit/boundary and
five executable), 129 verifier tests and 13 SDK integration tests. Clippy with
warnings denied, Rust formatting, Ruff and the existing Conductor/verifier
commands passed. All 13 SDK tests passed with both the installed 1.29.0 client
and the lock-pinned 1.30.0 client; the latter also passed all 20 generated
examples. The SDK test initially caught a stale catalogue after a new
scope bound; recapture from the executable repaired that demonstrated drift.
The existing Python Tests job collects the SDK suite; missing Cargo and a
missing binary now fail on GitHub Actions rather than silently skipping it.
The separate Layered Harness Contract job only validates the fixtures.

The preliminary hosted Python run at PR #443 head
`be1fe5a4421e88ea825479e01db6a3074e14f8fb` installed the locked MCP SDK 1.30.0
and executed all 13 SDK cases successfully. The complete Python suite reported
1,449 passed and two unrelated skips in 195.97 seconds:
[CI run and retained Python evidence](https://github.com/edithatogo/fyi-cli/actions/runs/43558901343).
The final base refresh changes only this evidence and its timestamp; required
checks are still verified separately on that final head before merge.

An empty-database persistent HTTP assessment (ten warmups, 100 measured calls)
recorded p50 3.612 ms, p95 4.154 ms, maximum 6.007 ms, final/peak RSS 20,291,584
bytes and post-warmup growth 36,864 bytes. Enforced local fixture limits are
250 ms p95, 128 MiB peak RSS and 16 MiB growth. The measured debug executable,
SDK 1.30.0/Python 3.12.14/Linux environment and reproduction are documented in
`docs/mcp-contract-harness.md`. These are not remote or production-load claims.

## Mutation evidence

At PR #441 implementation head `2b4edb2eb060c31a24f9ebe65cc30bcde18ddc8e`,
`cargo-mutants 27.1.0` ran 40 selected security-predicate mutations against an
isolated snapshot/cache: 39 caught, zero missed, zero timed out, one unviable
Duration-division mutation. The unmutated baseline passed 51 policy/write-gate
tests. Source restoration was verified. The target selection and bounded
reproduction command are retained in the MCP contract document; no broad
workspace mutation score is inferred.

## Existing source tests reused

Twenty-two existing `fyi-core` tests passed (1.88 seconds test time) using local
mock servers. This verifies current JSON parsing, feed-link extraction, basic
SSRF/redirect guards, small response/count bounds, conditional caching,
pacing timeout, retry final state, state preflight and status-error privacy.
No source file, fixture or shared harness was changed.

```bash
cargo test -p fyi-core --lib -- --exact \
  sync::tests::contract_fixtures_match_sync_parser_expectations \
  sync::tests::pull_incremental_accepts_unexpected_optional_fields \
  sync::tests::pull_incremental_rejects_malformed_json_without_local_changes \
  sync::tests::pull_incremental_rejects_missing_required_fields_without_local_changes \
  sync::tests::extracts_request_ids_from_rss_and_atom_links \
  sync::tests::pull_feed_fetches_watched_request_records \
  sync::tests::search_request_ids_from_feed_propagates_query_and_deduplicates_ids \
  sync::tests::validate_instance_url_rejects_private_targets_and_credentials \
  agent_runtime::tests::guarded_default_http_client_never_follows_cross_origin_redirects \
  agent_runtime::tests::guarded_default_http_client_allows_same_origin_relative_redirects \
  sync::tests::response_byte_guardrail_blocks_the_next_remote_call \
  sync::tests::request_count_guardrail_blocks_the_next_remote_call \
  sync::tests::conditional_get_reuses_cached_body_after_not_modified \
  sync::tests::validator_cache_is_not_replaced_by_error_response \
  sync::tests::shared_pacing_blocks_a_wait_beyond_the_runtime_budget \
  sync::tests::push_dirty_retries_and_marks_failed_after_exhaustion \
  sync::tests::push_dirty_records_nonretryable_auth_failure_without_secret_body \
  sync::tests::push_dirty_keeps_dirty_request_when_server_error_retries_exhaust \
  sync::tests::update_request_state_rejects_unknown_state_before_network_call \
  sync::tests::update_request_state_if_current_puts_only_after_matching_fetch \
  sync::tests::update_request_state_if_current_rejects_stale_state_before_put \
  sync::tests::api_status_error_classifies_contract_failures_without_body_content \
  --test-threads=4
```

## Original acceptance reconciliation

| Original requirement | Evidence now available | Exact remaining gate |
| --- | --- | --- |
| Versioned JSON/RSS/Atom/error fixtures | MCP historical v1, current v2 and complete tool snapshot; existing source JSON/feed tests | Source fixtures lack a shared version manifest; RSS/Atom samples remain inline, without versioned malformed-feed cases |
| Every remote schema, annotation, result and error | All eight remote tools represented; actual catalogue, handler negatives and SDK write preparations verified | Full successful MCP-to-source read/commit exchanges across source variants are not established by synthetic output examples |
| Disabled/read/degraded/write stdio modes | Actual disabled/read-enabled/write-gated sessions; policy-level degraded/kill tests | Degraded mode is not exposed through a tested subprocess control path; no external degraded-stdio run |
| Property/mutation boundaries | Generated invalid SDK arguments; exact integer/depth/schema bounds; deterministic scope/clock/replay; 39 caught selected mutants; reused state/SSRF tests | Broader generated pagination/state/URL/path properties and SSRF/source-guardrail/error-classification mutations |
| Latency/memory/request/byte/concurrency | Recorded local SDK latency/RSS budgets; admitted-attempt, payload-byte, duration and concurrency tests; existing source small-body/count tests | Raw multi-chunk cancellation/peak memory, exact retry wire counts and propagation of MCP allowances into source budgets remain separate evidence gaps |
| Compatibility/release matrix | Linux Rust 1.99.1 and official SDK 1.29.0/1.30.0 stdio/HTTP; existing stable/MSRV jobs; explicit matrix | Windows/macOS runtime/client combinations have not been executed here; locked SDK results depend on final-head CI |
| Deterministic faults | MCP pending-future timeout, clock skew, expiry, circuit cooldown/recovery; source malformed JSON and pacing-wait tests | Stalled HTTP headers/body, mid-body disconnect, malformed transfer chunks and corrupt validator/filesystem cache injection |
| Version/deprecation handling | Unknown contract versions and schema assertions fail closed; HTTP unknown version rejected; initialization fallback and legacy batch behavior verified; neither supported profile is currently designated deprecated and historical v1 is regression-only | Profile-withdrawal/contract-migration behavior has not been exercised; later MCP revisions are outside the advertised supported profiles |
| Complete issue/epic evidence and closeout | Scoped PRs link #173/#169; this evidence corrects the historical implemented label | Keep the track open until the remaining original acceptance rows are resolved and final-head checks are recorded |

## Closure decision and ownership

**Not archived: original acceptance is incomplete.** Missing source fault,
fixture and platform evidence is not a claim that those paths are defective.
Source acquisition/transport and shared CI/platform owners must supply their
missing acceptance evidence; this workstream must not change their components.
Issue #173 and epic #169 were reopened with the original acceptance criteria
retained and the missing evidence identified. Other child tracks were not changed.

Additional product limits remain explicit: confirmation state is process-local,
attachment bytes are reopened by the source API after path checks, and remote
state updates use client GET-then-PUT preflight rather than a demonstrated
atomic server condition. Durable restart replay, attachment immutability/race
resistance and atomic remote compare-and-set are not verified guarantees.
Comprehensive production write-audit emission is likewise not established by
the reusable audit serializer tests. Existing local update/import semantics
are documented accurately, without changing the local-management workstream.

Required hosted checks are reviewed on each final head before merge. Parallel
TUI, packaging and quality-infrastructure merges advanced master, requiring
refreshed heads and new required checks for PR #441. That slice is now merged;
the schema/client slice retains its separate final-head gate.
Final PR heads, Actions links and merge commits are recorded in the PRs and
the delivery report. Source acquisition, SQLite core, localization, CLI
dispatch, TUI, packaging and shared workflows remain with their assigned owners.
