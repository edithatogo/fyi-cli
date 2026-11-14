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

Existing ordinary ChatGPT workstream; sole integration coordinator retains final
verification, refresh, merge and pruning. Based on fixed #459, including #458's
offline acquisition repair; #457 remains a separate fixed journal candidate.

- [~] Write executed failing tests for local status discovery, fixed metric
  dimensions, byte accounting, failure/timeout/cancellation and emitted privacy.
- [ ] Implement typed process-local dispatch telemetry using existing tracing
  and policy ownership; no additional budget engine or telemetry dependency.
- [ ] Verify offline Rust and actual SDK contracts, linting, coverage and final
  candidate GitHub checks; document the precise measurement and retention scope.

The original phase remains incomplete. Source-owned HTTP status/kind/retryability,
raw-byte/cache/retry/throttle counters, pacing/cache/guardrail child events and
cross-platform qualification are not established by MCP dispatch telemetry.
No shared registry, source adapter, runtime or workflow changes are authorized
by this slice. The existing durable write journal remains #457's responsibility.
