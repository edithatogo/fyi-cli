# Implementation Plan: Alaveteli Bot Contract

## Phase 1: Rust identity and guardrails

- [~] Issue #141: Enforce traceable identity across Rust HTTP constructors.
- [~] Add validated identity wiring to the Tor-routed reqwest client and preserve the safe default constructor — `45f7e15`.
- [~] Add a deterministic no-network test asserting the Tor client User-Agent; execution is blocked by the missing Windows SDK linker libraries — `45f7e15`.
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

- 2026-07-10: `cargo test -p fyi-core --test tor_tests` cannot link on this
  workstation because the MSVC Windows SDK libraries are unavailable to
  `rust-lld`. The identity slice remains open until the focused test passes in
  a provisioned local or CI environment.
