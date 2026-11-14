# Plan: Remote MCP observability and security

## Phase 1: Policy and threat model

- [x] Task: Write failing tests for default-deny read/write policy and invalid configuration
- [x] Task: Implement per-instance remote capability policy and startup validation
- [x] Task: Update threat model, privacy review, credential flow, and data-retention decisions
- [ ] Task: Conductor review verification 'Policy and threat model'

## Phase 2: Operator controls

- [ ] Task: Write failing tests for kill switch, degraded mode, circuit breaker, and recovery
- [ ] Task: Implement deterministic operator controls and safe status reporting
- [ ] Task: Add configuration examples and rollback/disable runbook
- [ ] Task: Conductor review verification 'Operator controls'

## Phase 3: Observability and audit

- [~] Task: Define versioned remote MCP error, audit, trace, and metrics schemas
- [~] Task: Implement correlated MCP/SyncClient events with bounded cardinality
- [~] Task: Add secret/PII redaction, retention, and cardinality property tests
- [~] Task: Add incident-response and observability operator documentation
- [ ] Task: Conductor review verification 'Observability and audit'

## Phase 4: Closeout

- [ ] Task: Run unit, property, security, regression, performance, and full CI gates
- [ ] Task: Record GitHub issue #172 evidence and update metadata/registry
- [ ] Task: Conductor review verification 'Track closeout'

## Bounded MCP dispatch increment — 2026-11-15

Focused PR [#460](https://github.com/edithatogo/fyi-cli/pull/460).
Existing ordinary ChatGPT workstream; sole integration coordinator retains final
verification, reconciliation, merge and pruning. The original candidate used
fixed #459; final integration now retains merged #458/#457/#459, including the
shared-journal concurrent-process/restart/privacy test. The final head and
required Actions receipt are recorded on the PR.

- [x] Write executed failing tests for local status discovery, fixed metric
  dimensions, byte accounting, failure/timeout/cancellation and emitted privacy. `3e6051b` (RED).
- [x] Implement typed process-local dispatch telemetry using existing tracing
  and policy ownership; no additional budget engine or telemetry dependency. `985a1dd`.
- [x] Verify local offline Rust/SDK/full Python contracts, linting and three
  targeted mutations; document measurement and retention scope. `985a1dd`.
- [x] Reconcile once with merged #458/#457/#459 in the requested sequential
  window, preserving durable completion for success and failure and the latest
  harness/evidence. Base checkpoint `3730a8d`.
- [~] Verify required final-head GitHub checks, configured coverage and review
  state; record the actual merge and branch cleanup on #460.

The original phase remains incomplete. Source-owned HTTP status/kind/retryability,
raw-byte/cache/retry/throttle counters, pacing/cache/guardrail child events and
cross-platform qualification are not established by MCP dispatch telemetry.
Only this track's stale completed registry/index entries are corrected to active;
other workstreams are unchanged. Source adapters, runtime and workflows retain
their owners. The existing durable write journal remains intact.
