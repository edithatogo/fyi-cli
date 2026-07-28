# FYI Request System - Testing Strategy

## Executable harness contract

The repository maintains an explicit 13-layer harness inventory covering unit,
integration, end-to-end, smoke/system, mutation, property-based, edge,
performance, security, compatibility, usability, regression, and sanity tests.
Validate the inventory with:

```bash
uv run python scripts/verify_test_harness.py
uv run python scripts/verify_test_harness.py --run
uv run python scripts/verify_test_harness.py --run --run-expensive
```

The live smoke layer remains disabled unless `FYI_LIVE_SMOKE=1` is explicitly
set. Rust reusable-library line coverage is a hard 90% CI and Codecov gate;
interactive entrypoints, the TUI, Tor transport, and static jurisdiction
catalog are validated by smoke/system tests and excluded from this line
metric. An upload below the gate fails the workflow. Weekly/manual mutation analysis runs through
`cargo-mutants` in `.github/workflows/mutation.yml`.

## Overview

This document describes the comprehensive testing strategy for the FYI Request System, a research-grade tool for managing official information requests.

## Testing Pyramid

```
                    ┌─────────────┐
                    │   E2E/Load  │  ← Fewest tests, slowest
                   ─┴─────────────┴─
                  │ Integration Tests │
                 ─┴───────────────────┴─
                │   Unit Tests (155+)   │  ← Most tests, fastest
               ─┴───────────────────────┴─
              │  Property-Based (19+)   │
             ─┴─────────────────────────┴─
            │    Mutation Testing       │  ← Quality verification
           ─┴───────────────────────────┴─
          │      Security Testing       │
         ─┴─────────────────────────────┴─
        │       Performance Profiling    │  ← Baseline metrics
       ─┴────────────────────────────────┴─
```

## 1. Unit Testing ✅

**Framework:** pytest  
**Count:** 155+ tests  
**Coverage gate:** Rust reusable-library logic must stay **>90%** line coverage in
CI/Codecov via `cargo llvm-cov --fail-under-lines 90`; branch uploads below that
threshold fail the build.

### Test Files
- `tests/test_cli.py` - CLI argument parsing and commands
- `tests/test_security.py` - Redaction, sanitization, secure operations
- `tests/test_dashboard.py` - Dashboard generation
- `tests/test_monitor.py` - Feed monitoring
- `tests/test_scheduler.py` - Scheduling logic
- `tests/test_fyi.py` - FYI API integration
- `tests/test_webapp_forms.py` - Web form handling
- `tests/test_reporting.py` - Report generation

### Running Unit Tests
```bash
# Run all tests
pytest tests/ -v

# Run with coverage
pytest tests/ --cov=fyi_system --cov-report=html

# Run specific test file
pytest tests/test_security.py -v
```

## 2. Property-Based Testing (Hypothesis) ✅

**Frameworks:** hypothesis + proptest  
**Coverage:** Python generative tests in `tests/test_hypothesis.py` and
`tests/test_fuzz.py`, plus Rust property tests in
`crates/fyi-core/tests/property_tests.rs`

### What It Tests
- Email redaction (any format)
- Payload sanitization (any structure)
- State normalization (any input)
- URL handling (any valid URL)
- Data integrity (round-trip preservation)

### Bugs Found
Hypothesis automatically found **3 real bugs**:
```python
# Bug 1: {*@A.AC not redacted
# Bug 2: 0*@A.aD not redacted
# Bug 3: {@A.AL sanitization missed

# Fixed by changing regex from:
EMAIL_RE = re.compile(r'([A-Za-z0-9._%+-]+)@...')
# To:
EMAIL_RE = re.compile(r'([^@\s]+)@...')
```

### Running Hypothesis Tests
```bash
pytest tests/test_hypothesis.py -v
```

## 3. Mutation Testing ✅

**Tools:** `cargo-mutants` + custom Python mutation harness  
**Status:** Scheduled/manual workflow in `.github/workflows/mutation.yml`

### What It Does
Mutation testing introduces small bugs (mutants) into your code and verifies that tests catch them.

### Mutation Types
- Boolean mutations (True↔False)
- Comparison mutations (==, !=, <, >)
- Arithmetic mutations (+1↔-1)
- None check mutations (is None↔is not None)
- Return value mutations

### Running Mutation Tests
```bash
# Custom Windows-compatible script
python mutation_test.py

# With cosmic-ray (requires config)
cosmic-ray init cosmic-ray.toml session.sqlite
cosmic-ray exec session.sqlite
cosmic-ray report session.sqlite

# With mutmut (requires WSL on Windows)
mutmut run --paths-to-mutate src/fyi_system
```

### Mutation Score Targets
- **>90%**: Excellent (research-grade)
- **70-90%**: Good (production-ready)
- **<70%**: Needs improvement

## 4. Load Testing ✅

**Tool:** locust  
**Status:** Infrastructure ready

### What It Tests
- Concurrent user load (10-500 users)
- API response times under load
- Web dashboard performance
- Database query performance

### Running Load Tests
```bash
# Start webapp first
fyi-system serve

# Run load test (headless mode)
locust -f tests/load_test_fyi.py --headless \
  -u 100 -r 10 --run-time 60s \
  --host http://localhost:8000

# Or use web UI
locust -f tests/load_test_fyi.py
# Open http://localhost:8089
```

