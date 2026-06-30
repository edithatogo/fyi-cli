# Plan: release-readiness-audit-20260630

## Phase 1: Release Surface Inventory
- [x] Task: Audit release-facing docs and metadata [d23dcee]
    - [x] Write a checklist test or script that detects placeholder repository URLs and stale command names
    - [x] Inventory README, install docs, release docs, Cargo metadata, and workflow files
    - [x] Document release blockers and stale surfaces
- [x] Task: Conductor - User Manual Verification 'Release Surface Inventory' (Protocol in workflow.md) [b2b69a3]

## Phase 2: Documentation Alignment
- [x] Task: Align user-facing docs with the Rust-first CLI [b2b69a3]
    - [x] Add or update tests/checks for command examples where feasible
    - [x] Update README and installation/testing guidance
    - [x] Preserve and label legacy Python guidance where still relevant
- [x] Task: Repair repository links and badges [b2b69a3]
    - [x] Replace placeholder GitHub URLs and badge targets
    - [x] Verify all changed markdown links that point to local files
- [x] Task: Conductor - User Manual Verification 'Documentation Alignment' (Protocol in workflow.md) [b2b69a3]

## Phase 3: Packaging And CI Verification
- [x] Task: Verify Cargo release metadata and packaging smoke path [b2b69a3]
    - [x] Add or update a non-publishing packaging smoke check
    - [x] Verify cargo-dist configuration assumptions
    - [x] Document secret-gated publishing steps separately from local checks
- [x] Task: Harden CI quality gates [b2b69a3]
    - [x] Ensure fmt, clippy, tests, and audit/package checks are represented
    - [x] Keep network or credential dependent steps opt-in
- [x] Task: Conductor - User Manual Verification 'Packaging And CI Verification' (Protocol in workflow.md) [b2b69a3]

## Phase 4: Release Candidate Handoff
- [x] Task: Produce release-candidate checklist [b2b69a3]
    - [x] Add exact Windows/GNU Rust verification commands
    - [x] Include artifact cleanup and generated-file checks
    - [x] Record remaining blockers with owner/action/verification
- [x] Task: Conductor review [b2b69a3]
    - [x] Run conductor-review for release-readiness-audit-20260630
    - [x] Apply any fix recommendations
    - [x] Push to GitHub
- [x] Task: Conductor - User Manual Verification 'Release Candidate Handoff' (Protocol in workflow.md) [b2b69a3]

## Archive
- [x] Archive track: move to archive/ directory
