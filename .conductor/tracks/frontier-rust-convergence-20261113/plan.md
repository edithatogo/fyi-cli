# Frontier architecture, single-developer engineering and runtime migration — implementation plan
Status: active planning; individual capability work requires evidence before marking complete.

## Dependencies and delivery order
1. **Reconcile baseline**: merge or reconcile PR #402; review `.conductor/tracks.md`, `.conductor/index.md` and active source-adapter-boundary #309; preserve statuses and provenance.
2. **Inventory and contract freeze**: generate machine-readable Python/Rust CLI/MCP/API/provider command surface inventory and downstream consumer graph. Capture golden outputs and legacy/version crosswalk. No deletion before fixture coverage.
3. **P0 source boundary**: finish #310 receipt schema/writer, #311 CDX parity, #312 Wayback verification, #313 probing/pacing, #314 historical indexes; run release-pinned fyi-archive parity and shadow verification.
4. **P0 Rust 1.99**: add per-crate MSRV and pinned execution toolchain after green 1.99.1 cross-platform matrix, then align workflows and docs. Validate `--locked`, release archives and GitHub Actions.
5. **P0 Python 3.15**: evaluate `uv lock --python 3.15` and `uv sync --python 3.15 --extra dev`, full tests, lint/typing and optional fuzz; resolve or isolate incompatible dependencies before updating `requires-python`, classifiers, pyright/Ruff targets and workflow versions. If blockers remain, leave legacy compatibility unchanged and record blocking dependency and workaround.
6. **P1 Rust migration**: port remaining functional Python paths per inventory, preserving typed contract and read-only shadow parity. Migrate three legacy entrypoints and publish deprecation plan; remove only after downstream evidence.
7. **P1 hardening**: add case machine, jurisdiction rules, provider SDK v2 and agent sandbox; create new tracks with full specs only after existing related tracks are deduplicated.
8. **P2 knowledge pipelines**: versioned provenance DAG and evaluated agent/graph analytics. Do not enable experimental functionality by default.
9. **Release and deprecation**: document breaking changes, produce migration artifacts and pin consumers, then retire obsolete Python package only after an explicit milestone.

## CI/harness matrix: required, targeted and scheduled
| Check | PR gate | Scheduled / release | Implementation constraint |
|---|---|---|---|
| Rust format, Clippy all targets/features, build, unit/integration | required | release multi-OS | `-D warnings`; audit no implicit target omissions |
| Rust property/metamorphic + corpus regressions | targeted on affected modules | extended cross-module weekly | source receipts, deadline invariants, round-trip serialization, replay/dedup |
| Rust fuzz (cargo-fuzz/libFuzzer) | smoke on parser changes where feasible | sustained nightly/weekly | deterministic seed, crashes retained, no live upstream |
| Rust mutation (cargo-mutants) | changed-risk-scope smoke or informational | weekly with scored thresholds | don't mutate unchanged modules on every PR |
| Rust profiling (Criterion/dhat/heap) | regression microbench on critical performance changes | benchmark and memory trend | pinned benchmark environments, variance/tolerance documented |
| Python Ruff formatting/lint + basedpyright | required while application Python exists | release | remove blanket suppressions incrementally by module with baseline and ownership |
| Python Hypothesis/metamorphic | targeted changed-risk-scope | broader weekly | eliminate duplicate test execution across jobs |
| Python Atheris | bounded smoke if compatible | sustained opt-in/schedule | keep compatible isolated runtime until 3.15 supported; no false claims |
| Python profiling Scalene | not all PRs | manual/nightly baseline | benchmark live runtime, do not treat full pytest profiling as production profile |
| Dependency/license/security | required on PR for affected dependency graph | full weekly/release | cargo audit/deny, pip-audit, CodeQL, bandit/Ruff S, gitleaks, Presidio |
| Privacy and agent safety | targeted mandatory on source/MCP/agent changes | adversarial full weekly | prompt injection, SSRF, PII redaction, secret leakage, tool boundary |
| Documentation/prose/contracts | format and markdown/prose changed scope | full release | markdownlint, cspell/typos, links, schema validation, generated docs drift |
| Release/SBOM/provenance | verify on release/packaging changes | mandatory publish | reproducible archives where achievable, signed artifacts, SLSA/SBOM |
| Conductor consistency | mandatory fast PR | scheduled full reconciliation | registry ↔ metadata ↔ issue ↔ GitHub project state and acceptance evidence |

