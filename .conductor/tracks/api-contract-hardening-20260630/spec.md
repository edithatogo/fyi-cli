# Specification: api-contract-hardening-20260630

## Overview
Harden the FYI/Alaveteli API integration boundaries in the Rust workspace so request, correspondence, sync, and archive flows are validated against explicit contracts. The goal is to make API changes, malformed responses, authentication failures, and rate limits visible and recoverable without risking local data integrity.

## Functional Requirements

### Phase 1: Contract Inventory
1. Inventory all Rust API payload structs, CLI API calls, sync API calls, MCP-exposed API-adjacent behavior, and archive public-web endpoints.
2. Identify which behaviors are covered by unit tests, mocked integration tests, or opt-in live smoke tests.
3. Document contract gaps and classify them by risk.

### Phase 2: Response Validation And Error Semantics
1. Add tests for malformed JSON, missing required fields, unexpected optional fields, non-2xx responses, rate limits, and authentication failures.
2. Ensure errors preserve enough context for CLI/MCP users without exposing secrets.
3. Normalize API error messages across CLI, sync, and MCP surfaces where practical.

### Phase 3: Sync/API Boundary Safety
1. Verify pull, push, queue, and conflict behavior under retryable and non-retryable API errors.
2. Add regression tests for local-data preservation when remote API responses are partial, stale, or conflicting.
3. Ensure live smoke tests remain opt-in and polite, with rate-limit/backoff protection.

### Phase 4: Contract Documentation And Tooling
1. Document supported FYI/Alaveteli contract assumptions and known gaps.
2. Add fixture-based contract test data for common API response shapes.
3. Provide a local command/checklist for running contract tests before release.

## Non-Functional Requirements
- No required tests should depend on live FYI.org.nz network access.
- Live tests must be opt-in through environment variables and must use polite rate limits.
- Error paths must avoid logging API keys, tokens, request bodies containing sensitive data, or credentials.
- Contract fixtures should be small, readable, and safe to commit.

## Acceptance Criteria
- API payload and sync boundary tests cover success, malformed, rate-limited, unauthorized, and server-error responses.
- CLI and MCP surfaces return actionable but non-sensitive error messages.
- Contract fixture files and docs describe current assumptions.
- Workspace `fmt`, `clippy`, and tests pass.
