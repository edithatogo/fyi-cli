# Plan: Versioned public-safe process-event export

## Phase 1: Contract and fixtures

+ [x] Task: Define the versioned case/event/attachment metadata schemas. [checkpoint: c59079a]
+ [x] Task: Add cross-repository golden fixtures with source-order edge cases. [checkpoint: c59079a]
+ [x] Task: Add privacy-negative fixtures for identity and content fields. [checkpoint: c59079a]
+ [x] Task: Phase verification and checkpoint per `.conductor/workflow.md`. [checkpoint: c59079a]

## Phase 2: Deterministic emitter

+ [x] Task: Write failing tests for stable IDs, source ordering, and timestamp handling. [checkpoint: c59079a]
+ [x] Task: Implement the event projection and EvidenceDelta compatibility layer. [checkpoint: c59079a]
+ [x] Task: Add correction, deletion, and checkpoint semantics. [checkpoint: c59079a]
+ [x] Task: Add attachment metadata and WARC-link projection. [checkpoint: c59079a]
+ [x] Task: Phase verification and checkpoint per `.conductor/workflow.md`. [checkpoint: c59079a]

## Phase 3: Backfill and downstream handshake

+ [x] Task: Add resumable bounded export over a derived request store. [checkpoint: c59079a]
+ [x] Task: Benchmark representative and full-corpus-shaped fixtures. [checkpoint: c59079a]
+ [x] Task: Publish the pinned contract fixture for `fyi-archive` and `foi-process` CI. [checkpoint: c59079a]
+ [~] Task: Record the contract version and downstream acceptance evidence in GitHub issue #231. Contract evidence recorded; sidecar attachment export fixed and live-probe acceptance passed; full-corpus downstream acceptance remains open.
+ [~] Task: Phase verification and checkpoint per `.conductor/workflow.md`. Awaiting downstream acceptance.
