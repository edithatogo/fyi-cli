# Python dependency security maintenance — 13 November 2026

## State and compatibility decision

This is a draft proposal based on main `e056640851fd4e4284ef517b4a4641653c8aac53`.
Do not merge or enable auto-merge until the maintainer agrees to the Python 3.11+
minimum, or provides a reviewed secure alternative for Python 3.10.

The current main lock reported 48 advisories across 12 packages on Python 3.12.
A coordinated update removes those findings. A separate audit of the former
Python 3.10 resolution exposed CVE-2026-114605 in dnspython 2.8.0; the upstream
fixed 2.9+ family requires Python 3.11+. The application minimum and contributor
guide therefore explicitly propose Python 3.11+. Rust remains the primary
implementation; this compatibility decision concerns the legacy Python package.

The unused Safety development dependency required vulnerable Filelock versions.
No scripts, workflows or tests invoke it. Remove that dependency while retaining
the existing enforced pip-audit workflow without exclusions. The universal lock
uses secured Filelock, dnspython, FastMCP 3, MCP 1, authentication, certificate,
package-manager and telemetry dependency families.

## Local evidence

- Frozen installation succeeds with the proposed graph.
- All 157 selected client/API, credentials, encryption, MCP contract, receipt,
  Hypothesis, fuzz-target and workflow-contract tests pass on Python 3.11.18.
- The same 157 tests pass on Python 3.12.14 with the equivalent selected graph.
- Tests run with `pytest --disable-socket`, an isolated `XDG_DATA_HOME`, and the
  repository's test-only keyring backend; no live FYI/Alaveteli traffic is used.
- Export the entire locked development graph with `uv export --frozen --extra
  dev --no-emit-project`; pip-audit of the explicit pinned packages finds no
  known vulnerabilities on Python 3.12. `--no-deps --disable-pip` avoids a second
  resolution because the export already contains all pinned transitive packages.
- `uv lock --check`, `uv pip check`, the existing scoped Ruff lint/format checks,
  and scoped Basedpyright checks pass.

The initial full-suite attempt was interrupted when existing scheduler tests
attempted live networking. It is not recorded as a full-suite pass. Hosted PR
checks remain required and the track remains in progress.

## Rollback

Reverting this proposal restores the former dependency graph and Python floor,
including known vulnerabilities. Use a reviewed secure Python 3.10 backport if
that runtime must remain supported; do not add advisory exclusions to obtain a
passing security check.
