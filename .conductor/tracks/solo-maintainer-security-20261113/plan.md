# Plan

## Phase 1: Inventory and baseline

- [x] Inventory bots and collaborators (solo operation confirmed; only `app/renovate` and historical `app/dependabot`).
- [x] Record branch protection baseline: 12 stable required checks, no approvals, force-push/deletion blocked, `enforce_admins: true`.
- [x] Record missing context: `AGENTS.md` absent; `CONTRIBUTING.md` and `.github/SECURITY.md` present.
- [x] Record Renovate health: dashboard #325 active; `.github/dependabot.yml` never existed.

## Phase 2: Ruleset (additive)

- [x] Create an active ruleset on the default branch blocking `deletion` and `non_fast_forward`; no required status checks beyond the existing stable set; no approval/CODEOWNERS/team rules. (Ruleset `solo-maintainer-default-branch`, id 27191113 — verified via `gh api repos/edithatogo/fyi-cli/rulesets`.)
- [x] Verify the ruleset via `gh api repos/{owner}/{repo}/rulesets`.

## Phase 3: Canonical context, validation, drift receipt

- [x] Add canonical `AGENTS.md`.
- [x] Add `scripts/verify_solo_maintainer.py` (one-command validation; fail-closed; emits drift receipt JSON).
- [x] Commit the drift receipt for the observed state (`drift-receipt.json`, all checks PASS).

## Phase 4: Renovate and Codecov rollout

- [x] Inherit `github>edithatogo/renovate-config` in `renovate.json`; retain justified overrides (timezone, stabilityDays, internalChecksFilter, rangeStrategy, 0.x automerge exclusion).
- [x] Verify Codecov OIDC configuration and commit status; record evidence (`codecov/patch` success on master; upload gate required and green).
- [x] Keep zero mandatory human approvals (required approving review count stays 0).

## Phase 5: Verification and closure

- [x] Run the one-command validation; confirm it passes and the receipt is committed. (11/11 PASS, live mode.)
- [x] Update track metadata status to `completed`.
- [ ] Close issue #281 after the PR merges.
