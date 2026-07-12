# Implementation Plan

## Phase 1: Inventory and submission packets

- [ ] Task: Reconcile README, registry matrix, manifests, release assets, and current public listings.
    - [ ] Add separate Codex and Anthropic-specific child submission packets and review checklists.
    - [ ] Record authentication, write-action, privacy, support, and rollback disclosures.
    - [ ] Conductor - User Manual Verification 'Inventory and submission packets' (Protocol in workflow.md)

## Phase 2: Automated evidence and validation

- [ ] Task: Add deterministic packaging and metadata checks for every target.
    - [ ] Validate the machine-readable registry ledger, versions, hashes, URLs, licenses, manifests, and reproducible release references.
    - [ ] Add public-status probes with bounded timeouts and fingerprinted evidence.
    - [ ] Add negative tests proving a target cannot be marked live without evidence.
    - [ ] Conductor - User Manual Verification 'Automated evidence and validation' (Protocol in workflow.md)

## Phase 3: External submissions and follow-up

- [ ] Task: Submit eligible packets through each target's documented route.
    - [ ] File Codex review and Anthropic Connector submissions with operator-visible confirmation.
    - [ ] File catalog/community submissions and link resulting PRs or tickets.
    - [ ] Update the matrix only from public evidence; schedule follow-ups for blocked targets.
    - [ ] Stop adding targets unless they have a documented audience, submission route, maintainer, and measurable user value.
    - [ ] Conductor - User Manual Verification 'External submissions and follow-up' (Protocol in workflow.md)
