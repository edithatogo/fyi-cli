# Package and distribution registry submissions

## Overview

Turn the existing release artifacts into an evidence-backed submission program for package managers, container registries, and developer distribution channels.

## Targets

Crates.io, PyPI legacy maintenance, GHCR, Homebrew, Chocolatey, Scoop, WinGet, cargo-binstall, AUR, nixpkgs, Snap, Flatpak, asdf/mise, Debian/PPA, Fedora/COPR, and Docker Hub/Quay where justified.

## Requirements

- Use one signed/versioned GitHub release as the source of truth for binaries, hashes, SBOM, provenance, and changelog.
- Separate assets-ready from submitted, accepted, live, rejected, and blocked-external.
- Avoid publishing placeholders, unverifiable hashes, mutable URLs, or packages that cannot be reproduced.
- Prefer package-native security controls: signed metadata, least-privilege workflows, provenance attestations, dependency scanning, and rollback instructions.
- Record maintainer accounts, submission URLs, review state, and public install verification without exposing secrets.

## Acceptance criteria

- Each target has a completed packet or an explicit not-applicable/deferred rationale.
- At least one clean release is verified through each channel before status is live.
- CI rejects stale versions, missing hashes, malformed manifests, and unsupported platform claims.
- The runbook documents how to update, yank, roll back, and monitor each publication.

## Out of scope

- AI plugin directory submissions covered by the sibling track.
- Alaveteli maintenance or provider-adapter implementation.

