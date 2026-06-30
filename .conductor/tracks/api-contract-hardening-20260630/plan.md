# Plan: api-contract-hardening-20260630

## Phase 1: Contract Inventory
- [x] Task: Inventory API contracts and coverage (ecad245)
    - [x] Add a documented contract matrix for Rust API, CLI, sync, MCP, and archive surfaces
    - [x] Map existing tests to each contract area
    - [x] Identify high-risk untested error paths
- [x] Task: Conductor - User Manual Verification 'Contract Inventory' (Protocol in workflow.md) (e806a03)

## Phase 2: Response Validation And Error Semantics
- [x] Task: Add malformed and partial response tests (e806a03)
    - [x] Write failing tests for malformed JSON and missing required fields
    - [x] Implement or tighten parsing/error behavior
    - [x] Verify optional fields remain backward compatible
- [x] Task: Add HTTP failure contract tests (e806a03)
    - [x] Write failing tests for 401/403, 404, 429, and 5xx responses
    - [x] Normalize context-rich, non-secret error messages
    - [x] Verify CLI/MCP presentation where practical
- [x] Task: Conductor - User Manual Verification 'Response Validation And Error Semantics' (Protocol in workflow.md) (e806a03)

## Phase 3: Sync/API Boundary Safety
- [x] Task: Harden sync retry and preservation behavior (e806a03)
    - [x] Add tests for retryable and non-retryable push/pull errors
    - [x] Verify local dirty data is preserved on failed or stale remote responses
    - [x] Confirm queue and conflict states remain recoverable
- [x] Task: Keep live smoke tests opt-in and polite (e806a03)
    - [x] Document environment-gated live smoke commands
    - [x] Verify rate-limit/backoff behavior is covered by mocked tests
- [x] Task: Conductor - User Manual Verification 'Sync/API Boundary Safety' (Protocol in workflow.md) (e806a03)

## Phase 4: Contract Documentation And Tooling
- [x] Task: Add contract fixtures and docs (e806a03)
    - [x] Commit safe fixture responses for common FYI/Alaveteli shapes
    - [x] Document supported contract assumptions and known gaps
    - [x] Add release checklist commands for contract tests
- [x] Task: Conductor review (e806a03)
    - [x] Run conductor-review for api-contract-hardening-20260630
    - [x] Apply any fix recommendations
    - [x] Push to GitHub
- [x] Task: Conductor - User Manual Verification 'Contract Documentation And Tooling' (Protocol in workflow.md) (e806a03)

## Archive
- [~] Archive track: move to archive/ directory
