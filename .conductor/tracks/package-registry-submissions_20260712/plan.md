# Implementation Plan

## 2026-11-14 local reconciliation

- [x] Reconcile the public release, component versions, registry identity and native package targets.
- [x] Repair install metadata, reproducible source templates and checksum-enforced installers.
    - [x] Replace guessed asdf downloads with committed checksum approvals, exact platforms, bounded safe archive inspection, version checks and atomic installs.
    - [x] Harden deterministic release manifests and add read-only exact-byte verification; signature/SBOM/native install evidence remains separate.
- [x] Extend the existing Packaging Assets gate with exact manifest, release-evidence,
  submission-readiness and clean-install contract checks; do not change shared workflows.
- [x] Record verified local evidence, registry-specific submission/rollback packets and exact
  operator blockers; leave publication and native clean-install verification open.

Local implementation and regression evidence is in [acceptance-evidence.md](./acceptance-evidence.md).
Per-channel packets, prerequisites and rollback actions are in
[the operator runbook](../../../packaging/SUBMISSION_RUNBOOK.md). Repository
validation does not complete the external acceptance criteria below.

## Phase 1: Release-source hardening

- [~] Task: Make GitHub Releases the reproducible source for all package submissions.
    - [x] Add deterministic SHA-256 and release-manifest generation for published assets.
    - [x] Add GitHub artifact provenance attestation to the release workflow.
    - [x] Add a release compatibility table for CLI, MCP binary, MCPB, container, and legacy Python package.
    - [x] Group Release Please components and reject partial Rust release scopes in CI. (edd6fe9)
    - [x] Add a credential-free, cross-platform release preflight and hosted-MCP deployment contract. (89ed0b0)
    - [ ] **Blocked — release operator:** select the next coordinated version without
      retagging `v1.2.0`; qualify actual bytes, signed release, aggregate manifest,
      SBOM and authenticated provenance from the same source/build. The historical
      release contains CLI component `0.1.2` and only four CLI archives plus sidecars.
    - [ ] Conductor - User Manual Verification 'Release-source hardening' (Protocol in workflow.md); requires the preceding evidence, not another local implementation approval.

## Phase 2: Registry packet completion

- [~] Task: Complete and validate package-specific metadata.
    - [x] Declare package assets, release gating, help-only smoke commands, and no-write guarantees in a machine-readable matrix.
    - [x] Validate the install-smoke matrix in CI.
    - [x] Add a manual GHCR multi-arch verification workflow and fail-closed inspector.
    - [ ] **Blocked — container operator:** verify public GHCR index/child digests,
      amd64/arm64 pulls, isolated help, SBOM and provenance. Defer Docker Hub/Quay
      unless a consumer and maintainer justify a mirror.
    - [x] Add package contract and install-smoke checks to CI.
    - [ ] **Blocked — native builders/maintainers:** resolve crate ownership/names,
      vendor/source hashes and dependency licence closure; run the runbook's native
      package lint, build, install/help/uninstall and confinement checks. Record
      evidence for every advertised platform before promoting a packet to ready.
    - [ ] Conductor - User Manual Verification 'Registry packet completion' (Protocol in workflow.md); native results and registry ownership are outstanding.

## Phase 3: Publication and operational verification

- [~] Task: Publish eligible packages and verify clean installation.
    - [ ] **Blocked — registry operators:** submit only qualified packets using
      owned maintainer accounts and record actual submission URLs. Flathub is not
      applicable to the console application; private Flatpak remains optional.
    - [ ] **Blocked — registry reviewers/operators:** record public package/version
      URLs, independently checked digests, review outcomes and clean public installs.
    - [x] Add scheduled freshness monitoring with deterministic, credential-free alerts.
    - [ ] Conductor - User Manual Verification 'Publication and operational verification' (Protocol in workflow.md); no external submission or new listing has been performed or claimed.

Issue #194 remains open with `blocked-external`; the track is not archived or
marked wholly complete. Issue #202's previous closure covered its historical
local release-hardening slice and does not prove the operator gates above.
