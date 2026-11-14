# Plan: Remote MCP contract harness

## Phase 1: Versioned contracts

- [~] Task: Inventory remote MCP and Alaveteli contract variants and assign fixture versions
- [~] Task: Add shared success/error JSON, RSS, Atom, header, and malformed fixtures
- [x] Task: Add schema and annotation snapshot/contract tests for every remote tool
- [~] Task: Conductor review verification 'Versioned contracts' — MCP v2/catalogue verified; shared source/feed versions remain open

## Phase 2: Layered test harness

- [~] Task: Add unit, integration, end-to-end stdio, smoke/system, regression, and sanity suites — startup recovery plus successful SDK/source calls for all eight current tools now execute locally; source feeds/stalls/cache/platform acceptance remains open
- [~] Task: Add property tests for bounds, normalization, state expectations, and redaction
- [x] Task: Add edge and security tests for SSRF, capabilities, guardrails, and secrets
- [~] Task: Conductor review verification 'Layered test harness' — current-tool SDK/source success, status/disconnect/chunk/redirect faults pass on Linux; full original feed/stall/cache/raw-budget/platform criteria remain open

## Phase 3: Mutation and performance

- [~] Task: Add mutation targets for policy, confirmation/replay, SSRF, guardrails, and errors
- [x] Task: Define and test latency, memory, request, byte, and concurrency budgets — bounded MCP/local fixture evidence; source-transfer bounds separately gated
- [~] Task: Add deterministic compatibility matrix for Rust/MSRV, operating systems, and MCP clients — published with unexecuted Windows/macOS combinations explicit
- [~] Task: Conductor review verification 'Mutation and performance' — 39 viable selected mutants caught and local budgets passed; broader targets/platforms pending

## Phase 4: Usability and release readiness

- [x] Task: Add operator usability scenarios and actionable error-message checks
- [x] Task: Automate release-readiness evidence and fixture drift detection — existing required suites; incomplete release rows explicitly gated
- [~] Task: Run all layered and expensive gates at documented execution points
- [x] Task: Record GitHub issue #173 evidence and update metadata/registry — #173 and epic #169 reopened with exact incomplete criteria; final heads/checks/merges recorded on the PRs and delivery report
- [ ] Task: Conductor review verification 'Track closeout'

## Reconciliation — 2026-11-14

The registry previously described this track as implemented, but its plan and metadata had no completed acceptance evidence. The static v1 fixture check does not verify the executable. Reopened for protocol, client, schema, privacy and budget verification; the owned registry entry is reconciled to in progress with explicit closure dependencies.

- Baseline `a816dcd`: four executable stdio regressions failed (negotiation, batches, notifications, and error envelopes).
- Official Python MCP SDK 1.29.0 baseline: four interoperability/security scenarios failed.
- This slice targets the existing 2025-06-18 and 2025-03-26 profiles; it does not claim later MCP revisions.
- Remaining evidence is tracked in `acceptance-evidence.md`; unchecked acceptance gates remain open.

Confirmation and MCP operation-budget slice: deterministic expiry/scope/replay, concurrency, byte/duration, privacy and invalid-confirmation regressions pass; source raw-byte/retry and cross-restart/attachment guarantees remain open (see acceptance evidence).

## Current closure decision

- [x] Independent #169 increment: reject ambiguous per-instance credential
  environment mappings; verify generated identifier aliases and actual subprocess
  startup/privacy rejection before database creation. Reuse the existing MCP
  policy and Rust wire harness; retain separate source/platform acceptance.
  PR #452, RED checkpoint `d17d5df`, implementation `80d5f13`;
  480 generated configurations, 133 MCP tests and
  663 workspace tests passed; three targeted mutations caught at the original
  implementation checkpoint. After the user's explicit merge instruction, final
  combined head `5f2a3a3` passed all 12 protected contexts, MSRV and 676 workspace
  tests (one ignored), then merged as `e953438`. The feature branch was deleted;
  broader acceptance remains open.

Keep this track open and do not archive. The MCP implementation and executable
contract sensors are delivered in small PR slices; original source fixture/fault,
broader property/mutation and platform criteria remain partially evidenced.
`acceptance-evidence.md` records exact missing assertions, ownership boundaries,
commands and the distinction between executed handlers and synthetic examples.
No source transport, database core, localization, CLI dispatch, shared workflow
or packaging changes are authorized by this reconciliation.

## Offline fixture repair required by #169 — 2026-11-14

Automatic review stopped full Python verification when legacy scheduler tests
attempted a live feed. The current owner PR file lists contain no scheduler test
overlap. This repair is limited to test fixtures and this existing track; no
source implementation, shared test configuration, runtime lock or workflow edit.

- [x] Add an isolated subprocess regression that denies DNS/socket I/O before
  imports and fails on attempts even when legacy tests swallow exceptions.
  RED `5f45cec`: 36 legacy tests reported passes while the probe blocked 28 attempts.
- [x] Replace network feed retrieval in the three legacy scheduler test modules
  with synthetic successful/unavailable feed fixtures; retain real local cycle
  and database/report execution. Implementation `d1295c8`.
- [x] Verify the negative-control probe, real cycle outputs and scoped tests:
  39 passed; guarded full Python run 1,515 passed/two skipped, zero external
  Python DNS/socket attempts. See `offline-scheduler-evidence.md` for limits.
- [x] Publish focused PR #458 before resuming broader hosted verification.
- [~] Final-head checks/review and merge remain with the sole integration coordinator.

## Actual upstream fixture increment — 2026-11-14

Baseline `76de6a2` preserves the coordinator's #452/#456 integration. The
unpublished increment was then reconciled onto #458 at `d42a78f` before its
combined guarded verification. In this
ordinary ChatGPT workstream, tests first expose the missing versioned actual
MCP-to-source exchange corpus. An ephemeral local TLS/CONNECT fixture can drive
the unchanged production `SyncClient` without a production testing bypass,
external DNS/request, system trust modification or committed private key.

- [x] Add versioned synthetic wire exchanges for the existing five read and
  three write tools, preserving supported JSON shapes and Unicode.
  RED `0883e4c`, implementation `95631aa`; 37 tests and three caught mutations.
- [x] Execute real SDK prepare/commit/replay, exact wire payload/count,
  read-only denial, state preflight and bounded multipart assertions.
- [x] Add deterministic status/malformed/disconnect/chunk/redirect failures
  and measure production header/body timeout, connection closure and recovery;
  keep unexecuted cache, raw-transfer and platform requirements explicit.
- [x] Record local test-first/mutation evidence: 37 upstream tests, three caught
  mutations, guarded combined Python run 1,552 passed/two skipped and zero
  external Python DNS/socket attempts.
- [~] Record final-head CI/review evidence and request the sole coordinator's
  sequential final verification/merge window; #458 is a required dependency.

This does not replace #457's durable journal or alter source, archive, runtime
or workflow ownership. A source fixture is synthetic compatibility evidence,
not a claim that a live jurisdiction/platform has certified the behavior.
