# Plan: fyi-api-coverage-audit

## Phase 1: Search API Implementation

### 1.1 Search Endpoint
- [x] Task: Write failing tests for search API (`/search.json`)
- [x] Task: Implement search endpoint with query parameters
- [x] Task: Add pagination support (page, per_page)
- [x] Task: Add filtering and sorting options
- [x] Task: Conductor review verification 'Phase 1.1: Search API' — review workflow plus 53 focused Rust tests

### 1.2 Search Result Feeds
- [x] Task: Write failing tests for search result RSS feeds — `search_request_ids_from_feed` WireMock contract
- [x] Task: Implement RSS feed parsing for search results — bounded ID-discovery method with query propagation
- [x] Task: Conductor review verification 'Phase 1.2: Search Feeds' — review workflow plus local portable-MSVC execution

## Phase 2: Correspondence & Request Management

### 2.1 Add Correspondence
- [x] Task: Write failing tests for `add_correspondence` endpoint — WireMock verifies POST path, JSON payload, and response parsing
- [x] Task: Implement POST correspondence to existing requests
- [x] Task: Add multipart form data support for attachments — `add_correspondence_with_attachments` uses bounded file reads and multipart parts
- [x] Task: Test attachment upload functionality — WireMock verifies multipart field names and attachment bytes
- [x] Task: Conductor review verification 'Phase 2.1: Correspondence' — multipart, size, missing-file, and redaction checks

### 2.2 Update Request State
- [x] Task: Write failing tests for `update_request_state` — WireMock verifies PUT path, JSON payload, and response parsing
- [x] Task: Implement state transition logic
- [x] Task: Validate state values per Alaveteli API — fail-closed vocabulary validation; transition semantics remain server-authoritative because the payload has no current state
- [x] Task: Conductor review verification 'Phase 2.2: State Updates' — valid/invalid state tests pass locally

## Phase 3: Authority Discovery & Prefilled URLs

### 3.1 Remote Authorities
- [x] Task: Write failing tests for authority list endpoints — WireMock verifies wrapped authority response parsing
- [x] Task: Implement authority discovery endpoints
- [x] Task: Add authority filtering support — client-side case-insensitive matching across authority identifiers and labels
- [x] Task: Conductor review verification 'Phase 3.1: Authorities' — discovery, matching, error redaction, and feed checks pass

### 3.2 Prefilled URL Builder
- [x] Task: Write failing tests for `build_prefilled_url` — coverage includes query encoding and blank-tag handling
- [x] Task: Implement prefilled URL generation for `/new/<authority>`
- [x] Task: Test with various authority types — ministry, council, university, and agency-style slugs
- [x] Task: Conductor review verification 'Phase 3.2: Prefilled URLs' — slug, encoding, and blank-tag checks pass

## Phase 4: Health, Version & Feeds

### 4.1 Health & Version APIs
- [x] Task: Write failing tests for health and version endpoints — WireMock covers successful health and object-shaped version responses
- [x] Task: Implement `check_api_health` endpoint
- [x] Task: Implement `get_api_version` endpoint
- [x] Task: Conductor review verification 'Phase 4.1: Health APIs' — health/version contract checks pass

### 4.2 RSS/Authority Feeds
- [x] Task: Write failing tests for RSS feed parsing — RSS/Atom extraction, empty/malformed-safe handling, and watched-feed ingestion are covered
- [x] Task: Implement RSS feed parser
- [x] Task: Add authority-specific feed support — `pull_authority_feed` follows the `/feed` convention and rejects query/fragment paths
- [x] Task: Conductor review verification 'Phase 4.2: RSS Feeds' — RSS/Atom and authority-feed checks pass

## Phase 5: Integration Testing & Documentation

### 5.1 Live-Safe Contract Tests
- [x] Task: Create comprehensive wiremock test suite — sync tests cover JSON, multipart, feeds, authorities, health/version, guardrails, and error redaction
- [x] Task: Test all endpoints with realistic mock data — request, authority, feed, multipart, and database fixtures are covered
- [x] Task: Verify error handling for SyncClient endpoints — HTTP redaction, malformed payload, missing-file, guardrail, and retry tests pass; CLI/MCP presentation remains separately tracked in the contract inventory
- [x] Task: Conductor review verification 'Phase 5.1: Contract Tests' — review workflow plus local portable-MSVC test execution

### 5.2 Coverage Report & Documentation
- [x] Task: Generate API coverage report — CI Codecov report and 90% reusable-library gate are authoritative; no static report is committed
- [x] Task: Document all new endpoints with examples — `docs/RUST_API_MIGRATION.md` and `docs/api-contract-inventory.md`
- [x] Task: Update Rust API documentation — migration and contract inventory now cover the SyncClient surface
- [x] Task: Create migration guide for Python users — `docs/RUST_API_MIGRATION.md`
- [x] Task: Conductor review verification 'Phase 5.2: Documentation' — documentation review completed

## Completion Criteria
- [x] All phases complete for the Rust SyncClient track scope
- [x] API parity for the documented Rust SyncClient track scope; legacy Python-only presentation differences remain documented
- [x] All wiremock tests passing in CI and focused Rust tests passing locally
- [x] Coverage report shows the 90% reusable-library CI gate passing
- [x] Documentation complete and reviewed through Conductor review

## Track History
- **2026-07-08**: Track created for multi-jurisdictional expansion
- **2026-07-11**: Added Rust `search.rss` request-ID discovery with deduplication and query propagation; local execution is blocked by the absent MSVC Windows SDK, so CI remains the execution gate.
- **2026-07-11**: Added WireMock contract coverage for correspondence, state, authority, health, and object-shaped version APIs; local Rust execution remains blocked by the absent MSVC Windows SDK.
- **2026-07-11**: Added deterministic client-side authority filtering with WireMock coverage; remote filter query semantics remain intentionally unspecified.
- **2026-07-11**: Added async multipart correspondence uploads with attachment contract coverage; local Rust execution remains blocked by the absent MSVC Windows SDK, so CI is the execution gate.
- **2026-07-11**: Added fail-closed validation for the documented request-state vocabulary before PUT; graph-level transition validation remains pending because the payload does not include the current remote state.
- **2026-07-11**: Added prefilled URL coverage for authority slug variants, Unicode/reserved query values, and blank tags.
- **2026-07-11**: Added authority-scoped feed pulling with route and path-safety coverage, reusing the existing RSS/Atom ID parser and bounded request fetch path.
- **2026-07-11**: Reconciled API contract documentation and added `docs/RUST_API_MIGRATION.md`; coverage-report generation remains tied to the CI/Codecov gate rather than a locally generated static artifact.
- **2026-07-11**: Added redacted HTTP-error and missing-attachment tests for search feeds, authority discovery/feeds, and multipart correspondence; CLI/MCP-wide error presentation remains outside this Rust sync slice.
- **2026-07-11**: Conductor review closeout completed with portable-MSVC verification: 53 focused `sync::tests` passed; stale manual-verification markers were replaced by review evidence, and the plan now distinguishes server-authoritative transition semantics from client validation.

## Phase: Conductor Review Closeout
- [x] Task: Apply review findings — plan evidence, portable-MSVC verification, and Rust/Python stack documentation headings reconciled
- [x] Task: Execute review test gate — `cargo test --target x86_64-pc-windows-msvc -p fyi-core` passed: 148 tests, 0 failures
