# Track Specification: Improve Test Coverage

## Overview

This track focuses on improving the overall quality of the Python implementation by:
1. Increasing test coverage from 62% to 80%+
2. Fixing all ruff linting errors (135 errors)
3. Adding comprehensive tests for uncovered modules

## Current State

### Test Coverage: 62%
- **Total Statements:** 1239
- **Covered:** 773
- **Missing:** 466

### Modules with 0% Coverage
1. **cli.py** (167 statements) - CLI argument parsing and commands
2. **monitor.py** (34 statements) - Feed monitoring functionality
3. **scheduler.py** (27 statements) - Request scheduling

### Modules Below 80% Coverage
- **webapp.py:** 53% (283 stmts, 134 missing)
- **security.py:** 74% (137 stmts, 35 missing)
- **dashboard.py:** 73% (33 stmts, 9 missing)

### Linting Issues: 135 errors
- **E702:** 131 errors - Multiple statements on one line (cli.py)
- **F811:** 4 errors - Redefinition of unused imports (reporting.py)

### Type Checking Issues: 3 errors
- **db.py:** Type incompatibility with `int | None`
- **monitor.py:** Missing feedparser stubs
- **fetch.py:** Missing requests stubs

## Goals

1. **Test Coverage:** Increase from 62% to 80%+
2. **Linting:** Fix all 135 ruff errors
3. **Type Checking:** Resolve all 3 mypy errors
4. **Test Count:** Add 20+ new tests

## Scope

### In Scope
- Fix ruff errors in cli.py and reporting.py
- Create test_cli.py with tests for all CLI commands
- Create test_monitor.py with tests for monitoring
- Create test_scheduler.py with tests for scheduling
- Add tests for webapp.py routes and forms
- Add tests for security.py functions
- Install missing type stubs
- Fix type annotations in db.py

### Out of Scope
- Refactoring core functionality
- Adding new features
- Performance optimization
- Rust implementation planning

## Success Criteria

- [ ] All 135 ruff errors fixed
- [ ] All 3 mypy errors resolved
- [ ] Test coverage >= 80%
- [ ] All tests passing (50+ tests total)
- [ ] cli.py coverage >= 80%
- [ ] monitor.py coverage >= 80%
- [ ] scheduler.py coverage >= 80%
- [ ] webapp.py coverage >= 70%
- [ ] security.py coverage >= 80%

## Technical Approach

### 1. Fix Linting Errors First
- Run `ruff check --fix` for auto-fixable errors
- Manually refactor cli.py to break chained statements
- Fix duplicate imports in reporting.py

### 2. Add Module Tests
- **cli.py:** Test argument parsing, test each command function
- **monitor.py:** Test feed monitoring, test snapshot detection
- **scheduler.py:** Test scheduling logic, test interval timing

### 3. Improve Existing Tests
- Add edge cases to webapp tests
- Add security function tests
- Add dashboard rendering tests

### 4. Install Type Stubs
```bash
pip install types-requests types-feedparser
```

## Dependencies

- pytest (installed)
- pytest-cov (installed)
- ruff (installed)
- mypy (installed)

## Risks

- **Risk:** CLI testing may require mocking sys.argv
  - **Mitigation:** Use pytest's monkeypatch or click.testing
- **Risk:** Webapp testing may require test client
  - **Mitigation:** Use Python's built-in test client or mock

## Future Work

After this track:
- Consider integration tests
- Consider end-to-end tests
- Maintain 80% coverage for new code
