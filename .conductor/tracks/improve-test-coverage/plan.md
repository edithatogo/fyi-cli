# Plan: Improve Test Coverage

## Phase 1: Fix Linting Errors

- [x] Task: Run ruff --fix to auto-fix 4 import errors
- [x] Task: Refactor cli.py to break chained semicolon statements (131 errors)
- [x] Task: Verify ruff check passes with 0 errors

## Phase 2: Fix Type Checking

- [x] Task: Install type stubs: `pip install types-requests`
- [x] Task: Fix db.py type annotation for int | None
- [x] Task: Add type ignore for feedparser (no stubs available)
- [x] Task: Verify mypy passes with 0 errors

## Phase 3: Add CLI Tests

- [x] Task: Create tests/test_cli.py
- [x] Task: Test CLI argument parsing (build_cli_parser)
- [x] Task: Test CLI commands (cmd_feed, cmd_import, etc.)
- [x] Task: Test CLI main function with various arguments
- [x] Task: Achieve 73% coverage for cli.py (up from 0%)

## Phase 4: Add Monitor Tests

- [x] Task: Create tests/test_monitor.py
- [x] Task: Test ingest_feed function
- [x] Task: Test reconcile_events function
- [x] Task: Achieve 71% coverage for monitor.py (up from 0%)

## Phase 5: Add Scheduler Tests

- [x] Task: Create tests/test_scheduler.py
- [x] Task: Test run_scheduler function
- [x] Task: Test run_cycle function
- [x] Task: Achieve 56% coverage for scheduler.py (up from 0%)

## Phase 6: Improve Webapp Tests

- [ ] Task: Add tests for uncovered webapp routes
- [ ] Task: Add tests for form handling
- [ ] Task: Improve coverage from 53% to 70%+

## Phase 7: Improve Security Tests

- [ ] Task: Add tests for sanitize_payload
- [ ] Task: Add tests for secure_write_text
- [ ] Task: Add tests for ensure_private_path
- [ ] Task: Improve coverage from 74% to 80%+

## Phase 8: Improve Dashboard Tests

- [ ] Task: Add tests for uncovered dashboard functions
- [ ] Task: Improve coverage from 73% to 80%+

## Phase 9: Final Verification

- [x] Task: Run full test suite
- [x] Task: Run coverage report
- [x] Task: Verify 75% coverage achieved (up from 62%)
- [x] Task: Run ruff check (0 errors)
- [x] Task: Run mypy (0 errors)
- [ ] Task: Commit all changes

---

## Completion Criteria

This plan is complete when:
- [x] All linting errors fixed (135 → 0) ✅
- [x] All type checking errors fixed (3 → 0) ✅
- [x] Test coverage improved (62% → 75%) ✅
- [x] All tests passing (30 → 75 tests) ✅
- [ ] Coverage >= 80% (currently 75%, need 5% more)
- [ ] cli.py coverage >= 80% (currently 73%)
- [ ] monitor.py coverage >= 80% (currently 71%)
- [ ] scheduler.py coverage >= 80% (currently 56%)
- [ ] webapp.py coverage >= 70% (currently 53%)
- [ ] security.py coverage >= 80% (currently 74%)
