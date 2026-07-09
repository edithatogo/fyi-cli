# Specification: registry-distribution-expansion

## Overview
This track expands the registry and distribution footprint beyond the currently integrated registries (PyPI, Crates.io, official MCP registry, Glama, Smithery, conda, Homebrew/Chocolatey, Docker). It adds MCP catalogs, additional package managers, and container registries to maximize discoverability and ease of installation.

**Completion split:** this track is **repo-side complete** when manifests, catalog packages, CI/workflows, and the distribution matrix exist and status discipline is honest (`live` vs `assets-ready` vs `planned`). **External live** listings (third-party review, store publishes, public image pulls) are operator out-of-band work and do not block track archive.

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
   - Docker Hub (official Docker registry) — optional mirror
   - GHCR (GitHub Container Registry) — primary
   - Quay.io (Red Hat container registry) — optional mirror
   - Multi-architecture support (amd64, arm64; armv7 optional/not required for archive)
4. **Submission Automation:**
   - CI automation where registry APIs exist
   - Generate submission matrix documentation
   - Automate release publishing to primary registries (GitHub Releases, GHCR)
   - Version synchronization conventions across registries

## Non-Functional Requirements
- **Automation:** Primary release paths automated in CI; store/catalog submits may remain manual
- **Maintenance:** Minimal manual intervention for releases; operator checklist for external rows
- **Reliability:** Failed external registry submissions don't block core releases
- **Coverage:** 20+ distribution channels **tracked** (live or assets-ready)
- **Honesty:** Never mark Flathub/AUR/nixpkgs/Snap Store/GHCR as **live** without public proof in the matrix

## Acceptance Criteria

### Repo-side (required to archive this track)
- [x] MCP catalog **submission packages** present under `packaging/mcp-catalogs/` (PulseMCP, mcp.so, Docker MCP, mcp-get, OpenTools) + shared checklist
- [x] Package-manager **manifests/skeletons** present under `packaging/` (Scoop, WinGet, AUR, nix, Snap, Flatpak, asdf/mise, cargo-binstall, Debian, Fedora; plus existing Homebrew/Chocolatey)
- [x] GHCR multi-arch **workflow + docs** present (`container-publish.yml`, `docs/containers.md`, root `Dockerfile`) — status **assets-ready** until public pull verified
- [x] Submission matrix documentation complete (`docs/registry-distribution-matrix.md`)
- [x] CI paths for GitHub Releases (+ release-please) and GHCR present
- [x] Documentation updated with installation methods and honest draft vs live listing status (README + matrix + packaging READMEs)

### External live (operator checklist; **not** required to archive)
- [ ] All MCP catalogs **publicly list** fyi-cli/fyi-mcp (PulseMCP, mcp.so, Docker MCP Catalog, OpenTools; Awesome PR merged)
- [ ] All target package managers **publish** fyi-cli (Scoop bucket, WinGet, AUR, nixpkgs, Snap Store, Flathub, asdf registry, PPA/COPR as applicable)
- [ ] Container images **publicly pullable** from GHCR (then optional Docker Hub / Quay mirrors)
- [ ] Multi-architecture images built, pushed, and smoke-tested on amd64 + arm64
- [ ] External rows upgraded from **assets-ready** → **live** only with URL/API proof in the matrix
- [ ] Out-of-band issues still open until resolved: **#26** (Smithery score), **#32** (GitHub curated MCP surface)

## Out of Scope
- Platform-specific installers (DMG for macOS, MSI for Windows) - future enhancement
- App store submissions (Microsoft Store, Snap Store verification) - manual process
- Marketing/promotion of releases
- Claiming live third-party inclusion without matrix evidence

## Dependencies
- None (independent track)

## Success Metrics
- **Registry Count:** 20+ distribution channels tracked (live **or** assets-ready)
- **Repo-side completeness:** packaging assets + matrix + GHCR workflow
- **Automation Rate:** primary release/GHCR paths automated; external stores via operator checklist
- **Installation Ease:** users can install via live channels today; draft channels documented as assets-ready
- **CI Reliability:** core release success not blocked by external catalog failures