## Explicit defects to remediate
- CI inconsistent Python versions (3.11/3.12) and Rust `stable` while target minimums 3.15/1.99; no root toolchain pin.
- `pyproject.toml` has `requires-python >=3.11`, pyright 3.11 and Ruff py311; `uv.lock` also >=3.11.
- Some workflows target only `main` although repository default is `master` (profiling).
- Rust coverage excludes several source paths while enforcing 90%; document risk-specific coverage and separate aggregate inclusive coverage.
- Rust audit uses RUSTSEC ignore list: each exception must have owner, justification, expiry and upstream resolution track.
- Pin GitHub actions by immutable SHA where practical and use least-privilege permissions throughout.
- `ci-learning-candidates` emits an artifact but does not automatically repair or reconcile Conductor; build a non-committing detector + deterministic repair PR with safe constraints.
- Rust Clippy should include all features; remove weakly justified Python basedpyright suppressions incrementally, not with an uncontrolled global flip.
- Capture no-bypass policy: tests must fail if provider network access escapes the designated source boundary.
- Clarify release version authority: Rust crates 0.1.2 vs legacy Python 1.2.1. Use one release manifest recording component versions, Git SHA, provenance and compatibility; avoid falsely forcing unrelated packages to same semver.

## Harness and context contract
- `.conductor/` canonical. Define architecture decision records and repository map; keep compact `AGENTS.md` or equivalent as an index, not a duplicated parallel plan.
- Every track: unique ID, owner, status, requirements, non-goals, precise tests, acceptance evidence URI, release/rollback plan, dependency issue numbers, completion timestamp.
- Implement a validator that rejects duplicate active tracks, stale metadata, nonexistent links and 'completed' status without verifiable commit/test evidence.
- Use change-based test selection as an optimisation only; mandatory smoke, safety, contract and full release gates cannot be skipped by heuristic.
- Single developer: zero default manual review steps for safe code; automated PR CI and automerge for green changes, human confirmation for external writes, credential changes, sensitive disclosure and materially breaking releases.

## Close-out rule
No item is done simply because files, tests or badges exist. Each claimed capability must have demonstrable current-commit behaviour and an independent negative test. Archive completed tracks only after release verification, downstream consumer checks and Conductor reconciliation.

## Implementation ledger (2026-11-13; branch `plan/frontier-rust-migration-quality-20261113`)

- [x] Write frontier architecture and single-maintainer engineering requirements, integrate with existing source-adapter boundary.
- [x] Register track in `.conductor/tracks.md` and `.conductor/index.md`, with machine-readable metadata.
- [x] Add portable Conductor frontier contract validator and regression tests; synthetic six-test run passed in hosted PR CI before portability regression was added.
- [x] Add on-demand inventory of Python production modules and the three published entrypoints. Inventory includes duplicate-suffix `IASN2001657` files for investigation, not automatic deletion.
- [x] Eliminate developer-specific hardcoded OneDrive root from legacy Conductor status tool; add regression from unrelated working directory. Hosted verification pending for this addition.
- [x] Declare Rust 1.99 MSRV in all three Rust crate manifests and pin the 1.99.1 toolchain; dedicated MSRV CI executes all-target/all-feature Clippy, formatting and tests. Full hosted result pending.
- [x] Correct profiling branch trigger from `main` to `master`.
- [x] Add non-blocking Python 3.15 compatibility sensor: full frozen install and test report, retaining old supported runtime until proven.
- [x] Verify Rust 1.99.1 MSRV on GitHub PR #403 (2026-11-13): fmt, all-target/all-feature Clippy and all-feature tests succeeded on workflow run 43366742657. Keep gate permanently enabled.
- [~] Python 3.15 sensor succeeded: frozen lock installed and 824 tests passed (2 skipped) on hosted run 43366742659. Full interpreter-minimum change remains pending Atheris, Presidio, packaging compatibility and security-tool validation.
- [ ] Extend Conductor validation across legacy tracks with warning-only grandfathering initially; migrate old metadata and eliminate duplicate/stale states through reviewed PRs.
- [ ] Complete Python ↔ Rust CLI, network and archive parity matrix and consumer migration, then deprecate legacy package.
- [ ] Enforce evidence provenance and source boundary handoff through existing #309–#314 track and release-pinned archive shadow tests.
- [~] Coverage gate on run 43366742716 failed: 89.66% Rust line coverage against >=90% threshold. Added offline provider adapter contract tests in commit 0f52bd53; current-commit hosted coverage still must pass. Do not lower threshold or exclude additional code.\n- [ ] Introduce risk-based metamorphic/mutation/fuzz/benchmark target selection and full release evidence gates.
- [ ] Implement law/rule state machine and research-frontier extensions after foundational contracts.

No checkbox above asserting full CI pass, runtime compatibility or Rust-only application migration should be considered complete until corresponding hosted evidence is linked.
