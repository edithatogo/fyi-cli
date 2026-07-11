# Plan: Remote MCP write governance

## Phase 1: Write policy and confirmation contract

- [ ] Task: Define write capability, protected credential, confirmation, idempotency, and replay contracts
- [ ] Task: Write failing tests for default-disabled, read-only, expired, mismatched, and replayed writes
- [ ] Task: Implement short-lived payload-bound single-use confirmation records
- [ ] Task: Conductor review verification 'Write policy and confirmation contract'

## Phase 2: Request and correspondence writes

- [ ] Task: Write failing WireMock tests for prepare/commit request creation and correspondence
- [ ] Task: Implement governed request creation and correspondence tools via SyncClient
- [ ] Task: Add idempotency, safe errors, and durable redacted audit events
- [ ] Task: Conductor review verification 'Request and correspondence writes'

## Phase 3: Attachments and state updates

- [ ] Task: Write failing tests for attachment bounds, MIME/path handling, stale state, and rollback
- [ ] Task: Implement bounded attachment writes and optimistic state update tools
- [ ] Task: Add property, mutation, edge, security, and replay regression tests
- [ ] Task: Conductor review verification 'Attachments and state updates'

## Phase 4: End-to-end and operations

- [ ] Task: Add offline MCP stdio end-to-end prepare/commit/reject/replay scenarios
- [ ] Task: Add sandbox-only opt-in write smoke workflow and disable-by-default sensor
- [ ] Task: Update threat model, incident/rollback runbook, operator docs, and migration guide
- [ ] Task: Run full CI, coverage, security, compatibility, performance, and mutation gates
- [ ] Task: Record GitHub issue #171 evidence and update metadata/registry
- [ ] Task: Conductor review verification 'Track closeout'
