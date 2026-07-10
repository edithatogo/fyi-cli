# Implementation Plan: Alaveteli Bot Contract

## Phase 1: Rust identity and guardrails

- [x] Issue #141: Enforce traceable identity across Rust HTTP constructors.
- [x] Add validated identity wiring to the Tor-routed reqwest client and preserve the safe default constructor — `45f7e15`.
- [x] Add deterministic identity-header and Tor-client construction tests — `be9604d`; verified with the user-scoped MSVC toolchain.
- [ ] Issue #145: Enforce resource guardrails at every Rust send boundary.
- [ ] Audit every remaining Rust HTTP constructor and send path for guardrail coverage.
- [ ] Add red/green tests for guardrail trips, clean halt, and secret-free errors.
- [ ] Preserve and separately account for any pre-existing working-tree change.

## Phase 2: Back-pressure parity

- [ ] Issue #142: Parse and honor Alaveteli rate and advisory headers.
- [ ] Cover Rust primary paths and Python discovery/client paths.
- [ ] Test 429 halt, delta-seconds and HTTP-date Retry-After, bounded jitter fallback, malformed headers, and instance isolation.

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
