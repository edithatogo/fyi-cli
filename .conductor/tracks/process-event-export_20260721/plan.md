# Plan: Versioned public-safe process-event export

## Phase 1: Contract and fixtures

- [ ] Task: Define the versioned case/event/attachment metadata schemas.
- [ ] Task: Add cross-repository golden fixtures with source-order edge cases.
- [ ] Task: Add privacy-negative fixtures for identity and content fields.
- [ ] Task: Phase verification and checkpoint per `.conductor/workflow.md`.

## Phase 2: Deterministic emitter

- [ ] Task: Write failing tests for stable IDs, source ordering, and timestamp handling.
- [ ] Task: Implement the event projection and EvidenceDelta compatibility layer.
- [ ] Task: Add correction, deletion, and checkpoint semantics.
- [ ] Task: Add attachment metadata and WARC-link projection.
- [ ] Task: Phase verification and checkpoint per `.conductor/workflow.md`.

## Phase 3: Backfill and downstream handshake

- [ ] Task: Add resumable bounded export over a derived request store.
- [ ] Task: Benchmark representative and full-corpus-shaped fixtures.
- [ ] Task: Publish the pinned contract fixture for `fyi-archive` and `foi-process` CI.
- [ ] Task: Record the contract version and downstream acceptance evidence in GitHub issue #231.
- [ ] Task: Phase verification and checkpoint per `.conductor/workflow.md`.
