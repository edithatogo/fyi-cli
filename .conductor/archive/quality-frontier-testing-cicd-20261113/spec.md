# Quality Frontier: Testing and CI/CD Gaps

Issue: [fyi-cli #280](https://github.com/edithatogo/fyi-cli/issues/280).
Parent umbrella: [fyi-cli #282](https://github.com/edithatogo/fyi-cli/issues/282).

## Goal

Close the warranted testing and CI/CD gaps identified in #280 so that the repository
has deterministic, replayable test suites, bounded mutation and contract testing, and
least-privilege CI with pinned actions, dependency review, and workflow linting.

## Warranted Gaps

Per issue #280 evidence:

- **DST** — Deterministic simulation testing: replayable suites with bounded runtime and thresholds.
- **MT** — Mutation testing infrastructure and score thresholds.
- **Contract** — Contract tests for API boundaries and cross-runtime parity.

## CI Gaps

Per issue #280 evidence:

- **pinned_actions** — All GitHub Actions must use pinned versions (full commit SHA).
- **dependency_review** — Dependency review must run on PRs to catch vulnerable introductions.
- **workflow_lint** — Workflow files must be linted (actionlint or equivalent).

## Acceptance

- [x] Confirm applicability of DST, MT, and contract testing; document exclusions.
- [x] Add deterministic, replayable test suites with bounded runtime and thresholds.
- [x] Add mutation testing infrastructure with score thresholds.
- [x] Add contract tests for API boundaries.
- [x] Pin all GitHub Actions to full commit SHAs.
- [x] Add dependency_review to CI for PRs.
- [x] Add workflow linting (actionlint) to CI.
- [x] Keep PR checks fast; schedule expensive lanes (mutation, E2E) and retain evidence.
- [x] Add least-privilege, concurrency, immutable actions, updates, and linting gates.

## Non-Goals

- Fixing #281 (security and solo-maintainer context) — separate track.
- Closing #282 (umbrella) — depends on this track and #281 being complete.

Acceptance is independently reconciled in [acceptance-evidence.md](acceptance-evidence.md) and [ci-verification.json](ci-verification.json). The closing archive PR records its own final-head verification and merge.
