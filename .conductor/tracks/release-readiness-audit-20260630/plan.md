# Plan: release-readiness-audit-20260630

## Phase 1: Release Surface Inventory
- [x] Task: Audit release-facing docs and metadata [d23dcee]
    - [x] Write a checklist test or script that detects placeholder repository URLs and stale command names
    - [x] Inventory README, install docs, release docs, Cargo metadata, and workflow files
    - [x] Document release blockers and stale surfaces
- [~] Task: Conductor - User Manual Verification 'Release Surface Inventory' (Protocol in workflow.md)

## Phase 2: Documentation Alignment
- [ ] Task: Align user-facing docs with the Rust-first CLI
    - [ ] Add or update tests/checks for command examples where feasible
    - [ ] Update README and installation/testing guidance
    - [ ] Preserve and label legacy Python guidance where still relevant
- [ ] Task: Repair repository links and badges
    - [ ] Replace placeholder GitHub URLs and badge targets
    - [ ] Verify all changed markdown links that point to local files
- [ ] Task: Conductor - User Manual Verification 'Documentation Alignment' (Protocol in workflow.md)

## Phase 3: Packaging And CI Verification
- [ ] Task: Verify Cargo release metadata and packaging smoke path
    - [ ] Add or update a non-publishing packaging smoke check
    - [ ] Verify cargo-dist configuration assumptions
    - [ ] Document secret-gated publishing steps separately from local checks
- [ ] Task: Harden CI quality gates
    - [ ] Ensure fmt, clippy, tests, and audit/package checks are represented
    - [ ] Keep network or credential dependent steps opt-in
- [ ] Task: Conductor - User Manual Verification 'Packaging And CI Verification' (Protocol in workflow.md)

## Phase 4: Release Candidate Handoff
- [ ] Task: Produce release-candidate checklist
    - [ ] Add exact Windows/GNU Rust verification commands
    - [ ] Include artifact cleanup and generated-file checks
    - [ ] Record remaining blockers with owner/action/verification
- [ ] Task: Conductor review
    - [ ] Run conductor-review for release-readiness-audit-20260630
    - [ ] Apply any fix recommendations
    - [ ] Push to GitHub
- [ ] Task: Conductor - User Manual Verification 'Release Candidate Handoff' (Protocol in workflow.md)

## Archive
- [ ] Archive track: move to archive/ directory
