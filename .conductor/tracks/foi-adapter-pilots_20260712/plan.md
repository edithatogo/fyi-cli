# Implementation Plan

## Phase 1: Contract design

- [~] Task: Define normalized provider capabilities and provenance rules for non-Alaveteli APIs.
    - [ ] Refactor the Alaveteli-specific `FoiProvider` return types behind a provider-neutral boundary (issue #203).
    - [ ] Add fixtures and schemas for requests, authorities, messages, attachments, status transitions, and provider capability flags.
    - [ ] Define provider-specific privacy, identity, billing, and write gates.
    - [ ] Conductor - User Manual Verification 'Contract design' (Protocol in workflow.md)

## Phase 2: Read-only pilots

- [~] Task: Implement MuckRock and FragDenStaat read-only adapters.
    - [~] Add Rust provider implementations and instance configuration.
        - [x] Add the MuckRock read-only provider boundary and community instance metadata.
        - [x] Add the FragDenStaat read-only provider boundary using the documented API v1 routes.
    - [x] Add offline contract, integration, edge, and security tests for the MuckRock slice.
    - [x] Add bounded opt-in live smoke tests with public endpoints only.
    - [x] Add schema-drift fingerprints and fail-closed behavior for changed response contracts.
    - [ ] Conductor - User Manual Verification 'Read-only pilots' (Protocol in workflow.md)

## Phase 3: Official government API evaluation

- [ ] Task: Evaluate FOIA.gov and USCIS as separately governed providers.
    - [ ] Implement FOIA.gov agency/catalog read-only discovery only if its API contract is stable.
    - [ ] Produce a USCIS sandbox access and consent readiness checklist; write capabilities are completely excluded from this pilot.
    - [ ] Conductor - User Manual Verification 'Official government API evaluation' (Protocol in workflow.md)

## Phase 4: Quality and release

- [ ] Task: Complete provider compatibility and operational hardening.
    - [ ] Run full harness, mutation, performance, security, and compatibility gates.
    - [ ] Document rollout, rollback, rate limits, source attribution, and provider deprecation handling.
    - [ ] Conductor - User Manual Verification 'Quality and release' (Protocol in workflow.md)
