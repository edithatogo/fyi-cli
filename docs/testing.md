# Testing profiles and bounded acceleration

Issue: [fyi-cli #322](https://github.com/edithatogo/fyi-cli/issues/322). Implements the
owner-wide test-acceleration audit (2026-08-26) for this repository.

**The full clean-environment CI suite is authoritative.** The profiles below are local
feedback only; they never gate merges, and they never weaken test, coverage, mutation,
rights, or publication gates.

## Required execution environment

CI sets `PYTHON_KEYRING_BACKEND=keyrings.alt.file.PlaintextKeyring`. Without it,
`tests/test_encryption.py::TestSetupEncryption::test_setup_and_verify` fails on macOS with
keychain error `-25244`. Reproduce CI locally with:

```bash
PYTHON_KEYRING_BACKEND=keyrings.alt.file.PlaintextKeyring uv run pytest
```

## Profiles

| Profile | Command | Use |
|---|---|---|
| Full (authoritative) | `PYTHON_KEYRING_BACKEND=keyrings.alt.file.PlaintextKeyring uv run pytest` | CI and merge gate |
| Fast | `uv run pytest -m "not slow"` | local iteration |
| Fast + parallel | `uv run pytest -m "not slow" -n 4` | local iteration |
| Parallel (all) | `uv run pytest -n 4` | local verification |
| Serial feedback | `uv run pytest --testmon` | opt-in changed-code feedback |
| Mutation trial | `uv run --with pytest-gremlins pytest --gremlins` | ad-hoc, bounded target set |

`-n 4` is the bounded worker count for this repository (10-core development host; leaves
headroom for the Rust build and the editor). Do not raise it in CI; CI does not use `-n`.

## The `slow` marker

`slow` is registered in `pyproject.toml` and applied at module level (`pytestmark`) to the
long-running/resource-sensitive files:

- `tests/test_e2e_cli.py`, `tests/test_e2e_workflows.py` — subprocess CLI/workflow tests
- `tests/test_fuzz.py`, `tests/test_hypothesis.py` — hypothesis property/fuzz tests
- `tests/test_benchmarks.py`, `tests/test_process_events_benchmark.py` — benchmarks

CI runs the full suite (markers do not filter CI). Local fast profile skips them.

## pytest-testmon (opt-in only)

Serial (`-n 0` implied), never in CI. The dependency database is rebuilt automatically
when dependencies or test inventory change; if selection looks wrong, delete `.testmondata`
and re-run. Do not commit `.testmondata` (git-ignored).

## Decisions (evaluated, adopted or rejected)

- **pytest-xdist** — adopted for local runs; bounded `-n 4`; serial `slow` phase optional.
  CI unchanged.
- **pytest-testmon** — adopted as opt-in local profile; never a merge gate.
- **pytest-gremlins** — trial on the bounded fail-closed/core-logic target set
  (`src/fyi_system/acquisition_receipts.py`); ordinary tests remain the promotion gate.
- **VCR.py cassettes** — **not adopted**. The suite already uses in-process transports
  (`httpx.MockTransport`) everywhere, which keeps tests deterministic with zero secret
  surface. Cassettes add PII/credential recording risk for no coverage gain; there is no
  public replay-safe HTTP path in this suite that needs them.
- **Immutable database fixtures / in-memory SQLite** — the suite already isolates every
  database test with a per-test `tmp_path` database; there are no shared, reused fixtures.
  Where file-path/recovery semantics are under test, `:memory:` cannot substitute. No
  change; documented exclusion.
- **sysmon coverage core** — measured via `COVERAGE_CORE=sysmon`; see evidence in the
  track metadata. Not forced into CI; branch-coverage and plugin interactions make it
  opt-in for now.
- **Freezegun** — **not adopted**. The codebase injects clocks (`utc_now()`), so global
  time freezing is unnecessary and would be hazardous around asyncio/concurrency.
- **pytest-picked** — **not adopted**. testmon already provides dependency-aware selection;
  picked would only add ordering of directly changed files and is redundant.

## Before/after evidence (2026-11-13, this branch)

Environment: macOS 10-core aarch64, Python 3.15.2, `PYTHON_KEYRING_BACKEND=keyrings.alt.file.PlaintextKeyring`.

| Run | Result | Wall time |
|---|---|---|
| Full serial (baseline) | 844 passed, 2 skipped | 278.04s |
| Fast (`-m "not slow"`) | 780 passed, 2 skipped, 64 deselected | 38.65s (−86%) |
| xdist `-n 4` full | 844 passed, 2 skipped | 100.54s (2.8× faster) |
| xdist `-n 4` fast | 780 passed, 2 skipped, 64 deselected | 27.22s (10.2× faster) |
| pytest-gremlins (receipt boundary) | 22/31 gremlins zapped (71%), 9 survived | 149.8s |
| `COVERAGE_CORE=sysmon` (monitor module, branch) | 98% coverage, compatible | 3.96s |
| pytest-testmon (unchanged code) | 0 selected | 0.54s |

Full CI suite remains the merge gate; nothing in CI changed to use any of these profiles.

Recorded in `.conductor/tracks/test-acceleration-20261113/metadata.json`.
