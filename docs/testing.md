# Testing profiles and bounded acceleration

Issue: [fyi-cli #322](https://github.com/edithatogo/fyi-cli/issues/322). Implements the
owner-wide test-acceleration audit (2026-08-26) for this repository.

**The full clean-environment CI suite is authoritative.** Selective profiles below
are local feedback only and never replace test, coverage, mutation, rights or
publication gates. A separate bounded CI regression checks keyring isolation with
four workers; the full-suite merge gate remains serial and unfiltered.

## Isolated execution environment

The credential, encryption and keyring-isolation test modules use the real
`PlaintextKeyring` backend at a unique per-test `tmp_path / "keyring.cfg"`. The
function-scoped fixture in `tests/conftest.py` restores the previous backend after
the test and its fixture cleanup. Each xdist worker therefore writes different
files; tests never share the developer's persistent credential store.

Setting `PYTHON_KEYRING_BACKEND=keyrings.alt.file.PlaintextKeyring` alone is **not**
isolation: without the fixture it selects one shared file. The fixture's explicit
module inventory keeps metadata-only CI, which installs pytest alone, independent
of credential dependencies. New tests that use the real keyring must be added to
that inventory. Production keyring configuration is unchanged.

The existing CI environment variable remains compatible:

```bash
PYTHON_KEYRING_BACKEND=keyrings.alt.file.PlaintextKeyring uv run pytest
```

## Profiles

| Profile | Command | Use |
|---|---|---|
| Full (authoritative) | `uv run pytest` | Serial full-suite CI and merge gate |
| Fast | `uv run pytest -m "not slow"` | Local iteration |
| Fast + parallel | `uv run pytest -m "not slow" -n 4` | Local iteration |
| Parallel (all) | `uv run pytest -n 4` | Local verification |
| Serial feedback | `uv run pytest --testmon` | Opt-in changed-code feedback |
| Mutation trial | `uv run --with pytest-gremlins pytest --gremlins` | Ad-hoc, bounded target set |

`-n 4` is the bounded local worker count (10-core development host, leaving
headroom for Rust builds and the editor). CI uses four workers only for the
explicit keyring regression, not to replace the full suite:

```bash
uv run pytest -q -n 4 tests/test_credentials.py tests/test_encryption.py tests/test_keyring_isolation.py
```

## The `slow` marker

`slow` is registered in `pyproject.toml` and applied at module level to:

- `tests/test_e2e_cli.py`, `tests/test_e2e_workflows.py`: subprocess CLI/workflows.
- `tests/test_fuzz.py`, `tests/test_hypothesis.py`: property/fuzz tests.
- `tests/test_benchmarks.py`, `tests/test_process_events_benchmark.py`: benchmarks.

The full CI suite does not filter these markers. The local fast profile excludes
them; it is not evidence that the omitted tests passed.

## pytest-testmon (opt-in only)

Run serially, never as a CI merge gate. Rebuild its local dependency database after
dependency or test-inventory changes, or when selection appears stale, by deleting
`.testmondata` and rerunning. Do not commit `.testmondata` (git-ignored).

## Decisions

- **pytest-xdist**: bounded local `-n 4`. Real credential storage is isolated per
  test, not merely assumed safe because SQLite uses `tmp_path`.
- **pytest-testmon**: opt-in serial local feedback, never a merge gate.
- **pytest-gremlins**: bounded receipt-boundary trial; ordinary tests remain the
  promotion gate.
- **VCR.py**: not adopted. Existing in-process transports provide deterministic
  HTTP fixtures without a new credential or personal-data recording surface.
- **Database fixtures**: preserve per-test `tmp_path` databases where file-path
  and recovery behavior is tested; in-memory databases are not equivalent there.
- **sysmon coverage core**: measured experimentally; remains opt-in because branch
  coverage and plugin interactions need their own compatibility evidence.
- **Freezegun**: not adopted; explicit injectable clocks are preferred to global
  time freezing around asynchronous or concurrent code.
- **pytest-picked**: not adopted; testmon already supplies dependency-aware selection.

## Historical timing evidence (2026-11-13)

The following author-recorded measurements predate the keyring-isolation fix and
additional receipt regressions. They are not final-head timing claims or proof
that sharing a persistent keyring is safe. Environment: macOS 10-core aarch64,
Python 3.15.2, plaintext keyring backend.

| Run | Result | Wall time |
|---|---|---|
| Full serial (baseline) | 844 passed, 2 skipped | 278.04s |
| Fast (`-m "not slow"`) | 780 passed, 2 skipped, 64 deselected | 38.65s |
| xdist `-n 4` full | 844 passed, 2 skipped | 100.54s |
| xdist `-n 4` fast | 780 passed, 2 skipped, 64 deselected | 27.22s |
| pytest-gremlins (receipt boundary) | 22/31 killed, 9 survived | 149.8s |
| sysmon (monitor module, branch) | 98% coverage | 3.96s |
| pytest-testmon (unchanged code) | 0 selected | 0.54s |

See `.conductor/tracks/test-acceleration-20261113/metadata.json` for the historical
measurements and `keyring-isolation-review.md` in that track for the executed
regression evidence. Final-head CI determines merge readiness.
