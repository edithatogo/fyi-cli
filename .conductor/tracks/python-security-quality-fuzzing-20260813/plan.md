# Plan

- [x] Register bounded receipt, CDX, Wayback replay, and redaction targets.
- [x] Add deterministic seed corpora without production data.
- [x] Add an explicit Hypothesis pull-request gate.
- [x] Add least-privilege PR, scheduled, and manual Atheris workflow contracts.
- [x] Add Python security and quality checks.
- [x] Upgrade vulnerable Python, Rust, dashboard, and documentation dependency
  families; retain zero-vulnerability npm and pip-audit results locally.
- [x] Migrate the dashboard route contract and deterministic Web Storage test
  setup required by the secured Next.js and Vitest versions.
- [x] Add offline harness and workflow regression tests.
- [x] Run local verification: 34 Hypothesis tests and 12 focused contract tests
  passed; scoped Ruff lint/format and strict basedpyright passed; `uv lock
  --check`, `actionlint`, and `git diff --check` passed.
- [ ] Obtain a successful hosted PR run before marking the track complete.
- [ ] Confirm the upgraded Rust graph compiles and passes `cargo audit` on the
  hosted Linux runner; the local Windows SDK/linker environment cannot provide
  authoritative compile evidence.

## Dependency maintenance — 13 November 2026

- [x] Recheck the current Python dependency baseline: the urllib3-updated
  Python 3.12 lock reports 37 advisories across eight packages.
- [x] Refresh affected Python dependency families and retain FastMCP 3 / MCP 1
  API families. Include pending urllib3 and PyJWT fixes. Propose Python 3.11+
  because the secured dnspython release excludes the former Python 3.10 floor.
- [x] Remove unused Safety from the development extras after confirming there
  are no script/workflow/test invocations. Preserve the enforced pip-audit gate;
  Safety 3.7 requires vulnerable Filelock versions, while falling back to Safety
  3.2 reintroduces the obsolete urllib3 branch on Python 3.10.
- [ ] Obtain maintainer agreement to Python 3.11+, or a reviewed secured
  dnspython backport for Python 3.10. Keep the PR draft until this is resolved.
- [x] Verify the frozen environment and repeat the dependency audit: no known
  vulnerabilities in the proposed Python 3.11+ graph on the Python 3.12 audit.
- [x] Run existing client, API, credentials, encryption, parser and MCP contract
  tests with sockets disabled and an isolated test keyring.
  All 157 selected tests passed on both Python 3.11.18 and Python 3.12.14.
- [ ] Obtain successful hosted checks before merge. The initial full local
  suite attempted a live scheduler request and is not completion evidence.

The later `e056640` main snapshot contained 48 advisories across 12 packages.
The maintenance refresh preserves its dependency changes while upgrading the
affected families. Evidence and the unresolved runtime compatibility decision
are recorded in `maintenance-2026-11-13.md`.
