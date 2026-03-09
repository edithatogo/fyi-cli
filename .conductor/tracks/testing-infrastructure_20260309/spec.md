# Track Specification: Testing Infrastructure Improvements

## Overview
This track addresses critical gaps in the testing infrastructure identified in the comprehensive audit. The goal is to achieve research-grade testing maturity by adding end-to-end tests, API contract tests, running mutation testing, and fixing integration test issues.

## Current State
- **Unit Tests:** 187 tests ✅ Excellent
- **Property-Based (Hypothesis):** 19 tests ✅ Excellent (found 4 bugs)
- **Integration Tests:** 15 tests ⚠️ Partial coverage
- **E2E Tests:** ~10 tests ❌ Basic only
- **Mutation Testing:** Infrastructure ready ❌ Not run
- **API Contract Tests:** None ❌ Missing
- **Overall Maturity Score:** 72/100

## Functional Requirements

### 1. End-to-End CLI Tests
- [ ] Test actual `fyi-system` CLI command execution
- [ ] Verify CLI output and exit codes
- [ ] Test complete operator workflows
- [ ] Test feed ingestion → request → report → export flow
- [ ] Test error scenarios and help text

### 2. API Contract Tests
- [ ] Define FYI.org.nz API request/response schemas
- [ ] Create mock API server for testing
- [ ] Validate schema compatibility
- [ ] Test version compatibility
- [ ] Add contract tests for all API endpoints

### 3. Mutation Testing
- [ ] Run mutation_test.py on full codebase
- [ ] Document baseline mutation score
- [ ] Fix tests that don't catch mutants
- [ ] Achieve >80% mutation score
- [ ] Document surviving mutations

### 4. Integration Test Fixes
- [ ] Fix database fixture issues
- [ ] Fix hypothesis + tmp_path compatibility
- [ ] Add feed-to-request workflow test
- [ ] Add multi-user scenario tests
- [ ] Add concurrent access tests

## Non-Functional Requirements
- **Test Execution Time:** <5 minutes for full suite
- **Coverage:** Maintain >80% overall, target 95%
- **Mutation Score:** >80%
- **CI Integration:** All tests must pass in CI
- **Documentation:** All new tests documented

## Acceptance Criteria
1. ✅ 50+ new E2E tests added and passing
2. ✅ 20+ API contract tests added and passing
3. ✅ Mutation testing run with >80% score documented
4. ✅ All integration test failures fixed (0 failing)
5. ✅ Test execution time <5 minutes
6. ✅ All tests integrated into CI pipeline
7. ✅ Documentation updated with testing guide

## Out of Scope
- TypeScript migration testing
- Visual regression testing
- Accessibility testing
- Cross-platform testing (Linux/Mac)
- Chaos/resilience testing

## Success Metrics
- Testing Maturity Score: 72 → 90+
- E2E Coverage: 40% → 90%
- Integration Coverage: 60% → 90%
- Mutation Score: N/A → >80%
- Test Count: 187 → 280+
