# Track Specification: Research-Grade Quality

## Overview

This track elevates the FYI Request System to research-grade quality standards suitable for handling sensitive official information requests with maximum reliability and correctness guarantees.

## Quality Standards

### 1. Test Coverage: >95%
**Current: 80%** | **Target: >95%**

Research tools require near-complete coverage because:
- Handling sensitive government data
- Privacy-critical operations (TOR, proxy, redaction)
- Legal compliance requirements
- Reproducibility of research workflows

**Modules requiring attention:**
- `webapp.py`: 53% → 95% (283 statements, need ~119 more covered)
- `scheduler.py`: 56% → 95% (27 statements, need ~11 more covered)
- `reporting.py`: 89% → 95% (294 statements, need ~18 more covered)
- All other modules: Maintain >95%

### 2. Mutation Testing
**Current: None** | **Target: >90% mutation score**

Mutation testing ensures tests actually verify behavior, not just execute code.

**Tools:**
- **cosmic-ray** (Python mutation testing)
- **mutmut** (alternative mutation tester)

**Process:**
1. Install mutation testing tool
2. Run initial mutation analysis
3. Fix tests that don't catch mutants
4. Achieve >90% mutation score

**Mutation Types to Test:**
- Arithmetic operator changes
- Boolean value changes
- Condition boundary changes
- Return value changes
- Statement deletions

### 3. Load Testing
**Current: None** | **Target: Documented performance characteristics**

Research workflows may process hundreds of requests simultaneously.

**Test Scenarios:**
- Concurrent API requests (10, 50, 100, 500 concurrent)
- Database write load (1000 requests/minute)
- Feed ingestion throughput
- Dashboard generation under load
- Memory usage under sustained operation

**Tools:**
- **locust** (Python load testing)
- **pytest-benchmark** (micro-benchmarks)
- **memory_profiler** (memory tracking)

**Performance Targets:**
- Request ingestion: <100ms per request
- Dashboard generation: <2s for 1000 requests
- Feed parsing: <500ms per feed
- Memory: <500MB under normal load

### 4. Hypothesis Testing (Property-Based Testing)
**Current: None** | **Target: Property tests for all critical functions**

Property-based testing finds edge cases by generating thousands of test cases automatically.

**Tools:**
- **hypothesis** (Python property-based testing)

**Critical Properties to Test:**
- **Redaction**: Any email address gets redacted, regardless of format
- **URL parsing**: Any valid URL with query params gets sanitized
- **State machine**: Request status transitions are always valid
- **Data integrity**: Export/import round-trip preserves all data
- **Privacy**: No PII leaks in logs or outputs

**Example Properties:**
```python
@given(emails())
def test_redact_any_email(email):
    result = redact_text(f"Contact {email}")
    assert email not in result

@given(dictionaries(text(), text()))
def test_sanitize_preserves_structure(payload):
    result = sanitize_payload(payload)
    assert set(result.keys()) == set(payload.keys())
```

### 5. TypeScript Migration Preparation
**Current: Python** | **Target: TypeScript migration plan**

Long-term goal is to migrate to TypeScript for:
- Better type safety (compile-time vs runtime)
- Single binary distribution (via Bun/Deno)
- Better MCP server ecosystem support
- Easier CLI distribution

**Migration Strategy:**
1. **Phase 1**: Create TypeScript CLI skeleton with Commander
2. **Phase 2**: Port core domain logic (FYI API client)
3. **Phase 3**: Port privacy/security modules
4. **Phase 4**: Port webapp/dashboard
5. **Phase 5**: Full integration testing
6. **Phase 6**: Deprecate Python version

**TypeScript Tech Stack:**
- **Runtime**: Bun or Deno (single binary, TypeScript native)
- **CLI**: Commander.js
- **HTTP**: Undici or native fetch
- **Database**: Better-sqlite3 or SQLite async
- **Testing**: Vitest (fast, TypeScript native)
- **Type Checking**: TypeScript strict mode

**Interim Approach:**
- Keep Python implementation stable
- Create TypeScript implementation in parallel
- Run both implementations side-by-side
- Validate TypeScript against Python test suite
- Migrate users gradually

## Success Criteria

### Coverage (>95%)
- [ ] webapp.py: 53% → 95%
- [ ] scheduler.py: 56% → 95%
- [ ] reporting.py: 89% → 95%
- [ ] All modules: >95%
- [ ] Overall: >95%

### Mutation Testing (>90%)
- [ ] Mutation testing framework installed
- [ ] Initial mutation analysis run
- [ ] Tests improved to catch mutants
- [ ] Mutation score >90%
- [ ] Mutation testing in CI pipeline

### Load Testing
- [ ] Load testing framework installed
- [ ] Baseline performance documented
- [ ] Performance targets defined
- [ ] Load tests in CI pipeline
- [ ] Performance regression detection

### Hypothesis Testing
- [ ] Hypothesis library installed
- [ ] Property tests for redaction functions
- [ ] Property tests for URL parsing
- [ ] Property tests for data integrity
- [ ] Property tests for privacy guarantees
- [ ] 1000+ generated test cases per property

### TypeScript Migration
- [ ] TypeScript project skeleton created
- [ ] Commander CLI implemented
- [ ] Core types defined
- [ ] FYI API client ported
- [ ] Test suite ported
- [ ] Migration documentation written

## Dependencies

### Python Testing
- pytest (installed)
- pytest-cov (installed)
- hypothesis (installed - need to use)
- cosmic-ray OR mutmut (new)
- locust (new)
- pytest-benchmark (new)
- memory-profiler (new)

### TypeScript
- Bun or Deno runtime
- Commander.js
- Vitest
- TypeScript strict mode

## Risks

### Coverage
- **Risk**: webapp.py is large (283 statements)
- **Mitigation**: Focus on critical paths first, accept lower coverage for UI rendering code

### Mutation Testing
- **Risk**: Mutation testing is slow
- **Mitigation**: Run on CI only, use sampling for development

### Load Testing
- **Risk**: Requires external services (FYI API)
- **Mitigation**: Mock external services, use recorded responses

### TypeScript Migration
- **Risk**: Large effort, duplicates work
- **Mitigation**: Incremental migration, keep Python stable

## Timeline Estimate

- **Coverage to 95%**: 2-3 days
- **Mutation testing**: 1-2 days
- **Load testing**: 1-2 days
- **Hypothesis testing**: 2-3 days
- **TypeScript skeleton**: 1-2 days
- **Total**: 7-12 days

## Priority

**HIGH** - Research-grade quality is essential for:
- Legal compliance
- Privacy guarantees
- Research reproducibility
- User trust
