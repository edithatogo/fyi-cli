# Plan: Research-Grade Quality

## Phase 1: Coverage to 95%

### 1.1 Webapp Tests (53% → 95%)
- [ ] Task: Test all webapp routes (/, /requests, /request/<id>)
- [ ] Task: Test form handling (import, status update)
- [ ] Task: Test HTML rendering with data
- [ ] Task: Test error handling (404, 500)
- [ ] Task: Test JSON API endpoints
- [ ] Task: Test privacy redaction in web output

### 1.2 Scheduler Tests (56% → 95%)
- [ ] Task: Test scheduler loop logic
- [ ] Task: Test interval timing
- [ ] Task: Test once mode vs continuous mode
- [ ] Task: Test error handling in scheduler

### 1.3 Reporting Tests (89% → 95%)
- [ ] Task: Test attention report generation
- [ ] Task: Test triage report generation
- [ ] Task: Test export functions
- [ ] Task: Test handover document generation

### 1.4 Integration Tests
- [ ] Task: Test full request lifecycle
- [ ] Task: Test feed-to-request workflow
- [ ] Task: Test export-import round trip

## Phase 2: Mutation Testing

### 2.1 Setup
- [ ] Task: Install cosmic-ray or mutmut
- [ ] Task: Configure mutation testing
- [ ] Task: Run initial mutation analysis
- [ ] Task: Document baseline mutation score

### 2.2 Improve Mutation Score
- [ ] Task: Fix tests that don't catch mutants
- [ ] Task: Add assertions for edge cases
- [ ] Task: Test error conditions
- [ ] Task: Achieve >90% mutation score

### 2.3 CI Integration
- [ ] Task: Add mutation testing to CI pipeline
- [ ] Task: Set mutation score threshold
- [ ] Task: Configure scheduled mutation runs

## Phase 3: Load Testing

### 3.1 Setup
- [ ] Task: Install locust
- [ ] Task: Install pytest-benchmark
- [ ] Task: Install memory-profiler
- [ ] Task: Create load test scenarios

### 3.2 Performance Baselines
- [ ] Task: Benchmark request ingestion
- [ ] Task: Benchmark feed parsing
- [ ] Task: Benchmark dashboard generation
- [ ] Task: Benchmark export operations
- [ ] Task: Measure memory usage patterns

### 3.3 Load Scenarios
- [ ] Task: Test 10 concurrent requests
- [ ] Task: Test 100 concurrent requests
- [ ] Task: Test 500 concurrent requests
- [ ] Task: Test sustained load (1 hour)
- [ ] Task: Document performance characteristics

### 3.4 Optimization
- [ ] Task: Identify bottlenecks
- [ ] Task: Optimize slow paths
- [ ] Task: Add caching where appropriate
- [ ] Task: Document performance targets

## Phase 4: Hypothesis Testing

### 4.1 Setup
- [ ] Task: Verify hypothesis is installed
- [ ] Task: Learn hypothesis patterns
- [ ] Task: Create hypothesis test utilities

### 4.2 Property Tests - Security
- [ ] Task: Property test email redaction (any email format)
- [ ] Task: Property test URL sanitization
- [ ] Task: Property test bearer token redaction
- [ ] Task: Property test payload sanitization

### 4.3 Property Tests - Data Integrity
- [ ] Task: Property test export-import round trip
- [ ] Task: Property test database operations
- [ ] Task: Property test state transitions

### 4.4 Property Tests - Privacy
- [ ] Task: Property test no PII in logs
- [ ] Task: Property test no PII in errors
- [ ] Task: Property test file permissions

## Phase 5: TypeScript Migration Preparation

### 5.1 Research & Planning
- [ ] Task: Research Bun vs Deno vs Node.js
- [ ] Task: Evaluate Commander.js alternatives
- [ ] Task: Document TypeScript migration plan
- [ ] Task: Create migration timeline

### 5.2 TypeScript Skeleton
- [ ] Task: Create TypeScript project structure
- [ ] Task: Configure TypeScript strict mode
- [ ] Task: Set up Vitest testing
- [ ] Task: Create basic CLI with Commander

### 5.3 Core Types
- [ ] Task: Define TypeScript types for requests
- [ ] Task: Define TypeScript types for authorities
- [ ] Task: Define TypeScript types for events
- [ ] Task: Define TypeScript types for settings

### 5.4 FYI API Client (TypeScript)
- [ ] Task: Port FYI URL building
- [ ] Task: Port request ID extraction
- [ ] Task: Port API client functions
- [ ] Task: Write TypeScript tests

### 5.5 Validation
- [ ] Task: Run Python and TypeScript side-by-side
- [ ] Task: Compare outputs
- [ ] Task: Document differences
- [ ] Task: Create migration guide

## Phase 6: Documentation & CI

### 6.1 Documentation
- [ ] Task: Document testing strategy
- [ ] Task: Document performance characteristics
- [ ] Task: Document mutation testing process
- [ ] Task: Create quality assurance guide

### 6.2 CI/CD
- [ ] Task: Add coverage threshold to CI (>95%)
- [ ] Task: Add mutation testing to CI
- [ ] Task: Add load testing to CI
- [ ] Task: Add TypeScript build to CI

### 6.3 Quality Gates
- [ ] Task: Configure pre-commit hooks
- [ ] Task: Add coverage check to PRs
- [ ] Task: Add type checking to PRs
- [ ] Task: Add performance regression detection

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
- [ ] All phases complete
- [ ] Coverage >95%
- [ ] Mutation score >90%
- [ ] Load tests passing
- [ ] Property tests for all critical functions
- [ ] TypeScript migration plan documented
- [ ] TypeScript skeleton functional
- [ ] All quality gates in CI

## Track History

- **2026-03-09**: Track started
- **2026-03-09**: Scheduler tests completed (100% coverage)
- **2026-03-09**: E2E CLI tests fixed
- **2026-03-09**: Track marked as "Substantial Progress" (83% coverage, 280 tests)
- **2026-03-09**: Review fixes applied (integration test added, status corrected)
