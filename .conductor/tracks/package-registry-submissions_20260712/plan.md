# Implementation Plan

## Phase 1: Release-source hardening

- [~] Task: Make GitHub Releases the reproducible source for all package submissions.
    - [x] Add deterministic SHA-256 and release-manifest generation for published assets.
    - [x] Add GitHub artifact provenance attestation to the release workflow.
    - [x] Add a release compatibility table for CLI, MCP binary, MCPB, container, and legacy Python package.
    - [x] Group Release Please components and reject partial Rust release scopes in CI. (edd6fe9)
    - [ ] Conductor - User Manual Verification 'Release-source hardening' (Protocol in workflow.md)

## Phase 2: Registry packet completion

- [~] Task: Complete and validate package-specific metadata.
    - [x] Declare package assets, release gating, help-only smoke commands, and no-write guarantees in a machine-readable matrix.
    - [x] Validate the install-smoke matrix in CI.
    - [x] Add a manual GHCR multi-arch verification workflow and fail-closed inspector.
    - [ ] Verify GHCR multi-arch publication and Docker catalog prerequisites.
    - [x] Add package contract and install-smoke checks to CI.
    - [ ] Conductor - User Manual Verification 'Registry packet completion' (Protocol in workflow.md)

## Phase 3: Publication and operational verification

- [~] Task: Publish eligible packages and verify clean installation.
    - [ ] Submit community packages using least-privilege maintainer accounts.
    - [ ] Record public URLs, package versions, checksums, and review outcomes.
    - [x] Add scheduled freshness monitoring with deterministic, credential-free alerts.
    - [ ] Conductor - User Manual Verification 'Publication and operational verification' (Protocol in workflow.md)
