# Specification: fyi-api-coverage-audit

## Overview
This track audits the FYI/Alaveteli API surface and brings the Rust core to **full parity** with the Python client, closing web-artifact gaps. It ensures the Rust implementation has complete coverage of all Alaveteli API endpoints currently handled by Python.

## Functional Requirements
1. **Search API:**
   - Implement `/search.json` endpoint support with query parameters
   - Support pagination, filtering, and sorting options
2. **Correspondence Management:**
   - Implement `add_correspondence` endpoint (POST correspondence to existing requests)
   - Support attachment upload with multipart form data
3. **Request State Management:**
   - Implement `update_request_state` endpoint
   - Support all valid state transitions per Alaveteli spec
4. **Prefilled URL Builder:**
   - Implement `build_prefilled_url` for `/new/<authority>` endpoints
   - Generate pre-populated request forms
5. **Remote Authorities/Body List:**
   - Implement authority discovery endpoints
   - Support authority list fetching and filtering
6. **Health & Version APIs:**
   - Implement `check_api_health` endpoint
   - Implement `get_api_version` endpoint
7. **RSS/Feed Support:**
   - Implement RSS feed parsing
   - Support authority-specific feeds
   - Implement search result feeds

## Non-Functional Requirements
- **Test Coverage:** Live-safe contract tests using `wiremock` for all endpoints
- **Performance:** API calls must complete within 5 seconds under normal conditions
- **Compatibility:** Maintain backward compatibility with existing Rust API surface
- **Documentation:** Comprehensive API documentation with examples

## Acceptance Criteria
- All missing endpoints implemented in `crates/fyi-core/src/sync.rs`
- Full parity with Python `alaveteli_client.py` achieved
- Live-safe integration tests passing with `wiremock`
- Coverage report documenting API surface completeness
- No regression in existing Rust API functionality

## Out of Scope
- User interface changes
- Database schema modifications
- Performance optimization beyond functional requirements

## Dependencies
- None (foundational track)

## Success Metrics
- **API Parity:** 100% coverage of Python client endpoints
- **Test Coverage:** 90%+ code coverage on new endpoints
- **Zero Regressions:** All existing tests continue passing
