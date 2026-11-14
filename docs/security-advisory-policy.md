# Security advisories, required CI and local pre-commit checks

## Current state (14 November 2026)

The security page `/security/dependabot` is owner-restricted. The linked GitHub
connector cannot read its alert API. A green audit job is **not** equivalent
to zero vulnerabilities: `cargo audit` currently suppresses five RustSec
findings and `deny.toml` records 20 historical exceptions.

The canonical programme is
`/.conductor/tracks/security-advisory-remediation-20261114/`.
Issue [#325](https://github.com/edithatogo/fyi-cli/issues/325)
tracks the blocked dependency fixes. Existing exceptions expire
**14 December 2026**, unless removed earlier. They are *not* security fixes.

## What is fail-closed now?

`scripts/verify_security_exceptions.py` uses only the Python standard library.
It rejects any newly suppressed RustSec identifier, any exception missing
owner/issue/reason/expiry, an expired record, or an audit-workflow exception
absent from `deny.toml`. The frozen identifiers are explicit in the checker:
changing the JSON policy alone cannot grant a new exception.

Run:

```bash
python3.15 scripts/verify_security_exceptions.py --require-python 3.15.1
```

The required **Security Audit** job executes the same guard on Python
3.15.1 before `cargo audit` and `cargo deny`. This protects the current
baseline from additional silent suppressions and causes the job to fail
when the temporary exceptions expire. It does not remove existing risks,
and maintainers with write access can intentionally propose changes to the
checker itself; code review and protected CI still matter.

## Local pre-commit

This repository previously had no `.pre-commit-config.yaml`. It now
includes an opt-in local guard. Configure your development machine with
a **verified Python 3.15.1** environment, then:

```bash
python3.15 --version
python3.15 -m pip install pre-commit
python3.15 -m pre_commit install
python3.15 -m pre_commit install --hook-type pre-push
python3.15 -m pre_commit run security-exception-guard --all-files
cargo install cargo-deny --locked
python3.15 -m pre_commit run cargo-deny-advisories --hook-stage pre-push --all-files
```

The fast pre-commit check enforces exception policy; the optional pre-push
`cargo deny check advisories` scans the actual Rust dependency graph against
its current advisory database (subject to existing, expiring `deny.toml`
exceptions). It does **not** independently prove the suppressed risks fixed.

Installing a configuration file into the repository does **not**
automatically enable the hook in an existing clone; check
`.git/hooks/pre-commit` locally. `git commit --no-verify` can bypass a
local hook, but never the protected CI job. Gitleaks and Presidio already
run in GitHub Actions; they are **not** evidence of a locally installed
pre-commit hook.

For fast staged checks, Ruff, Rustfmt, git whitespace, actionlint and
Gitleaks are candidates for future explicit local configuration. Avoid
running full CodeQL, mutation testing or the entire Rust graph before
every commit; keep those as required or scheduled CI. No additional local
hook is claimed installed.

## Actual vulnerability remediation

The current suppressions have been reduced from 20 historical RustSec entries to seven, after 13 were verified as not encountered in the master 9050a664 Cargo-deny audit. This does **not** establish that any of the five explicitly ignored active advisories have been fixed.\n\nThe existing audit uses CLI ignores for RUSTSEC-2026-0387,
RUSTSEC-2026-0459 and RUSTSEC-2026-0370/0371/0372. Resolve or
remove their affected transitive dependencies using
`cargo tree -i <crate>` and lockfile evidence. For SQLite, check
malicious database/WAL admission and compatible SQLx/rusqlite versions;
for cryptography, test the p256/elliptic-curve/crypto-bigint chain; for
zbus, qualify a supported keyring dependency. Re-check all other
historical entries and remove obsolete exceptions.

The remediation target is `cargo audit` and `cargo deny` with **no
security ignores** and no applicable open Dependabot alerts. Each
dependency change needs exact-head security, tests, licence,
90% coverage, MSRV and compatibility evidence.

Python 3.15.1 qualification of **every** active Python CI runtime is
a separate shared-workflow change owned by the sole integration
coordinator; adding a qualified Python security-policy step does not
qualify the old 3.12 jobs.
