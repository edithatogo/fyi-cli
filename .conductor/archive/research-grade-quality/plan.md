# Plan: Research-Grade Quality

## Phase 1: Coverage to 95%

### 1.1 Webapp Tests (53% → 95%)
- [x] Task: Test all webapp routes (/, /requests, /request/<id>)
- [x] Task: Test form handling (import, status update)
- [x] Task: Test HTML rendering with data
- [x] Task: Test error handling (404, 500)
- [x] Task: Test JSON API endpoints
- [x] Task: Test privacy redaction in web output

### 1.2 Scheduler Tests (56% → 95%)
- [x] Task: Test scheduler loop logic
- [x] Task: Test interval timing
- [x] Task: Test once mode vs continuous mode
- [x] Task: Test error handling in scheduler

### 1.3 Reporting Tests (89% → 95%)
- [x] Task: Test attention report generation
- [x] Task: Test triage report generation
- [x] Task: Test export functions
- [x] Task: Test handover document generation

### 1.4 Integration Tests
- [x] Task: Test full request lifecycle
- [x] Task: Test feed-to-request workflow
- [x] Task: Test export-import round trip

## Phase 2: Mutation Testing

### 2.1 Setup
- [x] Task: Install cosmic-ray or mutmut
- [x] Task: Configure mutation testing
- [x] Task: Run initial mutation analysis
- [x] Task: Document baseline mutation score

### 2.2 Improve Mutation Score
- [x] Task: Fix tests that don't catch mutants
- [x] Task: Add assertions for edge cases
- [x] Task: Test error conditions
- [x] Task: Achieve >90% mutation score

### 2.3 CI Integration
- [x] Task: Add mutation testing to CI pipeline
- [x] Task: Set mutation score threshold
- [x] Task: Configure scheduled mutation runs

## Phase 3: Load Testing

### 3.1 Setup
- [x] Task: Install locust
- [x] Task: Install pytest-benchmark
- [x] Task: Install memory-profiler
- [x] Task: Create load test scenarios

### 3.2 Performance Baselines
- [x] Task: Benchmark request ingestion
- [x] Task: Benchmark feed parsing
- [x] Task: Benchmark dashboard generation
- [x] Task: Benchmark export operations
- [x] Task: Measure memory usage patterns

### 3.3 Load Scenarios
- [x] Task: Test 10 concurrent requests
- [x] Task: Test 100 concurrent requests
- [x] Task: Test 500 concurrent requests
- [x] Task: Test sustained load (1 hour)
- [x] Task: Document performance characteristics

### 3.4 Optimization
- [x] Task: Identify bottlenecks
- [x] Task: Optimize slow paths
- [x] Task: Add caching where appropriate
- [x] Task: Document performance targets

## Phase 4: Hypothesis Testing

### 4.1 Setup
- [x] Task: Verify hypothesis is installed
- [x] Task: Learn hypothesis patterns
- [x] Task: Create hypothesis test utilities

### 4.2 Property Tests - Security
- [x] Task: Property test email redaction (any email format)
- [x] Task: Property test URL sanitization
- [x] Task: Property test bearer token redaction
- [x] Task: Property test payload sanitization

### 4.3 Property Tests - Data Integrity
- [x] Task: Property test export-import round trip
- [x] Task: Property test database operations
- [x] Task: Property test state transitions

### 4.4 Property Tests - Privacy
- [x] Task: Property test no PII in logs
- [x] Task: Property test no PII in errors
- [x] Task: Property test file permissions

## Phase 5: TypeScript Migration Preparation

### 5.1 Research & Planning
- [x] Task: Research Bun vs Deno vs Node.js
- [x] Task: Evaluate Commander.js alternatives
- [x] Task: Document TypeScript migration plan
- [x] Task: Create migration timeline

### 5.2 TypeScript Skeleton
- [x] Task: Create TypeScript project structure
- [x] Task: Configure TypeScript strict mode
- [x] Task: Set up Vitest testing
- [x] Task: Create basic CLI with Commander

### 5.3 Core Types
- [x] Task: Define TypeScript types for requests
- [x] Task: Define TypeScript types for authorities
- [x] Task: Define TypeScript types for events
- [x] Task: Define TypeScript types for settings

### 5.4 FYI API Client (TypeScript)
- [x] Task: Port FYI URL building
- [x] Task: Port request ID extraction
- [x] Task: Port API client functions
- [x] Task: Write TypeScript tests

### 5.5 Validation
- [x] Task: Run Python and TypeScript side-by-side
- [x] Task: Compare outputs
- [x] Task: Document differences
- [x] Task: Create migration guide

## Phase 6: Documentation & CI

### 6.1 Documentation
- [x] Task: Document testing strategy
- [x] Task: Document performance characteristics
- [x] Task: Document mutation testing process
- [x] Task: Create quality assurance guide

### 6.2 CI/CD
- [x] Task: Add coverage threshold to CI (>95%)
- [x] Task: Add mutation testing to CI
- [x] Task: Add load testing to CI
- [x] Task: Add TypeScript build to CI

### 6.3 Quality Gates
- [x] Task: Configure pre-commit hooks
- [x] Task: Add coverage check to PRs
- [x] Task: Add type checking to PRs
- [x] Task: Add performance regression detection

## Phase 7: Review Fixes

### 7.1 Track Status Correction
- [x] Task: Update track status from completed to in-progress
- [x] Task: Document remaining work and lessons learned

### 7.2 Integration Testing
- [x] Task: Add scheduler integration test with real database operations 1f03cb0
    - [x] Test scheduler full cycle with real DB
    - [x] Verify run_log table updates
    - [x] Test result structure validation

---

## Completion Criteria

This plan is complete when:
- [x] All phases complete
- [x] Coverage >95%
- [x] Mutation score >90%
- [x] Load tests passing
- [x] Property tests for all critical functions
- [x] TypeScript migration plan documented
- [x] TypeScript skeleton functional
- [x] All quality gates in CI

## Track History

- **2026-03-09**: Track started
- **2026-03-09**: Scheduler tests completed (100% coverage)
- **2026-03-09**: E2E CLI tests fixed
- **2026-03-09**: Track marked as "Substantial Progress" (83% coverage, 280 tests)
- **2026-03-09**: Review fixes applied (integration test added, status corrected)
- **2026-06-15**: Track completed (coverage >95%, mutation score >90%, load testing complete)
