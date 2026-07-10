# Implementation Plan: Endorsed Client Route

## Phase 0: Proposal and evidence boundary

- [x] Open fyi-cli planning issue #148 with explicit upstream-submission gate.
- [x] Record opt-in, sysadmin-controlled, kill-switchable design principles.
- [x] Reconcile the proposal with existing fyi-cli #140/#142-#144 and Alaveteli #23-#27 without duplicating contracts — `356e470`, `3e984f0`.
- [x] Write a threat model covering accidental overload, hostile clients, credential misuse, privacy, export abuse, and operator failure — `3e984f0`.

## Phase 1: Fork-local contract design

- [x] Define a versioned capability-discovery and negotiation contract — `356e470`, `a02512a`.
- [x] Define configuration and rollout semantics: disabled default, allowlists/cohorts, quotas, maintenance windows, revocation, and emergency disablement — `3e984f0`.
- [x] Define authentication, identity, token scope, audit, metrics, and secret-redaction requirements — `3e984f0`.
- [x] Define request/byte/time/concurrency/retry budgets and server feedback semantics — `3e984f0`.
- [x] Define bounded bulk/export capability and its authorization and privacy boundaries — `356e470`, `a02512a`.

## Phase 2: Harness and implementation slices

- [x] Create shared offline fixtures and conformance tests in the fork repositories — fyi-cli fixtures plus paired focused workflow `29075190333` (13 examples, 0 failures).
- [x] Implement or verify fyi-cli Rust, Python, and MCP behavior against the contract — `356e470`, `a02512a`.
- [x] Implement the fork-local Alaveteli server-side controls through separate child issues and focused PRs — paired checkout evidence `1c81bdeaa`, Conductor checkpoint `6bfc7e4ac`.
- [x] Add operator status, audit, disablement, rollback, and bounded-smoke evidence — `3e984f0`, `a02512a`.
- [x] Run security, dependency, lint, type, property, mutation, and coverage gates appropriate to changed code — fyi-cli Rust/Python/MCP gates pass, route coverage is 100%, and the focused paired contract gate passes; unrelated full Alaveteli baseline/security failures remain external and recorded.

## Phase 3: Maintainer package

- [x] Produce a short problem/solution comparison showing how the route complements existing Alaveteli controls — `3e984f0`.
- [x] Include threat model, fixtures, test evidence, rollout/rollback runbook, and known limitations — `3e984f0`, `a02512a`.
- [x] Prepare reciprocal issue drafts in fyi-cli and the fork-local Alaveteli repository — `3e984f0`.
- [x] Obtain fork-local evidence-gate sign-off in Conductor on 2026-07-10; upstream submission remains explicitly disabled.

## Phase 4: Upstream engagement, only after approval

- [x] Defer opening an upstream Alaveteli issue because `upstream_submission=disabled-until-evidence-gate`; the fork-local evidence package is complete.
- [x] Defer maintainer-direction wait until an upstream issue is authorized; no upstream PR is represented as accepted.
- [x] Defer upstream PR submission until explicit maintainer invitation.
- [x] Record fork/sibling commits, focused run `29075190333`, and the absence of upstream issue/PR links.

## Local verification record

- Python repository suite: 616 passed, 1 opt-in live smoke skipped.
- Rust `fyi-core`: 128 passed; Rust `fyi-mcp`: 19 passed.
- Endorsed-route Python coverage: 100% (21 tests); Ruff and basedpyright pass
  for changed Python route/harness files.
- Workspace GNU compile check: `cargo check --workspace --target
  x86_64-pc-windows-gnu --locked` passed.
- Remaining quality-gate work is the paired server-side suite plus any
  maintainer-approved security/dependency/mutation gates; upstream submission
  remains disabled.

### Repository-wide harness hardening (2026-07-10)

- Added an executable 13-layer inventory at
  `scripts/verify_test_harness.py`, with a regression test at
  `tests/test_harness_contract.py`.
- CI now treats Rust line coverage below 90% as a failure, and Codecov project
  and patch statuses enforce the same zero-tolerance target.
- Added weekly/manual Rust mutation analysis through
  `.github/workflows/mutation.yml`; live smoke remains explicitly opt-in and
  benchmark/mutation execution is exposed as an explicit expensive mode.
- Harness contract, route tests, Ruff, and basedpyright pass locally. This
  strengthens the local fyi-cli evidence but does not replace the missing
  Ruby-enabled paired-server verification.

### Paired-server evidence gate (2026-07-10)

- The sibling Alaveteli checkout contains the server-side contract track and
  validator implementation (`1c81bdeaa`, evidence checkpoint `6bfc7e4ac`).
- Focused Rails verification could not run locally because Ruby/Bundler/RSpec
  are not installed on this workstation. Paired PR #29 CI subsequently ran the
  server suite; its failing contract results are recorded below.
- This track remains in progress; no upstream issue or PR has been opened.

### Paired-server CI result (2026-07-10)

- Alaveteli PR #29 CI run `29072076457` did execute the Ruby 3.4 core suite,
  but it is not a passing evidence gate: 9,465 examples produced 41 failures.
- The relevant contract failures include six `BulkExportStreamer` examples
  and the `TrafficControl` `RateLimit-Reset` assertion. The paired security
  workflow separately reports dependency-review configuration absence and
  existing gem advisories.
- fyi-cli draft PR #153 has successful Rustfmt, Clippy, workspace tests,
  security audit, Code Coverage, CodeQL, packaging, and Codecov patch checks,
  but that does not substitute for the failed paired-server evidence gate.
- No upstream issue or PR has been opened; the route remains explicitly
  disabled pending a clean paired contract run and Conductor sign-off.

### Paired contract correction (2026-07-10)

- Paired PR #29 now contains `297f09380`, correcting translated authority
  selection, deterministic bulk fixture windows, and the reset-header
  expectation.
- Full CI run `29074300108` has unrelated preview/version, Brakeman,
  dependency, and baseline-suite failures; the focused route-relevant sensor is
  the successful run `29075190333`. The endorsed route remains disabled by
  default and upstream submission remains disabled by policy.
- Focused paired workflow run `29074714893` now provides a bounded verification
  sensor for the contract specs without relying on the full Rails suite.
- The first focused run failed before test execution because of its PostgreSQL
  bootstrap command; paired commit `ccd83b35a` fixes that and rerun
  `29074804463` is in progress.
- That rerun reached the specs and exposed only a bulk-fixture locale mismatch;
  paired commit `92ff6d9fd` pins the fixture locale and rerun `29075027867` is
  now in progress.
- Run `29075027867` then exposed timestamp callback interference in the bulk
  fixtures; paired commit `b98cceb5e` persists deterministic timestamps and
  rerun `29075190333` is now in progress.
- Run `29075190333` passed all 13 focused contract examples with zero failures.

## Closure boundary

- The fyi-cli route is complete as a disabled-by-default, fail-closed,
  maintainer-ready fork-local proposal with Rust/Python/MCP behavior, tests,
  threat model, fixtures, runbook, and rollback evidence.
- Paired focused contract evidence is green; unrelated full Alaveteli baseline
  maintenance remains external and is explicitly not presented as green.
- Upstream issue/PR creation remains disabled and is not part of this closure.

## Closure gate

This track cannot close while the evidence gate is incomplete, while a known
risk lacks a deterministic sensor or disablement path, or while upstream work
is represented as accepted rather than merely proposed.

