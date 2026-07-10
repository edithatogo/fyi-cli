# Implementation Plan: Alaveteli Bot Contract

## Phase 1: Rust identity and guardrails

- [ ] Issue #141: Enforce traceable identity and bounded request guardrails.
- [ ] Audit every Rust HTTP constructor, including API and Tor paths.
- [ ] Add red/green tests for identity rejection, guardrail trips, clean halt, and secret-free errors.
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

