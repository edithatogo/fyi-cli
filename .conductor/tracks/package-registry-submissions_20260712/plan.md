# Implementation Plan

## Phase 1: Release-source hardening

- [ ] Task: Make GitHub Releases the reproducible source for all package submissions.
    - [ ] Verify cargo-dist artifacts, SHA-256 manifests, SBOM, signatures, and provenance.
    - [ ] Add a release compatibility table for CLI, MCP binary, MCPB, container, and legacy Python package.
    - [ ] Conductor - User Manual Verification 'Release-source hardening' (Protocol in workflow.md)

## Phase 2: Registry packet completion

- [ ] Task: Complete and validate package-specific metadata.
    - [ ] Fill Homebrew, Chocolatey, Scoop, WinGet, cargo-binstall, AUR, nix, Snap, Flatpak, asdf/mise, Debian, and Fedora packets.
    - [ ] Verify GHCR multi-arch publication and Docker catalog prerequisites.
    - [ ] Add package lint and install smoke checks to CI.
    - [ ] Conductor - User Manual Verification 'Registry packet completion' (Protocol in workflow.md)

## Phase 3: Publication and operational verification

- [ ] Task: Publish eligible packages and verify clean installation.
    - [ ] Submit community packages using least-privilege maintainer accounts.
    - [ ] Record public URLs, package versions, checksums, and review outcomes.
    - [ ] Add scheduled freshness and security monitoring with deterministic alerts.
    - [ ] Conductor - User Manual Verification 'Publication and operational verification' (Protocol in workflow.md)

