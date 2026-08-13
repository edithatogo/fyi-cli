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
