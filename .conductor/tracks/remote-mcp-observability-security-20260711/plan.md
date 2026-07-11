# Plan: Remote MCP observability and security

## Phase 1: Policy and threat model

- [ ] Task: Write failing tests for default-deny read/write policy and invalid configuration
- [ ] Task: Implement per-instance remote capability policy and startup validation
- [ ] Task: Update threat model, privacy review, credential flow, and data-retention decisions
- [ ] Task: Conductor review verification 'Policy and threat model'

## Phase 2: Operator controls

- [ ] Task: Write failing tests for kill switch, degraded mode, circuit breaker, and recovery
- [ ] Task: Implement deterministic operator controls and safe status reporting
- [ ] Task: Add configuration examples and rollback/disable runbook
- [ ] Task: Conductor review verification 'Operator controls'

## Phase 3: Observability and audit

- [ ] Task: Define versioned remote MCP error, audit, trace, and metrics schemas
- [ ] Task: Implement correlated MCP/SyncClient events with bounded cardinality
- [ ] Task: Add secret/PII redaction, retention, and cardinality property tests
- [ ] Task: Add incident-response and observability operator documentation
- [ ] Task: Conductor review verification 'Observability and audit'

## Phase 4: Closeout

- [ ] Task: Run unit, property, security, regression, performance, and full CI gates
- [ ] Task: Record GitHub issue #172 evidence and update metadata/registry
- [ ] Task: Conductor review verification 'Track closeout'
