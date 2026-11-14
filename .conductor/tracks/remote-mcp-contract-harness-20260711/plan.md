# Plan: Remote MCP contract harness

## Phase 1: Versioned contracts

- [~] Task: Inventory remote MCP and Alaveteli contract variants and assign fixture versions
- [~] Task: Add shared success/error JSON, RSS, Atom, header, and malformed fixtures
- [x] Task: Add schema and annotation snapshot/contract tests for every remote tool
- [~] Task: Conductor review verification 'Versioned contracts' — MCP v2/catalogue verified; shared source/feed versions remain open

## Phase 2: Layered test harness

- [~] Task: Add unit, integration, end-to-end stdio, smoke/system, regression, and sanity suites — startup-degraded mode and restart recovery now have executable SDK evidence; successful source read/commit integration remains open
- [~] Task: Add property tests for bounds, normalization, state expectations, and redaction
- [x] Task: Add edge and security tests for SSRF, capabilities, guardrails, and secrets
- [~] Task: Conductor review verification 'Layered test harness' — local/SDK modes, startup-degraded denial, restart recovery and privacy passed; source fault and full successful remote integration remain open

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

- [~] Independent #169 increment: reject ambiguous per-instance credential
  environment mappings; verify generated identifier aliases and actual subprocess
  startup/privacy rejection before database creation. Reuse the existing MCP
  policy and Rust wire harness; retain separate source/platform acceptance.

Keep this track open and do not archive. The MCP implementation and executable
contract sensors are delivered in small PR slices; original source fixture/fault,
broader property/mutation and platform criteria remain partially evidenced.
`acceptance-evidence.md` records exact missing assertions, ownership boundaries,
commands and the distinction between executed handlers and synthetic examples.
No source transport, database core, localization, CLI dispatch, shared workflow
or packaging changes are authorized by this reconciliation.
