# Security advisory remediation and exception retirement

Track ID: `security-advisory-remediation-20261114`
Status: in_progress
Owner: FYI-CLI security/dependency lane; final integration exclusively through **FYI-CLI — Issue fixes and PR merges**
GitHub tracking: [Dependency Dashboard #325](https://github.com/edithatogo/fyi-cli/issues/325)

## Goal

Eliminate known actionable dependency vulnerabilities, replace indefinite
suppression with expiring reviewed exceptions, and prevent introduction of
future RustSec suppressions through the protected **Security Audit** job.
Retain the solo-maintainer ruleset, CI quality thresholds and source boundaries.

## Existing baseline (14 November 2026)

- `deny.toml` contains **20** historical RustSec ignore entries.
- `.github/workflows/ci.yml` invokes `cargo audit` with **five** explicit
  ignored advisories: RUSTSEC-2026-0387, -0459, -0370, -0371, -0372.
- Rust audit and cargo-deny can return success while those findings are suppressed;
  passing audit is not evidence that ignored risks were remediated.
- The GitHub connector cannot enumerate owner-only Dependabot alerts. No
  complete alert inventory, severity or patch release is claimed here.
- `.pre-commit-config.yaml` was previously absent: no repository-managed hook
  installation was documented. Local Git hook state is not observable remotely.
- Renovate, dependency-review, CodeQL, Gitleaks and Presidio run separately.

## Non-negotiable boundaries

No changes to source transports, MCP, archive state, Python fallback, release
packaging or consumer activation from this track. Keep
`archive_cutover_allowed=false`. Protect the existing 90% Rust line coverage,
Rust MSRV 1.99.1, Python 3.15.1 qualification requirement, signed release
contracts, and all 12 required contexts. Do not change a vulnerability's state
to dismissed merely because an exception exists.

## Acceptance

1. Enumerate every open Dependabot alert using owner-authorized credentials;
   record exact alert/GHSA/RUSTSEC/package/version/manifest/patch and
   transitive reachability, priority, mitigation, owner and due date.
2. Remediate both genuine security advisories and unmaintained dependency
   chains via compatible patched versions with regression tests. Never force
   incompatible cross-version transitive overrides.
3. A required protected CI job rejects new unapproved RustSec suppressions,
   undocumented entries and expired existing exceptions. No permanent
   exemption is available; retirement is tracked until zero ignores remain.
4. A documented optional pre-commit hook runs the same guard; bypassing it
   cannot bypass CI. Validate actual installed-hook status locally when possible.
5. For every upgrade, demonstrate Cargo lock resolution, cargo audit/deny,
   MSRV, strict Clippy, all-feature tests, immutable fixture/security tests,
   required Python/JS checks, 90% coverage and final-head Actions.
6. Only close the track when zero applicable open security alerts and zero
   suppressed actionable advisories are demonstrated using fresh evidence.
   Any blocked upstream package remains explicitly open with follow-up.
