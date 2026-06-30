# Plan: api-contract-hardening-20260630

## Phase 1: Contract Inventory
- [x] Task: Inventory API contracts and coverage (ecad245)
    - [x] Add a documented contract matrix for Rust API, CLI, sync, MCP, and archive surfaces
    - [x] Map existing tests to each contract area
    - [x] Identify high-risk untested error paths
- [~] Task: Conductor - User Manual Verification 'Contract Inventory' (Protocol in workflow.md)

## Phase 2: Response Validation And Error Semantics
- [ ] Task: Add malformed and partial response tests
    - [ ] Write failing tests for malformed JSON and missing required fields
    - [ ] Implement or tighten parsing/error behavior
    - [ ] Verify optional fields remain backward compatible
- [ ] Task: Add HTTP failure contract tests
    - [ ] Write failing tests for 401/403, 404, 429, and 5xx responses
    - [ ] Normalize context-rich, non-secret error messages
    - [ ] Verify CLI/MCP presentation where practical
- [ ] Task: Conductor - User Manual Verification 'Response Validation And Error Semantics' (Protocol in workflow.md)

## Phase 3: Sync/API Boundary Safety
- [ ] Task: Harden sync retry and preservation behavior
    - [ ] Add tests for retryable and non-retryable push/pull errors
    - [ ] Verify local dirty data is preserved on failed or stale remote responses
    - [ ] Confirm queue and conflict states remain recoverable
- [ ] Task: Keep live smoke tests opt-in and polite
    - [ ] Document environment-gated live smoke commands
    - [ ] Verify rate-limit/backoff behavior is covered by mocked tests
- [ ] Task: Conductor - User Manual Verification 'Sync/API Boundary Safety' (Protocol in workflow.md)

## Phase 4: Contract Documentation And Tooling
- [ ] Task: Add contract fixtures and docs
    - [ ] Commit safe fixture responses for common FYI/Alaveteli shapes
    - [ ] Document supported contract assumptions and known gaps
    - [ ] Add release checklist commands for contract tests
- [ ] Task: Conductor review
    - [ ] Run conductor-review for api-contract-hardening-20260630
    - [ ] Apply any fix recommendations
    - [ ] Push to GitHub
- [ ] Task: Conductor - User Manual Verification 'Contract Documentation And Tooling' (Protocol in workflow.md)

## Archive
- [ ] Archive track: move to archive/ directory
