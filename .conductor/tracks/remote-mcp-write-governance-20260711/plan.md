# Plan: Remote MCP write governance

## Phase 1: Write policy and confirmation contract

- [x] Task: Define write capability, protected credential, confirmation, idempotency, and replay contracts
- [x] Task: Write failing tests for default-disabled, read-only, expired, mismatched, and replayed writes
- [x] Task: Implement short-lived payload-bound single-use confirmation records
- [ ] Task: Conductor review verification 'Write policy and confirmation contract'

## Phase 2: Request and correspondence writes

- [ ] Task: Write failing WireMock tests for prepare/commit request creation and correspondence
- [x] Task: Implement governed request creation and correspondence tools via SyncClient
- [~] Task: Add idempotency, safe errors, and durable redacted audit events — process-local confirmation/replay works; restart durability and actual audit emission are being implemented under #169
- [ ] Task: Conductor review verification 'Request and correspondence writes'

## Phase 3: Attachments and state updates

- [x] Task: Write failing tests for attachment bounds, MIME/path handling, stale state, and rollback
- [x] Task: Implement bounded attachment writes and optimistic state update tools
- [x] Task: Add property, mutation, edge, security, and replay regression tests
- [ ] Task: Conductor review verification 'Attachments and state updates'

## Phase 4: End-to-end and operations

- [ ] Task: Add offline MCP stdio end-to-end prepare/commit/reject/replay scenarios
- [ ] Task: Add sandbox-only opt-in write smoke workflow and disable-by-default sensor
- [ ] Task: Update threat model, incident/rollback runbook, operator docs, and migration guide
- [ ] Task: Run full CI, coverage, security, compatibility, performance, and mutation gates
- [ ] Task: Record GitHub issue #171 evidence and update metadata/registry
- [ ] Task: Conductor review verification 'Track closeout'

## Remainder reconciliation — 2026-11-14

Baseline `70fef29` includes the existing confirmation and operation-budget gates.
The historical checked audit/durability item was not supported: the production
gate uses only an in-memory map, and its reusable audit serializer is not a
durable production write log. The parent and this track remain incomplete.

- [~] Add tests first for required persistent write state, restart replay denial,
  concurrent admission, failure/cancellation reservations and audit privacy.
- [ ] Implement a bounded MCP-owned SQLite journal. Persist a reservation before
  any upstream dispatch; retain uncertain outcomes, fail on storage corruption,
  and store only opaque scope tags and fixed audit fields, never request content,
  raw idempotency/confirmation tokens or credential values.
- [ ] Integrate production startup and prepare/commit/reject/replay/outcome events;
  preserve the existing payload-bound monotonic confirmation gate.
- [ ] Run focused and workspace verification, targeted mutations and final-head
  hosted gates; hand off a focused PR to the sole integration coordinator.

Source transfer and platform acceptance remain with their owners. This increment
does not establish atomic upstream compare-and-set or immutable attachment bytes.
No source/database-core/application-CLI/i18n/shared-workflow change is included.
