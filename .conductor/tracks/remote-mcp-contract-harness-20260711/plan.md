# Plan: Remote MCP contract harness

## Phase 1: Versioned contracts

- [~] Task: Inventory remote MCP and Alaveteli contract variants and assign fixture versions
- [ ] Task: Add shared success/error JSON, RSS, Atom, header, and malformed fixtures
- [ ] Task: Add schema and annotation snapshot/contract tests for every remote tool
- [ ] Task: Conductor review verification 'Versioned contracts'

## Phase 2: Layered test harness

- [~] Task: Add unit, integration, end-to-end stdio, smoke/system, regression, and sanity suites
- [ ] Task: Add property tests for bounds, normalization, state expectations, and redaction
- [ ] Task: Add edge and security tests for SSRF, capabilities, guardrails, and secrets
- [ ] Task: Conductor review verification 'Layered test harness'

## Phase 3: Mutation and performance

- [ ] Task: Add mutation targets for policy, confirmation/replay, SSRF, guardrails, and errors
- [ ] Task: Define and test latency, memory, request, byte, and concurrency budgets
- [ ] Task: Add deterministic compatibility matrix for Rust/MSRV, operating systems, and MCP clients
- [ ] Task: Conductor review verification 'Mutation and performance'

## Phase 4: Usability and release readiness

- [ ] Task: Add operator usability scenarios and actionable error-message checks
- [ ] Task: Automate release-readiness evidence and fixture drift detection
- [ ] Task: Run all layered and expensive gates at documented execution points
- [ ] Task: Record GitHub issue #173 evidence and update metadata/registry
- [ ] Task: Conductor review verification 'Track closeout'

## Reconciliation — 2026-11-14

The registry previously described this track as implemented, but its plan and metadata had no completed acceptance evidence. The static v1 fixture check does not verify the executable. Reopened for protocol, client, schema, privacy and budget verification; shared registry reconciliation follows only after the owned changes merge.

- Baseline `a816dcd`: four executable stdio regressions failed (negotiation, batches, notifications, and error envelopes).
- Official Python MCP SDK 1.29.0 baseline: four interoperability/security scenarios failed.
- This slice targets the existing 2025-06-18 and 2025-03-26 profiles; it does not claim later MCP revisions.
- Remaining evidence is tracked in `acceptance-evidence.md`; unchecked acceptance gates remain open.
