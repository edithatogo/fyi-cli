# Plan: fyi-api-coverage-audit

## Phase 1: Search API Implementation

### 1.1 Search Endpoint
- [x] Task: Write failing tests for search API (`/search.json`)
- [x] Task: Implement search endpoint with query parameters
- [x] Task: Add pagination support (page, per_page)
- [x] Task: Add filtering and sorting options
- [ ] Task: Conductor - User Manual Verification 'Phase 1.1: Search API' (Protocol in workflow.md)

### 1.2 Search Result Feeds
- [x] Task: Write failing tests for search result RSS feeds — `search_request_ids_from_feed` WireMock contract
- [x] Task: Implement RSS feed parsing for search results — bounded ID-discovery method with query propagation
- [ ] Task: Conductor - User Manual Verification 'Phase 1.2: Search Feeds' (Protocol in workflow.md)

## Phase 2: Correspondence & Request Management

### 2.1 Add Correspondence
- [x] Task: Write failing tests for `add_correspondence` endpoint — WireMock verifies POST path, JSON payload, and response parsing
- [x] Task: Implement POST correspondence to existing requests
- [x] Task: Add multipart form data support for attachments — `add_correspondence_with_attachments` uses bounded file reads and multipart parts
- [x] Task: Test attachment upload functionality — WireMock verifies multipart field names and attachment bytes
- [ ] Task: Conductor - User Manual Verification 'Phase 2.1: Correspondence' (Protocol in workflow.md)

### 2.2 Update Request State
- [x] Task: Write failing tests for `update_request_state` — WireMock verifies PUT path, JSON payload, and response parsing
- [x] Task: Implement state transition logic
- [ ] Task: Validate state transitions per Alaveteli spec
- [ ] Task: Conductor - User Manual Verification 'Phase 2.2: State Updates' (Protocol in workflow.md)

## Phase 3: Authority Discovery & Prefilled URLs

### 3.1 Remote Authorities
- [x] Task: Write failing tests for authority list endpoints — WireMock verifies wrapped authority response parsing
- [x] Task: Implement authority discovery endpoints
- [x] Task: Add authority filtering support — client-side case-insensitive matching across authority identifiers and labels
- [ ] Task: Conductor - User Manual Verification 'Phase 3.1: Authorities' (Protocol in workflow.md)

### 3.2 Prefilled URL Builder
- [ ] Task: Write failing tests for `build_prefilled_url`
- [ ] Task: Implement prefilled URL generation for `/new/<authority>`
- [ ] Task: Test with various authority types
- [ ] Task: Conductor - User Manual Verification 'Phase 3.2: Prefilled URLs' (Protocol in workflow.md)

## Phase 4: Health, Version & Feeds

### 4.1 Health & Version APIs
- [x] Task: Write failing tests for health and version endpoints — WireMock covers successful health and object-shaped version responses
- [x] Task: Implement `check_api_health` endpoint
- [x] Task: Implement `get_api_version` endpoint
- [ ] Task: Conductor - User Manual Verification 'Phase 4.1: Health APIs' (Protocol in workflow.md)

### 4.2 RSS/Authority Feeds
- [ ] Task: Write failing tests for RSS feed parsing
- [ ] Task: Implement RSS feed parser
- [ ] Task: Add authority-specific feed support
- [ ] Task: Conductor - User Manual Verification 'Phase 4.2: RSS Feeds' (Protocol in workflow.md)

## Phase 5: Integration Testing & Documentation

### 5.1 Live-Safe Contract Tests
- [ ] Task: Create comprehensive wiremock test suite
- [ ] Task: Test all endpoints with realistic mock data
- [ ] Task: Verify error handling for all endpoints
- [ ] Task: Conductor - User Manual Verification 'Phase 5.1: Contract Tests' (Protocol in workflow.md)

### 5.2 Coverage Report & Documentation
- [ ] Task: Generate API coverage report
- [ ] Task: Document all new endpoints with examples
- [ ] Task: Update Rust API documentation
- [ ] Task: Create migration guide for Python users
- [ ] Task: Conductor - User Manual Verification 'Phase 5.2: Documentation' (Protocol in workflow.md)

## Completion Criteria
- [ ] All phases complete
- [ ] 100% API parity with Python client
- [ ] All wiremock tests passing
- [ ] Coverage report shows 90%+ for new code
- [ ] Documentation complete and reviewed

## Track History
- **2026-07-08**: Track created for multi-jurisdictional expansion
- **2026-07-11**: Added Rust `search.rss` request-ID discovery with deduplication and query propagation; local execution is blocked by the absent MSVC Windows SDK, so CI remains the execution gate.
- **2026-07-11**: Added WireMock contract coverage for correspondence, state, authority, health, and object-shaped version APIs; local Rust execution remains blocked by the absent MSVC Windows SDK.
- **2026-07-11**: Added deterministic client-side authority filtering with WireMock coverage; remote filter query semantics remain intentionally unspecified.
- **2026-07-11**: Added async multipart correspondence uploads with attachment contract coverage; local Rust execution remains blocked by the absent MSVC Windows SDK, so CI is the execution gate.
- **2026-07-11**: Added fail-closed validation for the documented request-state vocabulary before PUT; graph-level transition validation remains pending because the payload does not include the current remote state.
