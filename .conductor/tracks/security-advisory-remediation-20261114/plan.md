# Implementation plan — retire advisory suppressions

## Phase 0: canonical alert and reachability inventory
- [x] Read Rust audit/deny policy, CI configuration and existing hooks on master
  (14 November baseline; audit run 43578036426).
- [ ] Obtain real Dependabot open-alert JSON from an owner with security-alert
  read permissions; reconcile GitHub, RustSec, Cargo.lock, uv.lock,
  dashboard/package-lock.json and Actions usage.
- [ ] Build a versioned alert ledger containing direct/transitive path,
  runtime reachability, affected range, fixed version, CVSS/impact,
  environment, owner and evidence timestamp. No invented alert IDs.

## Phase 1: stop future suppression
- [x] Add Python stdlib `scripts/verify_security_exceptions.py` with
  test-first negative fixtures: reject any new IDs, expired/missing exception
  records, mismatched `deny.toml` and workflow bypass attempts. [checkpoint: `30d836a`, PR #451]
- [x] Record the seven retained historical exceptions in
  `security/advisory-exceptions.json` with owner, issue, reason and expiry.
  [checkpoint: `30d836a`, PR #451]
- [x] Run the guard in the required Security Audit job and add the opt-in
  `pre-commit` entry. [checkpoint: `30d836a`, PR #451]
- [x] Verify final PR head `adb06db` and record integration into `master`.
  Security Audit and coverage passed on that head.
  [checkpoint: `30d836a`] [Final CI](https://github.com/edithatogo/fyi-cli/actions/runs/43836403252) · [PR #451](https://github.com/edithatogo/fyi-cli/pull/451)

### Retirement of 13 stale exceptions

- [x] Retire the 13 identifiers reported `advisory-not-detected` at baseline
  `9050a664`. PR #455 merged the cleanup into the guard branch as `910dbd3`.
  [Baseline audit](https://github.com/edithatogo/fyi-cli/actions/runs/43578036426/job/132396499550) · [PR #455](https://github.com/edithatogo/fyi-cli/pull/455)
- [x] Verify that PR #451 carried the same 13 removals to `master` and omitted
  those IDs from the exception policy and guard. [checkpoint: `30d836a`]

At `951e008` there are eight active time-bounded exceptions in
`deny.toml`, the policy and the CI audit flags. Seven are historical and the
eighth covers transitive `humantime`. All eight expire on 14 December 2026.
The active exceptions still require remediation or verified non-applicability
and a fresh unsuppressed audit. [Exception policy](../../../security/advisory-exceptions.json) · [PR #497](https://github.com/edithatogo/fyi-cli/pull/497)

## Phase 2: patch exploitable advisories first
- [ ] RUSTSEC-2026-0459: trace libsqlite3-sys through sqlx/rusqlite/arti
  using `cargo tree -i`; test fixed supported versions and database/WAL
  admission policy. Keep alert open until fixed or independently unreachable.
- [ ] RUSTSEC-2026-0387: trace crypto-bigint through p256,
  elliptic-curve and ssh-key-fork-arti; update the supported chain when
  possible, preserve sensitive crypto contracts and add side-channel-aware
  use/reachability rationale.
- [ ] RUSTSEC-2026-0370/0371/0372: review zbus and keyring transitive
  chains, qualify modern supported releases and Linux credential-provider
  behaviour. They concern maintenance, not automatically proven exploits.
- [~] #495 (`RUSTSEC-2026-0543` / `RUSTSEC-2026-0542`): eliminate direct and
  `sqlx` `chrono` dependencies across the workspace by migrating to `time`
  (zero matches in `cargo tree`; only an unactivated weak optional edge under
  `tor-* -> serde_with 3.24.0` remains in `Cargo.lock`); record synchronized
  time-bounded Tor transitive exception `RUSTSEC-2026-0542` (`humantime 2.4.0`
  via `arti-client 0.48.0`, expires `2026-12-14`, tracked under
  [#495](https://github.com/edithatogo/fyi-cli/issues/495)) until upstream
  `arti-client` / `tor-*` replaces `humantime`.
- [ ] Verify the full set of other historical exceptions individually;
  remove stale `advisory-not-detected` entries immediately where evidence
  supports non-applicability.

## Phase 3: make audit strictly unsuppressed
- [ ] After each successful fix, delete its `deny.toml`, CLI flag,
  exception-policy entry and the matching grandfathered identifier;
  include verification of the former advisory in PR evidence.
- [ ] Reach `cargo audit` with **no --ignore flags**, `cargo deny`
  without security advisory exceptions and no open remediable
  Dependabot security alerts.
- [ ] Include an automated weekly fresh advisory scan and due-date reporting
  through existing CI, not an alternative network client or parallel release.

## Phase 4: broad hygiene and local developer hooks
- [ ] Reconcile Python 3.15.1 across all active CI, reusable, uv, fuzzing,
  Presidio and auxiliary jobs with the sole coordinator.
- [ ] Review pre-commit applicability: Git whitespace, Ruff, Rustfmt,
  actionlint, gitleaks and advisory policy should be staged-file/fast;
  full cargo audit, pip-audit, JS audit, property/mutation and CodeQL
  remain CI/scheduled rather than slowing every commit.
- [ ] Prove local hook installation on each development machine; repository
  configuration alone does not activate `.git/hooks/pre-commit`.

## Closing rule
Record every PR head, exact SHA, outcome, remaining advisory and rollback.
Do not close while a still-applicable suppression, unqualified patch,
unavailable scanner or unverified private alert remains.
