# Implementation Plan: Alaveteli Bot Contract

## Phase 1: Rust identity and guardrails

- [x] Issue #141: Enforce traceable identity across Rust HTTP constructors.
- [x] Add validated identity wiring to the Tor-routed reqwest client and preserve the safe default constructor — `45f7e15`.
- [x] Add deterministic identity-header and Tor-client construction tests — `be9604d`; verified with the user-scoped MSVC toolchain.
- [~] Issue #145: Enforce resource guardrails at every Rust send boundary; stacked
  fork-local PR #147 depends on identity PR #146.
- [x] Route all `SyncClient` request, health-check, and retry send paths through
  one guarded executor — `c30b9f4`.
- [x] Bound response buffering before accounting bytes and enforce request-count,
  response-byte, runtime, and concurrency limits at that boundary — `c30b9f4`.
- [x] Add red/green wiremock tests proving request-count and response-byte trips
  block the next remote call without exposing response content — `c30b9f4`.
- [ ] Integrate and test the same executor contract for Tor/proxy-assisted Rust
  sends and complete the all-constructor audit.
- [x] Working tree was clean before the guardrail slice; no unrelated changes
  were included.

## Phase 2: Back-pressure parity

- [~] Issue #142: Parse and honor Alaveteli rate and advisory headers; stacked
  fork-local PR #150 depends on PR #147.
- [x] Align Rust and Python snapshots for standard rate-limit fields and
  `X-Advisory-Status`, including degraded pacing — `37677ca`.
- [x] Add shared offline fixtures and parity tests for valid and malformed
  headers; reject negative unsigned values consistently — `37677ca`.
- [ ] Complete cross-path 429 halt, HTTP-date, bounded-jitter, and instance
  isolation evidence against the full client contract.

## Phase 3: Cache and bounded bulk mode

- [ ] Issue #143: Add conditional caching and bounded bulk export mode.
- [ ] Test 304 cache hits, stale/error behavior, no stale writes, and bounded request/byte/time/batch controls.
- [ ] Ensure unavailable or unauthorized bulk export cannot fall back to unbounded retrieval.

## Phase 4: Cross-repo verification

- [ ] Issue #144: Run fork-local Alaveteli contract verification and reconcile both tracks.
- [ ] Consume shared offline fixtures and run opt-in bounded live smoke only when explicitly enabled.
- [ ] Close only when every known risk is fixed, verified false positive, or blocked by a dated disabled follow-up.

## PR standard

One child issue maps to one PR. Each PR must state scope, non-scope, test-first
evidence, security/quality sensors, rollback, and harness changes. Parent and
paired issue links are mandatory. No PR may include unrelated refactoring or
the pre-existing dirty `agent_runtime.rs` change unless explicitly assigned.

## Verification blockers

- [x] Provision a non-admin MSVC/Windows SDK toolchain under
  `%USERPROFILE%\msvc_portable` and verify it through
  `scripts/verify_msvc_portable.ps1`.
- [x] Re-run the focused identity test through
  `scripts/Invoke-MsvcPortable.ps1` and remove this blocker only after it
  passes: 5 passed, 0 failed on 2026-07-10.

## Verification record

- `cl.exe`: Microsoft C/C++ Optimizing Compiler `19.44.35228` for x64.
- Command: `pwsh -NoProfile -File scripts/verify_msvc_portable.ps1`.
- Command: `pwsh -NoProfile -File scripts/Invoke-MsvcPortable.ps1 cargo test --target x86_64-pc-windows-msvc -p fyi-core --test tor_tests`.
- Result: 5 passed, 0 failed; no elevation, registry, system environment, or
  `Program Files` writes were used.

## Guardrail slice verification

- Fork-local draft PR: https://github.com/edithatogo/fyi-cli/pull/147
- Scoped command: `pwsh -NoProfile -File scripts/Invoke-MsvcPortable.ps1 cargo test --locked -p fyi-core sync::tests --lib`.
- Result: 28 passed, 0 failed.
- Full scoped core library command: `pwsh -NoProfile -File scripts/Invoke-MsvcPortable.ps1 cargo test --locked -p fyi-core --lib`.
- Full result: 115 passed, 0 failed.
- Scoped check: `cargo check --locked -p fyi-core` completed successfully.
- Remaining closure blocker: Tor/proxy-assisted send integration and its
  deterministic boundary tests are intentionally still open.

## Back-pressure slice verification

- Fork-local draft PR: https://github.com/edithatogo/fyi-cli/pull/150
- Paired Alaveteli fixture issue: https://github.com/edithatogo/alaveteli/issues/25
- Scoped Rust library result: 117 passed, 0 failed.
- Python full result: 604 passed, 1 opt-in live smoke skipped.
- Repository Ruff still reports pre-existing baseline findings; this slice did
  not broaden that unrelated lint debt.
