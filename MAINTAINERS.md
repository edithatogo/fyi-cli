# Maintainer Policy

This repository is maintained through GitHub issues, pull requests, GitHub
Actions, and the Conductor track registry in `.conductor/`.

## Response Targets

- New bug reports: triage within 10 business days.
- Security reports: use GitHub private vulnerability reporting; acknowledge
  within 2 business days.
- Pull requests: first maintainer response within 10 business days when CI is
  passing and the change is in scope.
- External registry issues: keep a tracking issue open while the registry state
  is pending, blocked, or awaiting indexing.

These are response targets, not guarantees. The maintainer may batch low-risk
documentation, packaging, and registry follow-ups into periodic hygiene passes.

## Issue States

Issues should normally have one or more of these labels:

- `needs-triage`: awaiting maintainer classification.
- `accepted`: in scope and ready for implementation.
- `blocked-external`: waiting on a registry, platform, upstream service, or
  account action outside this repository.
- `duplicate`: already tracked elsewhere.
- `wontfix`: intentionally out of scope.
- `good first issue`: suitable for a new contributor.

Completed Conductor tracks are represented by closed issues or archived track
records. Open external registry work remains open until the live registry state
confirms the release, score, or indexing change.

## Release And Registry Hygiene

Release readiness includes:

- Local Rust validation with fmt, clippy, tests, and package smoke checks.
- Passing GitHub Actions on `master`.
- A GitHub release or registry release where the target registry requires one.
- Updated issue comments when a registry state is externally blocked or pending.

Glama, Smithery, GitHub MCP registry, and similar external surfaces may lag the
repository. Do not close registry tracking issues until the public registry page
or API confirms the expected state.
