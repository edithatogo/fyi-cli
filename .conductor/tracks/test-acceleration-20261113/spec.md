# Bounded Test-Acceleration Profiles

Issue: [fyi-cli #322](https://github.com/edithatogo/fyi-cli/issues/322).

## Goal

Adopt bounded, local-only test-acceleration profiles for the Python suite so the required CI
gate (full clean-environment suite) stays authoritative, while local iteration is faster.
Implementation is repository-specific, following the owner-wide test-acceleration audit
(2026-08-26).

## Recommended bounded work (from issue #322)

- [ ] Add a registered `slow` marker and a fast local profile that excludes it; keep full CI authoritative.
- [ ] Trial `pytest-xdist` with a bounded worker count and isolate resource-sensitive/file-mutating tests into a serial phase.
- [ ] Add an opt-in, serial pytest-testmon profile for changed-code feedback; rebuild its local database after dependency or inventory changes.
- [ ] Trial pytest-gremlins only on a bounded fail-closed/core-logic target set; retain ordinary tests as the promotion gate.
- [ ] For public, replay-safe HTTP only, evaluate redacted VCR.py cassettes; keep in-process transports for deterministic unit tests and never record credentials or restricted payloads.
- [ ] Reuse immutable database fixtures and prefer in-memory databases where file-path/recovery semantics are not under test.
- [ ] On supported Python/Coverage versions, measure the `sysmon` core; do not force it where branch coverage, plugins, concurrency, or dynamic contexts are incompatible.
- [ ] Use Freezegun only where an injectable explicit clock is impractical; avoid global time freezing around asyncio or concurrency.
- [ ] Do not add pytest-picked if testmon provides the needed dependency-aware selection; if retained, use it only to order or select directly changed test files.

## Acceptance criteria

- Record before/after wall time and collected/executed test counts.
- Keep a full clean-environment suite in CI; selective profiles are local feedback only.
- Document incompatibilities and exclusions rather than weakening test, coverage, mutation, rights, or publication gates.

## Non-Goals

- Replacing the required full CI suite with selective profiles (full suite remains the merge gate).
- Weakening coverage, mutation, rights, or publication gates.
- Adding network-dependent tests or recording credentials/restricted payloads in VCR cassettes.
