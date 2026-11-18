# Acceptance evidence

## Reproducible harness increment (2026-11-17)

### Verified implementation checkpoint

The implementation is verified at source head
`cfa72e7e47458e2656142a8ab00a39b31b81f4bd`, tested merge
`5e3629d02cc13e5954b095de6e6126454d6894a6`, with identical tree
`64701a568c5e30ad78e31ab7cb9331aa6af8ebc5`.
[Compatibility/mutation run 44061120141](https://github.com/edithatogo/fyi-cli/actions/runs/44061120141)
contains these actual, independently inspected results:

| Gate | Observed result |
| --- | --- |
| Linux/Windows/macOS × Rust 1.100.0/1.99.1 | All six cells passed; each ran six wire, three journal and exactly 16 official SDK cases without skips, failures or filtered tests |
| Linux source/fixture/property/privacy suite | 63 passed, zero skipped: 54 corpus cases including 12 source replays, five generated properties and four privacy negative controls |
| Generated source scenarios | 66 generated examples passed, plus four explicit examples; source-suite execution took 30.40 seconds |
| Targeted security mutations | 11 exact baselines passed; 15 caught; zero survivors, unviable edits or timeouts |
| Mutation evidence retention | All 26 log hashes, original source-file hashes, exact selectors and single-test outcomes verified |

All six compatibility artifacts identify the recorded source/merge, clean scoped
inputs, locked MCP SDK 1.30.0 and matching retained binaries. Ubuntu used CPython
3.12.16; Windows/macOS used 3.12.10. Architectures were Linux x86_64, Windows
x86_64 MSVC and macOS aarch64. These receipts qualify the documented stdio and
local journal scope; the actual source TLS replay remains Linux-specific.

The successful mutation artifact is `14317928476`, ZIP SHA-256
`5b5990141d8c6af06bae0a4c51b9a6f190e9cc8e795a554ecf9c9df50948885f`.
Its committed manifest blob is `880fa2c9e5cc153e8543bcfd00db0a72918426d8`,
SHA-256 `e5669e6939d51818fa667d2fb1cce3916e8bcc065a64e993194d93c51581acb7`.
The degraded mutation fails first at its write-denial assertion; the retry
mutation fails first at its declared uncertain-write assertion. The earlier
failed report below is preserved; this is a fresh complete execution.

[Repository CI at this checkpoint](https://github.com/edithatogo/fyi-cli/actions/runs/44061120028)
passed 1,808 Python tests with only two opt-in live-site skips. All 22 SDK, 38
upstream, 54 corpus, five generated, four privacy-sensor and 28 mutation-runner
cases ran without skips or failures. Current Rust and the separate MSRV job each
passed 811 workspace tests with one existing ignored subprocess entry point.
Configured line coverage passed at 93.83% (17,648 of 18,809 lines), retaining the
90% gate. The Python evidence artifact is `14319306124`, ZIP SHA-256
`ab32ef9c319739caa90b5b44a2cddcdee48dcaa868d985de919a2d122a73c743`.

This completes the reproducible harness increment. Required checks for the
final publication head are retained on [PR #493](https://github.com/edithatogo/fyi-cli/pull/493).
The broader issue and track remain open for the source/cache/raw-transfer,
attachment/state, platform qualification and genuine migration gates below.

### Implementation and earlier candidate evidence

Starting source is `1f0ae6d0b00663d52ccb65904152ca4e933b3486`.
[PR #493](https://github.com/edithatogo/fyi-cli/pull/493) implements the existing
#173 assignment with the following independently reviewable sensors:

- Shared v1 corpus: 17 synthetic JSON/RSS/Atom/error cases, exact file hashes,
  strict schemas, private verifier failures and negative controls. Existing
  root fixtures are reused unchanged. Twelve JSON/error variants have actual
  SDK/source replay cases using the existing non-forwarding Linux TLS fixture.
- Five generated source-contract families: result pagination/order/query
  encoding across collection variants, matched/stale state preconditions and
  immutable confirmation scope/replay, confined path normalization with exact
  multipart bytes, invalid path rejection before I/O, and source-error privacy.
  Negative controls cover escaped strings in responses and diagnostic logs.
- Current-source mutation manifest: 15 exact targets and 11 distinct baseline
  selectors across capability, confirmation scope/expiry/replay, SSRF, source
  and MCP guardrails, typed errors/retryability and nested redaction. The runner
  exports committed HEAD and its manifest, uses offline/locked Cargo and one
  build job, cancels output/time overruns, retains bounded per-test logs, and
  fails on survivors, unviable edits, missing selectors or stale source.
- Six explicit platform/compiler cells: Ubuntu 24.04, Windows 2025 and macOS 15
  against repository-pinned current Rust 1.100.0 and MSRV 1.99.1. Each cell uses
  an additional CPython 3.12 / lock-pinned MCP client environment, verifies the
  Rust wire/journal targets and exactly 16 portable stdio SDK cases without
  skips, and retains compiler/source/client/executable identities. The existing
  required Python 3.15.2 Linux suite remains separate and unchanged.

### Verification at implementation time

The new runner imports and missing shared manifest produced the intended RED
failures before implementation. Fixture and runner negative controls pass on
Linux CPython 3.12.14 with the frozen MCP SDK 1.30.0 test environment. The
standalone shared corpus verifies 17 cases. The combined existing contract and
new harness regression run passed **214 tests**, with the 12 new source replays
deselected pending a compiled binary (4.42 seconds). Ruff, formatting, Rustfmt,
the existing remote-MCP verifier and the Conductor verifier pass. The 15 mutation
source selectors validate, without claiming mutation execution. Runtime receipts
remain pending the candidate execution.

Independent review reproduced a real Windows-checkout defect: enabling Git's
`core.autocrlf` changed the RSS fixture hash. The shared corpus now has an explicit
`-text` attribute. The same checkout-filter command preserves all 17 manifest
hashes after correction; no fixture bytes or hashes were replaced. Local and
hosted Actionlint both passed the new workflow.

Review also narrowed the write-retry mutation to remove only the `read_only`
guard and the degraded mutation to disable only degraded write rejection.
Reads remain unchanged, so the selected tests must reach their write-denial
assertions. The earlier broader replacements failed read assertions and do not
establish the narrower write-denial evidence. Runtime verification of the
corrected manifest and final candidate head is recorded on PR #493.

### First hosted candidate and corrections

At source head `cebb3bf6fd5d5dabe206633d26ffce570c6aca0b`, tested merge
`2e7f94f49d7baa1a54b1cfe0431472ecc0fef96d` has the same source tree
`0dadbb9b069230f5606af3a829f224351105d449`. The
[first compatibility run](https://github.com/edithatogo/fyi-cli/actions/runs/44054213740)
executed all six platform/compiler stdio cells: each passed six wire tests,
three journal tests and exactly 16 SDK cases without skips. All retained binary
hashes match their receipts. All 11 mutation baselines and 15 original mutations
executed; the two refinements above require fresh execution, and the other 13
failed at the intended assertions.

The Linux source replay passed 62 tests, including all 12 corpus replays, and
failed the generated privacy case because its fixture expected `/version.json`
instead of the existing production `/api/v2/version.json`. The fixture path is
corrected. Full Python CI independently found that mismatch and a conditional
hardening step disallowed by the repository's workflow contract. Hardening now
uses the same unconditional first step as the existing portability workflow;
all 133 workflow-hardening tests and local Actionlint pass after correction.
The first candidate's full Rust workspace passed 811 tests with one ignored;
configured line coverage passed at 93.83%. Full Python reported 1,797 passed,
two failed and two unrelated skips; it is not counted as a passing candidate.

Using the retained Linux/current binary, whose unchanged production source and
SHA-256 were verified, the corrected five generated families passed 66 generated
examples plus four explicit examples in 49.97 seconds. An earlier local privacy
run had an intermittent timeout with a dispatch-start event but no completion;
its unsuccessful receipt is retained. The exact example passed three diagnostic
replays with unchanged bounds, followed by the complete five-family pass. No
cause was established and no timeout was relaxed. Fresh hosted verification of
all corrections remains required; the final-head receipts are linked from PR #493.

### Corrected source candidate and custom-assertion parser repair

At source head `40bfe36c6c7fc1b83347ebba2c0ea523ab4635ab`, tested merge
`924da1bdd62b9f592516dfddf6c151739312a7df` has the same source tree
`02917b48fb2c8681eea0ea1ae2a21a6cb1bbe74e`. The
[second compatibility run](https://github.com/edithatogo/fyi-cli/actions/runs/44057966689)
passed all six stdio cells, with nine Rust and 16 SDK contracts per cell. The
Linux source step passed all 63 cases without skips, including 12 source replays,
five generated properties and four privacy negative controls. Hypothesis
reported 66 passing generated examples; the source step took 26.71 seconds.

[Full Python CI](https://github.com/edithatogo/fyi-cli/actions/runs/44057966812/job/133952097248)
passed 1,799 tests with two opt-in live-site tests skipped. All 22 SDK, 38 upstream,
54 shared-corpus, five generated and four privacy-sensor cases ran without skips.
Rust 1.99.1 workspace verification passed 811 tests with one existing ignored
subprocess entry point. Configured line coverage passed at 93.83% (17,649 of
18,809 lines); the 90% gate is unchanged.

The second mutation report is retained as failed: 11 baselines passed, 14 mutants
were caught and one was classified unviable. The narrowed write-retry mutation
actually reached its intended write assertion, but Rust printed its custom
message without the generic `assertion failed` prefix. The parser incorrectly
rejected that executed assertion. The corrected manifest declares the exact
message, and the runner accepts it only immediately after the selected test's
unique panic frame with one failed test and exit status 101. Other log copies,
unrelated assertions, generic panics and compile failures remain rejected.

The actual retained failure reproduced RED before this correction. The corrected
parser passes 28 unit tests and classifies all 15 retained mutation logs as
caught; it still rejects the earlier read-assertion failure for the write-specific
target. This replay validates the parser, not a new mutation execution. The
original 14/1 report remains unchanged, and fresh execution of the corrected
runner is required before its full gate can pass.

Local Rust builds were killed with SIGKILL in dependency compilation, including
after reducing to one build job, no debug information and serial code generation.
Those attempts do not establish executable test success. Full local Python
3.15.2 environment preparation also stopped on missing CFFI development
headers; the successful local frozen test environment uses Python 3.12.14.
Hosted dependency preparation and runtime checks remain required. No Rust
version, source dependency, runtime lock or existing required gate was changed
to conceal either environment limitation.

### Acceptance boundary

The fixture verifier establishes syntax/schema/identity expectations. It does
not prove the production feed helper rejects malformed XML; that helper still
extracts link text and the deliberately truncated feeds preserve complete
links. Feed implementation remains with #170. Raw transfer and filesystem-cache
qualification, immutable attachment content, atomic upstream state transitions,
platform ACL/power-loss behavior and genuine future profile migration remain
separate gates. New matrix and mutation configuration requires actual passing
execution before it counts as acceptance evidence. The track remains open and
unarchived; no complete-workspace mutation score or live-platform certification
is claimed.

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

## Independent increment: degraded stdio startup and restart recovery

Reconciled against master `39591dcf91bb50ed847b0d36bdc3bd670a7e60a7` on
2026-11-14. The MCP files and owned evidence had not changed since PR #443;
no open MCP PR supplied this path. `set_degraded` existed only as an internal
policy control. This increment adds the operator startup setting
`FYI_MCP_REMOTE_DEGRADED` and reuses the existing SDK launcher and v2 preparation
fixtures. No source transport, application CLI, shared workflow or manifest is
changed.

Tests first reproduced seven failures against the unchanged executable: two
degraded-mode scenarios still discovered write tools, and five malformed-setting
scenarios exited successfully instead of rejecting the configuration. The base
debug executable SHA-256 was
`997c85e8a734685ff9460d702d74f3c0186e5115a6300d648e5ff9dbe9821827`.
The failing test checkpoint is
[`09fa80e6cf04`](https://github.com/edithatogo/fyi-cli/commit/09fa80e6cf04ec0c70374840fe4d199f8e908fbb).

The repaired path accepts only `true`/`1` or `false`/`0` (unset means normal).
Degraded startup hides remote writes and denies all three write preparations
even when called directly; it preserves configured read definitions and local
health/list/ping. The SDK executes normal → degraded → recovered subprocess
launches for both spellings, verifies fresh confirmation tokens after restart,
and confirms invalid commits remain private policy/confirmation errors. Existing
read-only scenarios now also prove that clearing degraded mode does not grant
write capability. Five invalid settings fail before database creation, with
empty stdout and no rejected values or private paths in stderr. Rust tests
additionally reject ambiguous and non-Unicode controls without global env races.

Local Linux verification with Rust 1.99.1 and MCP SDK 1.30.0: 129 MCP Rust tests
(124 unit/boundary and five executable wire tests), 22 SDK scenarios and 129
fixture/verifier cases passed. Clippy with warnings denied, Rust formatting and
Ruff passed. SDK requests are bounded to five seconds and sessions to thirty;
invalid-startup subprocesses have a five-second timeout. Use:

```bash
cargo test -p fyi-mcp --locked
python -m pytest -q tests/test_mcp_client_interop.py -k 'degraded or read_only'
python -m pytest -q tests/test_mcp_client_interop.py tests/test_remote_mcp_contract.py
```

Recovery here requires a new subprocess. It does not establish live in-process
operator toggling, durable replay, successful source reads/commits, new source
fault coverage or a non-Linux platform run. No deprecated profile is invented.
The focused PR is handed to the sole integration coordinator for an exclusive
final verification/merge window; this workstream does not refresh or merge it
concurrently. The broader track remains in progress and unarchived.

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
| Versioned JSON/RSS/Atom/error fixtures | MCP historical v1, current v2 and complete tool snapshot; existing source JSON/feed tests | A shared synthetic JSON/status corpus now drives actual MCP/source calls; RSS/Atom samples remain inline, without versioned malformed-feed cases |
| Every remote schema, annotation, result and error | All eight remote tools represented; actual catalogue, handler negatives and SDK write preparations verified | All eight current tools now have actual SDK/source success and wire assertions; original authority-feed/provenance/error-classification requirements remain open |
| Disabled/read/degraded/write stdio modes | Actual disabled/read-enabled/write-gated sessions plus strict startup-degraded mode, direct denial, restart recovery and privacy through the SDK | Startup-degraded criterion evidenced on Linux; recovery is restart-based, with no live in-process toggle or durable replay claimed |
| Property/mutation boundaries | Generated invalid SDK arguments; exact integer/depth/schema bounds; deterministic scope/clock/replay; 39 caught selected mutants; reused state/SSRF tests | Broader generated pagination/state/URL/path properties and SSRF/source-guardrail/error-classification mutations |
| Latency/memory/request/byte/concurrency | Recorded local SDK latency/RSS budgets; admitted-attempt, payload-byte, duration and concurrency tests; existing source small-body/count tests | Current-tool ordinary failure counts and redirect counts are observed exactly; raw multi-chunk cancellation/peak memory, other source retries and MCP-to-source allowance propagation remain open |
| Compatibility/release matrix | Linux Rust 1.99.1 and official SDK 1.29.0/1.30.0 stdio/HTTP; existing stable/MSRV jobs; explicit matrix | Windows/macOS runtime/client combinations have not been executed here; locked SDK results depend on final-head CI |
| Deterministic faults | MCP pending-future timeout, clock skew, expiry, circuit cooldown/recovery; source malformed JSON and pacing-wait tests | Disconnect, truncation and malformed chunk/header faults now execute through MCP; stalled headers/body and corrupt reused caches remain open |
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
refreshed heads and new required checks for PR #441. That slice and the
schema/client PR #443 are merged. The independent degraded-startup increment
has its own final-head gate and requires the sole integration coordinator's
exclusive merge window.
Final PR heads, Actions links and merge commits are recorded in the PRs and
the delivery report. Source acquisition, SQLite core, localization, CLI
dispatch, TUI, packaging and shared workflows remain with their assigned owners.

## Epic #169 resumption: credential configuration contracts

### Reconciled delivery baseline

Live master `9050a6649ebbcf48e6e2919e5bc19bbed4b4f8c0` includes PR #449's
degraded stdio startup and restart recovery. Its final head
`19ff55e9b6579e7396cb5a5e6832d20bb64b2b0d` was independently rechecked: all
12 protected contexts succeeded with their required app identities, including
[CI 43577452760](https://github.com/edithatogo/fyi-cli/actions/runs/43577452760)
and [CodeQL 43577452908](https://github.com/edithatogo/fyi-cli/actions/runs/43577452908).
The optional offline AI job remained queued and is not counted as passed. The
already-merged startup/recovery path is retained, not reimplemented. Its Linux
subprocess evidence does not establish a successful upstream write, hot mode
switching or durable restart replay. The epic and this track stay open.

### Test-first change

Published checkpoint `d17d5dfaefe17947c62f963b2c90a2995a9c1daf` contains the
failing tests before production changes. Against unchanged master `9050a66`,
the policy admitted distinct IDs that resolve to one credential environment
variable, and the real MCP executable accepted that configuration. Both rejection
tests failed on those exact outcomes; the distinct-ID compatibility test passed.
All identifiers, endpoints and credential markers are synthetic, with no source
network I/O.

Policy construction now requires unique protected credential names across the
entire allowlist, including entries whose write capability is off. It retains
the legacy ASCII-uppercase and hyphen-to-underscore operator naming convention.
Write dispatch uses the same name derivation; exact instance IDs and independent
capabilities are preserved. Invalid mappings fail before SQLite creation or
protocol output, without reflecting private values. No source transport,
database core, application CLI, localization, shared manifest/workflow or
Python-runtime change belongs to this increment.

### Executed verification

- `cargo test --locked -p fyi-mcp`: **133 passed**, comprising 127 unit/boundary
  and six executable wire tests. The new contracts exercise 480 generated alias
  configurations, both configuration orders and all write-capability pairs,
  distinct-ID compatibility and legacy key examples. Two ambiguous subprocess
  configurations fail within five seconds with no database/protocol side effect
  and no private ID, endpoint, database path or credential in stderr.
- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
  passed on the production candidate.
- `cargo test --workspace --all-features --locked`: **663 passed, zero failed,
  one ignored**, including doctests. Two earlier compilation attempts exhausted
  local disk before any tests ran. The successful run used
  `CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0` after removing only reproducible
  build artifacts; assertions, features, tests and repository CI configuration
  were unchanged. Saved earlier coverage reports were retained.
- Three targeted, temporary mutations were each rejected by executed test
  assertions: disabling collision rejection, removing ASCII case folding, and
  removing separator normalization. No mutation was accepted or counted on a
  compile failure. This is a three-operator sensor check, not a broad mutation
  score. The production policy file was restored byte for byte with SHA-256
  `b0e4475b5e772c7532be1581be970c73864aa6e3d44cdf7bc57a2ef034a5f2a1`.
- Existing Conductor and remote-MCP fixture verifiers and diff whitespace checks
  passed. Required hosted checks, including the unchanged coverage/security
  gates, remain mandatory on the final integration head.

PR [#452](https://github.com/edithatogo/fyi-cli/pull/452) publishes the RED
checkpoint `d17d5dfaefe17947c62f963b2c90a2995a9c1daf` and implementation
`80d5f134e259078963a8e7904662b63611fb7f8e`. The published implementation tree
`054e8f9d188c9fb34326b73fabbb3c28cfa07c1f` matches the locally verified tree.
This record does not pre-approve hosted checks or a later integration head;
the PR retains their exact-head evidence and the coordinator's merge decision.

At candidate publication, final master reconciliation, merge and branch pruning
were handed to the sole integration coordinator. This bounded normalization
contract does not complete the broader source URL/path/pagination/state
properties, raw-transfer/fault, platform,
durable-audit/replay, attachment immutability or server compare-and-set acceptance.

### Final credential integration receipt

After the user's explicit instruction to proceed with merging, #450 was integrated
first. Its master commit `70fef298442c72744583deb39c87ed83e005e60b` was then merged
once into the existing #452 branch with a compare-and-set fast-forward update.
Both histories and the eight-file MCP delta were preserved. No source, shared-CI
or runtime branch was refreshed as part of that integration.

Final head `5f2a3a38bcd1abd61503d1a7c1c33c42b854f5a8` passed all 12 protected
contexts, matched by application ID and head, plus Rust 1.99.1 MSRV, Conductor,
dependency, privacy, property/fuzz and workflow checks. It was squash-merged as
`e95343801ee9a5a3946a66334eaae1b3a8466704` on 14 November 2026. The final head,
GitHub tested merge `f0cb611c1df8fb9ba6d0359a953e572f50c6d25b` and actual merge
all have tree `9170e9a02a56710308f9b83decc3b1a4f5f9ea1d`. The expected-head
merge guard was used, and master had not moved. GitHub deleted the remote branch;
its absence was verified.

- [Workspace Tests](https://github.com/edithatogo/fyi-cli/actions/runs/43597417302/job/132454806483):
  **676 passed, zero failed, one ignored**, including **127 MCP unit/boundary
  tests plus six executable wire tests**, and the 13 merged terminology tests.
  Generated credential-alias rejection, exact-ID/key compatibility and the
  five-second subprocess privacy/no-database/no-protocol contract all executed.
- [Code Coverage](https://github.com/edithatogo/fyi-cli/actions/runs/43597417302/job/132454806566):
  **675 instrumented tests passed**, one ignored; **14,092/14,856 lines (94.86%)**,
  retaining the existing 90% gate and exclusions.
- [Python Tests](https://github.com/edithatogo/fyi-cli/actions/runs/43597417302/job/132454806401):
  **1,512 passed, two skipped**, including all **22 MCP client cases** with SDK
  **1.30.0** on Python **3.12.16**. The degraded/recovery suite remains intact;
  this run does not qualify the separately owned Python 3.15.1 migration.
- [CodeQL](https://github.com/edithatogo/fyi-cli/actions/runs/43597417375) and all
  remaining protected checks passed. Optional self-hosted offline AI remained
  queued and is not counted as passed. Final review readback found no submitted
  review or inline thread; this is not a fabricated independent approval.

Issues #169/#173 remain open. The shared project index now lists this track as
active, consistent with its registry, metadata and original incomplete criteria.
No broader source/fault/transfer, platform, property/mutation or durability gate
is waived, and the track is not archived.


## Actual upstream contract increment — 2026-11-14

Baseline `76de6a2` includes the coordinator's #452/#456 integration. RED at local
checkpoint `bb2951b`: the actual-upstream versioned fixture inventory assertion
failed because that corpus was missing. This is a harness-coverage gap, not a
claim that existing endpoint implementations failed.

The new `upstream-v1.json` corpus and non-forwarding local TLS/CONNECT server
exercise the unchanged production executable and `SyncClient` with the official
SDK. No source file, live authority, system trust, runtime lock or shared workflow
is changed. The fixture's ephemeral CA and synthetic protected header are scoped
to the child; only the fixed `.invalid` target is accepted. See
`docs/mcp-upstream-harness.md` for the reviewable matrix and source interfaces.

**37 tests pass locally**: 16 successful scenarios, 15 immediate failure variants,
two production-timeout stall cases, inventory, proxy non-forwarding, stale/read-only
rejection, and a generated query/limit test with 12 deterministic examples.
Success covers all five existing read tools and
all three write tools, including multipart bytes/MIME, no-I/O prepare/invalid
confirmation/replay, exact JSON payloads, Unicode, source response variants and
GET-then-PUT preflight. Failure covers the six required status classes, malformed
JSON/headers/chunks, missing version, disconnect/truncation, uncached 304 and
foreign/loopback redirect rejection. Ordinary failures observe exactly one wire
request; successful same-origin redirect observes two. These are current tool
contracts, not generic source retry qualification or atomic upstream CAS.

The stall tests first failed their executed assertions while the fixture still
returned a successful fast response. The implemented header/body stalls now
remain open until the unchanged source client's 60-second timeout. Both assert
50–70 seconds elapsed, a private error, exactly one request, independent peer
connection closure and a responsive subsequent ping. Their fixture/SDK deadlines
are longer than production's; no production timeout or URL/TLS check was changed.

Three temporary **MCP-only** mutations were each caught by executed wire/result
assertions: removing the search result cap, removing the protected write header,
and bypassing state preflight. Each mutant compiled. Production source was
restored byte for byte, rebuilt, and the full suites rerun. No `sync.rs` or source
mutation was made; this is not a broad source mutation score.

Local environment: Linux x86_64, Rust 1.99.1 and Python 3.12.14. The initial 35
cases passed with installed SDK 1.29.0. The final **37 passed in 135.41 seconds**
with locked SDK **1.30.0** and unchanged `uv.lock`. `cargo fmt --all -- --check`,
strict workspace/all-target/all-feature Clippy and the full Rust workspace pass:
**676 passed, zero failed, one ignored**, on baseline `76de6a2` without #457's
separate journal code. The TLS child-trust runtime fixture explicitly skips on
other platforms; those combinations remain unqualified. Exact combined/hosted
counts and final candidate/review evidence belong on the focused PR. Optional
queued AI does not count as passed.

Automatic approval review rejected the earlier unguarded full Python run on a
legacy scheduler's external feed attempt. That run is not a pass. Separate
PR #458 supplies synthetic feeds and an isolated network-attempt sensor; this
unpublished harness branch was then reconciled once onto its fixed candidate
`d42a78f1e941b39ddf82df8eba77c3a6cab0a4db`. The original RED checkpoint
`bb2951b` is retained locally, with its unchanged inventory test replayed on the
dependency. Published RED `0883e4cf6702bc5167e464e2d2253c5484c9c877` and
implementation `95631aa29f55185932f1e76d913b98eb8b77039b` have the same trees
as their local checkpoints. Implementation tree:
`2f0af0e2475748915637306f65f27be6974c962b`. Both increments remain separate
for sequential coordinator review.

After including the offline fixture repair, the combined full local Python run
completed with **1,552 passed, zero failed, two skipped in 197.11 seconds**.
Inherited proxies were cleared and a loopback-only Python DNS/socket guard was
installed before pytest; **zero external Python DNS/socket attempts** occurred.
Swallowed attempts would fail the run. This guard is not an OS/native-binary
sandbox claim. The 37 actual-upstream cases, including both measured production
stalls, ran rather than skipped. The two skips are existing opt-in discovery
smoke tests. The rejected unguarded run remains excluded.

Required workflows filter pull requests to `master`/`main`. Focused harness
PR [#459](https://github.com/edithatogo/fyi-cli/pull/459) therefore targets `master` with #458 as an explicit ancestry dependency and
an isolated compare link for its own eight-file delta. No workflow changes or
checks are bypassed. The sole coordinator must integrate #458 first, reconcile
the dependent candidate once and verify the resulting final head before merge.

Remaining concrete interfaces/gates:

1. A source-owned bounded, typed **read-only authority feed** API. Current feed
   APIs mutate the application database; the original authority-feed MCP tool,
   RSS/Atom malformed fixtures and resource/provenance contract are incomplete.
2. A source constructor accepting MCP's remaining raw request/byte/time budget.
   Current default raw response allowance is 500 MiB versus MCP's 4 MiB result
   limit. Early stream cancellation and peak memory are not established here.
3. Typed safe source error kind/status/retryability and cache metadata; preserve
   privacy without parsing strings. Reused-cache corruption still requires its
   own deterministic sensor; Linux header/body timeout closure is now covered.
4. Immutable attachment content across source path reopen and atomic upstream
   state transition capability. Do not substitute multipart/preflight success.
5. Windows/macOS client/runtime/ACL, complete declared compiler matrix, broader
   source properties/mutations and real profile-withdrawal/migration evidence.

Durable replay/audit is independently delivered in #457. Its final reconciled
head `c8150242d5306bfb1c410eaae9114a0de6fbe13b` passed all 12 required checks
and merged as `103844052210104666e01d4994d9e2f77986851d`; the write-governance
track retains the exact journal receipt. This harness reuses that implementation.

## Combined integration and concurrent MCP processes — 15 November 2026 AEST

The user's later instruction authorized sequential integration. #458 was merged
first, after all 12 protected checks passed on `d42a78f`, as
`393241da3dd3124c969e2d3f4304b27a2cf8fd22`. Its hosted Python job executed the
offline denial sensor and passed **1,515 tests**, with two skips. #457 then
incorporated that repaired master, passed its fresh gates and merged as
`103844052210104666e01d4994d9e2f77986851d`. Both remote branches were pruned
and their absence verified. The new #459 candidate includes that merged base;
older candidate checks do not stand in for its final verification.

The journal's existing concurrency test uses two SQLite connections in one
process. Its executable restart test establishes pre-I/O failure persistence;
the original upstream harness exercises successful writes through one MCP
process at a time. The remaining combination is now an additional test in that
same harness:
`test_shared_journal_allows_one_subprocess_write_and_blocks_restart_replay`.

Two actual SDK clients start separate MCP processes sharing one journal. Both
prepare the same scope before concurrent confirmations. Exactly one POST and
successful result are allowed; the losing client must receive the precise
private durable-replay error. Both processes must still answer a ping. After
both stop, a third process must deny the same scope without another request.
Responses, diagnostics and persisted files must exclude the synthetic body,
idempotency key, credential and confirmation tokens. The full scenario is
bounded to 20 seconds, with five-second SDK call limits and existing teardown.
The fixture's expected exchange queue independently catches duplicate dispatch
even when the second upstream call itself returns an error.

The module now has **38 cases**; the earlier 37-case execution remains historical
evidence for its original candidate. Local syntax, Ruff check/format, MCP fixture,
layered-harness and Conductor checks passed. The available local pytest attempt
stopped during dependency import and did not execute this new case; it is not
RED or passing runtime evidence. PR #459 records the actual final combined head
and fresh hosted execution before merge. No dependency download, new harness,
source transport change or platform qualification is claimed by this addition.

Existing registry entries for these MCP tracks are corrected to retain their
open status; no other workstream's status changes. The remaining feed, cache,
raw-budget, immutable-attachment, upstream atomic-state, platform and broader
property/mutation requirements above still prevent archiving #173 or closing
#169. Neither supported protocol profile is declared deprecated.
