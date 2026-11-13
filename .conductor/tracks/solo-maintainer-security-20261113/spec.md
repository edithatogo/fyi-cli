# Solo Maintainer Security and Context

Issue: [fyi-cli #281](https://github.com/edithatogo/fyi-cli/issues/281).
Parent umbrella: [fyi-cli #282](https://github.com/edithatogo/fyi-cli/issues/282).

## Goal

Maximise repository security for a solo maintainer: an additive ruleset blocking
force-push/deletion on the default branch, solo operation with no approval/CODEOWNERS/team
gate but retained owner recovery, canonical agent/security/contribution context, a
one-command validation command, and a committed drift receipt recording the observed state.

## Evidence (from issue #281)

- Protection/rulesets: **yes / 0** (branch protection exists; no rulesets).
- Context gaps: **agents** (CONTRIBUTING.md and .github/SECURITY.md exist; AGENTS.md missing).
- Setting follow-up: **none**.

## Bot inventory (2026-11-13)

- Collaborators: `edithatogo` (User) only — solo operation confirmed.
- PR authors: `edithatogo` (14 of last 15), `app/dependabot` (1, historical).
- Renovate hosted app: active — created Dependency Dashboard issue [#325](https://github.com/edithatogo/fyi-cli/issues/325) and merged #321; PRs #395–#399 closed/superseded.
- `.github/dependabot.yml`: never existed in this repository — no update-coverage gap.
- Branch protection (master): 12 required checks (stable set), `strict: true`,
  `enforce_admins: true`, `allow_force_pushes: false`, `allow_deletions: false`,
  required approving review count **0** (no approval gate), code-owner reviews not required.

## Acceptance

- [x] Add a ruleset blocking force-push and deletion on the default branch (additive to
      existing branch protection; required status checks remain the stable 12 contexts).
      Ruleset `solo-maintainer-default-branch` (id 27191113), active, rules `deletion` +
      `non_fast_forward`, no status/approval rules.
- [x] Keep solo operation: no approval/CODEOWNERS/team gate; owner recovery retained
      (`enforce_admins: true`, 0 required approving reviews, code-owner reviews not required;
      CODEOWNERS retained for review routing only).
- [x] Add canonical `AGENTS.md` (agent context) linking existing security/contribution context.
- [x] Add one-command validation (`scripts/verify_solo_maintainer.py`) that fails closed and
      emits a drift receipt JSON.
- [x] Commit a drift receipt recording observed protection/ruleset/context state
      (`.conductor/tracks/solo-maintainer-security-20261113/drift-receipt.json`, all PASS).
- [x] Inherit `github>edithatogo/renovate-config` in `renovate.json`, retaining justified
      repository-specific overrides (Pacific/Auckland timezone, stabilityDays,
      internalChecksFilter, widen rangeStrategy, 0.x automerge exclusion).
- [x] Verify Codecov OIDC upload is configured and commit status is active (evidence below).

## Codecov evidence (2026-11-13)

- `.github/workflows/ci.yml`: `codecov/codecov-action` pinned to a commit SHA, `use_oidc: true`,
  `id-token: write`, `fail_ci_if_error: true`, uploading `lcov.info` from `cargo llvm-cov`
  (gate: `--fail-under-lines 90`).
- Master commit status: `codecov/patch: success` (verified via
  `gh api repos/edithatogo/fyi-cli/commits/master/status`).
- Anonymous Codecov web API returns 404 without a token; the active commit status plus the
  `fail_ci_if_error: true` upload gate (the coverage job is required and green on master)
  are the verifiable evidence of an active Codecov repository. Full dashboard access requires
  the owner's Codecov login — not required for this gate.

## Renovate and Codecov rollout (from issue #281)

- [x] Grant the hosted Renovate app access — already active (dashboard #325; PRs #395–#399; #321 merged).
- [x] Inherit `github>edithatogo/renovate-config`; retain justified repository-specific overrides.
- [x] Remove `.github/dependabot.yml` — never existed in this repository.
- [x] Produce real test coverage, upload with Codecov OIDC (`id-token: write`) — configured in
      `.github/workflows/ci.yml` (`use_oidc: true`, pinned action, `fail_ci_if_error: true`).
- [x] Verify both the Codecov repository and commit status are active — `codecov/patch`
      success on master; OIDC upload gate required and green (see Codecov evidence).
- [x] Keep zero mandatory human approvals — required approving review count is 0; automated
      checks are the merge gate.

## Non-Goals

- Removing or weakening existing branch protection (including `enforce_admins`) — solo
  recovery stays with the owner via repository administration, not by lowering the gate.
- Editing CI required check lists — the 12 stable contexts are already the merge gate.
- Closing #282 (umbrella) — closes only when #280 and #281 are complete.
