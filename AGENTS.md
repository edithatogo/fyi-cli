# AGENTS.md — fyi-cli agent context

Canonical context for coding agents (human or AI) working in this repository. Read this
before changing anything. Security and contribution process live in
[.github/SECURITY.md](.github/SECURITY.md) and [CONTRIBUTING.md](CONTRIBUTING.md); both remain
authoritative for their domains.

## Project

`fyi-cli` / `fyi-mcp` — a multi-jurisdiction Freedom-of-Information client for
Alaveteli-based platforms (Alaveteli, WhatDoTheyKnow, FragDenStaat, …). Rust-first workspace
(`crates/fyi-core`, `crates/fyi-cli`, `crates/fyi-mcp`); the legacy Python package
(`src/fyi_system`) is a reference implementation and is not extended.

## Non-negotiable constraints

- **Read-only by default.** No agent may send a real information request, message, or form
  submission to any live authority, portal, or external service unless the operator has
  explicitly authorised that exact action in the session. Dry-run/offline paths are the
  default everywhere.
- **Tests must not touch the network.** Use mock transports and fixtures. Any test that
  needs a live endpoint is a manual, operator-authorised smoke check — never CI.
- **No credentials, tokens, cookies, or PII in commits, logs, fixtures, or issue text.**
- **Fail closed.** Verification scripts and guards must error out, not warn-and-continue, on
  any missing evidence.

## Commands

```bash
# Rust
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-features

# Python (legacy reference package + scripts)
.venv/bin/python -m pytest -q       # or: uv run pytest -q

# One-command solo-maintainer security validation (see scripts/verify_solo_maintainer.py)
python scripts/verify_solo_maintainer.py

# Conductor registry/contracts check
python scripts/verify_frontier_conductor.py
```

## Merge gate (solo maintainer, zero human approvals)

- Default branch `master` is protected: 12 stable required checks (Rustfmt, Clippy Lints,
  Workspace Tests, Package Smoke, Security Audit, Code Coverage, Layered Harness Contract,
  Python Tests, CodeQL, Analyze (python), Analyze (javascript-typescript), Packaging Assets),
  `strict: true`, force-push and deletion blocked, `enforce_admins: true`, an additive
  ruleset blocks `deletion`/`non_fast_forward`, and **zero** required approving reviews.
- Required checks are the merge gate; GitHub Actions are pinned to full commit SHAs;
  `dependency-review` fails PRs on high-severity vulnerabilities; `actionlint` lints
  workflows.
- Master is never pushed to directly: branch → PR → squash-merge (`--auto --delete-branch`).
- Conductor contracts (`scripts/verify_frontier_conductor.py`,
  `conformance/cli/command-name-parity.json`) must pass whenever CLI surface changes.

## Conductor

Plans, specs, and quality tracks live under `.conductor/` (`tracks.md` index, `tracks/<id>/`
per track). Keep the registry in sync with reality; mark tasks `[x]` with a commit
checkpoint when done. Current quality-frontier tracks: `quality-frontier-testing-cicd-20261113`
(#280, complete) and `solo-maintainer-security-20261113` (#281).

## Distribution and registries

Distribution status is tracked in `docs/registry-distribution-matrix.md`. Claiming a channel
"live" requires a verifiable asset (release, package, listing). Do not mark external
submissions (registries, app stores) complete without evidence; #32 tracks the deliberately
open external-registry monitor follow-up.
