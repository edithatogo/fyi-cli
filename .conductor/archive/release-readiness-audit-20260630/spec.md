# Specification: release-readiness-audit-20260630

## Overview
Reconcile the repository's release-facing documentation, packaging metadata, CI workflows, and command references with the current Rust-first FYI CLI. The repo still contains legacy Python-era release text, placeholder GitHub URLs, and mixed install/test instructions that can mislead users and release automation.

## Functional Requirements

### Phase 1: Release Surface Inventory
1. Audit README, install docs, release docs, changelog, Cargo metadata, GitHub workflow files, and package/distribution configuration.
2. Identify stale Python-first commands, placeholder repository URLs, obsolete badges, and release status claims.
3. Produce a short machine-readable or markdown release-readiness checklist.

### Phase 2: Documentation Alignment
1. Update user-facing quick-start, install, testing, and command references to distinguish Rust CLI, legacy Python modules, and archive commands accurately.
2. Replace placeholder GitHub URLs and badge targets with repository-correct values.
3. Keep privacy, ethics, and no-mass-requesting guidance visible in release-facing docs.

### Phase 3: Packaging And CI Verification
1. Verify Cargo workspace metadata, cargo-dist configuration, and release workflow assumptions.
2. Add or repair CI checks for formatting, clippy, tests, security audit, and packaging smoke tests where feasible.
3. Document any release blockers that require credentials or external services.

### Phase 4: Release Candidate Handoff
1. Produce a release-candidate checklist with exact local verification commands.
2. Ensure release instructions are non-interactive and Windows-aware.
3. Confirm no generated profiling/build artifacts are left tracked accidentally.

## Non-Functional Requirements
- Preserve existing legacy documentation where it remains useful, but label legacy paths clearly.
- Avoid changing product behavior in this track unless required for packaging smoke tests.
- Prefer automated checks that can run without secrets or network-dependent publishing.

## Acceptance Criteria
- All release-facing links and badges resolve to the current repository or are explicitly marked as placeholders.
- README and installation docs accurately describe the Rust workspace and current command names.
- CI/release checklist includes exact commands for `fmt`, `clippy`, tests, audit, and package smoke checks.
- Any remaining release blockers are documented with owner/action/verification notes.
