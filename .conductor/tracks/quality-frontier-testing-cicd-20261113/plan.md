# Plan

## Phase 1: Applicability and exclusions

- [x] Confirm DST applicability: which code paths benefit from deterministic simulation (document exclusions). [checkpoint: b250b49]
- [x] Confirm MT applicability: which units are suitable for mutation testing (document exclusions). [checkpoint: 329e6b2]
- [~] Confirm contract testing applicability: which API boundaries need contract tests.
- [ ] Document all exclusions in `QUALITY_FRONTIER_EXCLUSIONS.md` or equivalent.

## Phase 2: Deterministic simulation testing (DST)

- [ ] Add deterministic, replayable test suites with bounded runtime.
- [ ] Add thresholds for DST pass/fail criteria.
- [ ] Integrate DST into CI as a fast lane.

## Phase 3: Mutation testing (MT)

- [ ] Add mutation testing infrastructure (e.g., mutmut or equivalent for Python/Rust).
- [ ] Add mutation score thresholds.
- [ ] Integrate MT into CI as an expensive/scheduled lane.

## Phase 4: Contract testing

- [ ] Add contract tests for API boundaries (Python/Rust cross-runtime parity).
- [ ] Add contract test thresholds.

## Phase 5: CI hardening

- [ ] Pin all GitHub Actions to full commit SHAs (pinned_actions).
- [ ] Add dependency_review workflow for PRs.
- [ ] Add actionlint workflow linting.
- [ ] Add least-privilege, concurrency, immutable actions.
- [ ] Schedule expensive lanes (mutation, E2E) to keep PR checks fast.
- [ ] Retain CI evidence (artifacts, logs).

## Phase 6: Verification and closure

- [ ] Run full CI locally and confirm all required checks pass.
- [ ] Update track metadata status to `completed`.
- [ ] Close issue #280.

## Task Order

Work sequentially: Phase 1 → Phase 2 → Phase 3 → Phase 4 → Phase 5 → Phase 6.
