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
- [~] Add Python stdlib `scripts/verify_security_exceptions.py` with
  test-first negative fixtures: reject any new IDs, expired/missing exception
  records, mismatched `deny.toml` and workflow bypass attempts.
- [~] Time-box all current 20 historical entries in
  `security/advisory-exceptions.json` with owner, issue, reason and expiry.
- [~] Run that guard inside the **required Security Audit** job using exact
  Python 3.15.1 and expose an opt-in local `pre-commit` entry.
- [ ] Confirm all required exact-final-head CI/coverage/security gates and
  independent review; request coordinator merge window. No merge here.

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
