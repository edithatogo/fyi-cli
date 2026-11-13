# Plan

## Phase 0: Baseline

- [x] Record full-suite baseline: wall time, collected/executed counts, slowest 25 durations. (On `chore/test-acceleration-322`: 844 passed, 2 skipped, 278.04s serial. Slowest: hypothesis redaction ~24s, e2e CLI tests ~3–8s each.)

## Phase 1: `slow` marker and fast local profile

- [x] Register the `slow` marker in `pyproject.toml`.
- [x] Tag resource-sensitive/file-mutating/benchmark tests `slow` at module level (`e2e_cli`, `e2e_workflows`, `fuzz`, `hypothesis`, `benchmarks`, `process_events_benchmark`).
- [x] Add a fast local profile (`-m "not slow"`) documented in docs/testing.md + AGENTS.md/CONTRIBUTING.md; CI keeps the full suite authoritative. (780 passed, 64 deselected, 38.65s.)

## Phase 2: pytest-xdist trial (bounded)

- [x] Add `pytest-xdist` to dev dependencies (locked; `uv.lock` regenerated).
- [x] Trial `-n 4` on this 10-core host; no resource-sensitive/file-mutating tests isolated (none found — all use per-test `tmp_path`). Serial phase = `slow` subset, optional.
- [x] Record before/after wall time and counts. (844 passed, 2 skipped: 100.54s with `-n 4` — 2.8×; combined fast `-n 4`: 27.22s — 10.2×.)

## Phase 3: pytest-testmon (opt-in, serial)

- [x] Add `pytest-testmon` to dev dependencies (locked; `uv.lock` regenerated).
- [x] Configure opt-in profile (`--testmon`); serial; `.testmondata` git-ignored; rebuild by deleting it (auto-rebuilt on dependency/inventory changes). Documented in docs/testing.md.
- [x] Record counts and wall time. (Unchanged code: 0 selected, 0.54s.)

## Phase 4: Bounded mutation triage (pytest-gremlins)

- [x] Trial pytest-gremlins (ad-hoc, `uv run --with pytest-gremlins`) on the bounded fail-closed/core-logic target set (`src/fyi_system/acquisition_receipts.py`).
- [x] Ordinary tests remain the promotion gate. (22/31 gremlins zapped, 71%; 149.8s; documented.)

## Phase 5: VCR evaluation (public, replay-safe HTTP only)

- [x] Evaluate VCR.py: **not adopted** — in-process transports (`httpx.MockTransport`) already keep tests deterministic with zero recording surface; no public replay-safe HTTP path needs cassettes; credential/PII recording risk outweighs benefit. Documented in docs/testing.md.

## Phase 6: Fixtures, coverage core, time freezing

- [x] Database fixtures: already per-test `tmp_path`; no shared/reused fixtures exist; `:memory:` cannot substitute where file-path/recovery semantics are tested. No change; documented.
- [x] `sysmon` coverage core: measured with `COVERAGE_CORE=sysmon` (coverage 7.14.3, Python 3.15.2, branch coverage) — compatible on the monitor module (98%, 12 passed). Kept opt-in, not forced into CI; branch-coverage/plugin interactions may differ elsewhere. Documented.
- [x] Freezegun: **not adopted** — clocks are injected (`utc_now()`); global freezing hazardous around asyncio/concurrency. Documented.
- [x] pytest-picked: **not adopted** — testmon provides dependency-aware selection. Documented.

## Phase 7: Verification and closure

- [x] Record before/after wall time and counts in docs/testing.md + metadata.json.
- [x] Confirm full clean-environment suite still required in CI (unchanged).
- [x] Document incompatibilities and exclusions (docs/testing.md).
- [x] Update metadata to completed.
- [ ] Close issue #322 after the PR merges.