### Performance Targets
- Request ingestion: <100ms per request
- Dashboard generation: <2s for 1000 requests
- Feed parsing: <500ms per feed
- Memory: <500MB under normal load

## 5. Security Testing ✅

**Tools:** bandit, safety  
**Status:** Passing (0 issues)

### What It Tests
- Security vulnerabilities in code
- Insecure coding patterns
- Dependency vulnerabilities
- Hardcoded secrets

### Running Security Tests
```bash
# Static analysis
bandit -r src/fyi_system -ll

# Dependency vulnerabilities
safety check
safety check --full-report
```

### Security Requirements
- No hardcoded secrets
- All user input validated
- SQL injection prevented
- XSS protection enabled
- Secure file permissions

## 6. Performance Profiling ✅

**Tools:** cProfile, snakeviz, memory-profiler, line-profiler  
**Status:** Baseline established

### Performance Baselines

| Operation | Time | Notes |
|-----------|------|-------|
| Redact text (4000 calls) | 562ms | Regex-heavy |
| Sanitize payload (3000 calls) | 680ms | Recursive |
| Normalize state (140000 calls) | 208ms | Fast |
| Insert 100 requests | 1192ms | DB commits |
| Query all requests (100x) | 87ms | Cached |
| Generate attention report (10x) | 687ms | Complex |

### Running Profiling
```bash
# Profile specific function
python -m cProfile -o profile.stats tests/profile_fyi.py

# Visualize results
snakeviz profile.stats

# Memory profiling
python -m memory_profiler src/fyi_system/security.py

# Line-by-line profiling
python -m line_profiler src/fyi_system/redact_text.py
```

### Performance Optimization Targets
- Reduce regex operations in redact_text
- Batch database commits
- Cache frequently accessed data
- Use connection pooling

## 7. Integration Testing ✅

**Status:** Completed (Robust coverage of end-to-end web client simulations)

### What It Tests
- End-to-end request lifecycle via HTTP mock handlers (dashboard -> request creation -> status updating -> timeline check)
- Integration flows (create -> update -> export bundle)
- Filter & search integrations (search -> view detail -> status update)

### Running Integration Tests
```bash
pytest tests/test_webapp.py -k "Integration" -v
```

## Test Coverage Goals

| Module | Current | Target | Status |
|--------|---------|--------|--------|
| cli.py | 83% | 95% | ⚠️ Needs work |
| monitor.py | 85% | 95% | ⚠️ Close |
| scheduler.py | 56% | 95% | ❌ Needs work |
| reporting.py | 89% | 95% | ⚠️ Close |
| webapp.py | 96% | 95% | ✅ Achieved |
| security.py | 88% | 95% | ⚠️ Close |
| dashboard.py | 91% | 95% | ⚠️ Close |
| **Overall** | **95%** | **95%** | ✅ Achieved |

## Continuous Integration

### Pre-commit Hooks
```bash
# Install pre-commit
pip install pre-commit
pre-commit install

# Runs on every commit:
- ruff check (linting)
- ruff format (formatting)
- mypy (type checking)
- pytest (unit tests)
```

### CI Pipeline (GitHub Actions)
```yaml
# .github/workflows/ci.yml
name: CI
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-python@v4
        with:
          python-version: '3.10'
      - run: pip install -e ".[dev]"
      - run: pytest tests/ --cov=fyi_system --cov-fail-under=95
      - run: bandit -r src/fyi_system
      - run: safety check
      - run: python mutation_test.py
```

## Test Data Management

### Test Fixtures
```python
# conftest.py
@pytest.fixture
def sample_request():
    return {
        'authority_slug': 'test',
        'title': 'Test Request',
        'body': 'Test body'
    }

@pytest.fixture
def temp_db(tmp_path):
    db_path = tmp_path / "test.db"
    init_db(str(db_path))
    return db_path
```

### Test Data Rules
- Never commit real request data
- Use generated/test data only
- Sanitize any exported data
- Clean up temp files after tests

## Recommended Testing Workflow

1. **Write tests first** (TDD)
2. **Run unit tests** frequently
3. **Run hypothesis tests** before commits
4. **Run mutation tests** weekly
5. **Run load tests** before releases
6. **Run security tests** in CI

## Additional Recommended Tools

### Not Yet Implemented
- [ ] **Fuzz Testing**: python-afl, honggfuzz
- [ ] **Contract Testing**: pact-python
- [ ] **Visual Regression**: pytest-snapshot
- [ ] **API Testing**: schemathesis (OpenAPI)
- [ ] **Chaos Engineering**: chaos-monkey

### Future Enhancements
- [ ] Docker-based test environments
- [ ] Parallel test execution (pytest-xdist)
- [ ] Test impact analysis
- [ ] Automated test generation (pytest-robotframework)

## Resources

- [pytest documentation](https://docs.pytest.org/)
- [hypothesis documentation](https://hypothesis.readthedocs.io/)
- [cosmic-ray documentation](https://cosmic-ray.readthedocs.io/)
- [locust documentation](https://docs.locust.io/)
- [bandit documentation](https://bandit.readthedocs.io/)
