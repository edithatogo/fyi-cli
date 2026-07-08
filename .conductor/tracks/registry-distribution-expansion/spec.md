# Specification: registry-distribution-expansion

## Overview
This track expands the registry and distribution footprint beyond the currently integrated registries (PyPI, Crates.io, official MCP registry, Glama, Smithery, conda, Homebrew/Chocolatey, Docker). It adds MCP catalogs, additional package managers, and container registries to maximize discoverability and ease of installation.

## Functional Requirements
1. **MCP Catalog Expansion:**
   - PulseMCP (community MCP catalog)
   - mcp.so (MCP discovery site)
   - Docker MCP Catalog
   - mcp-get (CLI package manager for MCP servers)
   - OpenTools (AI tools directory)
   - Awesome-MCP-Servers PR (GitHub curated list)
2. **Package Manager Integration:**
   - Scoop (Windows package manager)
   - WinGet (Microsoft's official Windows package manager)
   - AUR (Arch User Repository)
   - nixpkgs (NixOS package collection)
   - Snap (Ubuntu/Linux snap packages)
   - Flatpak (Linux universal packages)
   - asdf/mise plugin (multi-runtime version manager)
   - `cargo-binstall` metadata (fast binary installation for Rust crates)
   - Debian/Ubuntu PPA (Personal Package Archive)
   - Fedora COPR (community build system)
3. **Container Registry Expansion:**
   - Docker Hub (official Docker registry)
   - GHCR (GitHub Container Registry)
   - Quay.io (Red Hat container registry)
   - Multi-architecture support (amd64, arm64, armv7)
4. **Submission Automation:**
   - CI automation where registry APIs exist
   - Generate submission matrix documentation
   - Automate release publishing to all registries
   - Version synchronization across registries

## Non-Functional Requirements
- **Automation:** 70%+ of registry submissions automated in CI
- **Maintenance:** Minimal manual intervention for releases
- **Reliability:** Failed registry submissions don't block releases
- **Coverage:** Available in 20+ distribution channels

## Acceptance Criteria
- All MCP catalogs include fyi-cli/fyi-mcp
- All package managers have fyi-cli available
- Container images published to Docker Hub, GHCR, Quay
- Multi-architecture container images built and tested
- Submission matrix documentation complete
- CI automation for all API-accessible registries
- Release workflow includes all registries
- Documentation updated with installation methods

## Out of Scope
- Platform-specific installers (DMG for macOS, MSI for Windows) - future enhancement
- App store submissions (Microsoft Store, Snap Store verification) - manual process
- Marketing/promotion of releases

## Dependencies
- None (independent track)

## Success Metrics
- **Registry Count:** 20+ distribution channels
- **Automation Rate:** 70%+ automated submissions
- **Installation Ease:** User can install via preferred package manager
- **CI Reliability:** 95%+ release success rate
