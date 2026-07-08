# Registry distribution matrix

This repository now tracks an implementation-oriented submission matrix for the multi-jurisdictional FYI CLI rollout.

## Current package and distribution coverage
- Crates.io: published via cargo-dist / release automation
- PyPI: existing package publishing flow
- MCP registry: server metadata is already described in the repository
- Glama and Smithery: badges/docs are already present
- Docker: container publishing templates exist

## Expansion targets
- MCP catalogs: PulseMCP, mcp.so, Docker MCP Catalog, mcp-get, OpenTools, Awesome-MCP-Servers
- Package managers: Scoop, WinGet, AUR, nixpkgs, Snap, Flatpak, asdf/mise, cargo-binstall, Debian/PPA, Fedora/COPR
- Containers: Docker Hub, GHCR, Quay (multi-arch)

## Implementation notes
- Each registry entry should be tracked as a release asset or submission checklist item.
- Release automation should be capable of emitting the metadata required by each catalog.
- The submission matrix should be reviewed before each release candidate.
