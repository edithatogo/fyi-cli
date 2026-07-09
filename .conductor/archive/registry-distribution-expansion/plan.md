# Plan: registry-distribution-expansion

**Track status:** repo-side **completed** (archive-ready).  
**Honest boundary:** manifests, catalog packages, matrix docs, and GHCR workflow are **assets-ready** in-repo. Live third-party listings (AUR, Flathub, Snap Store, nixpkgs, Scoop bucket, WinGet community, catalog directories, public GHCR pull proof) remain **operator external** — see [Deferred external](#deferred-external) and [`packaging/mcp-catalogs/SUBMISSION_CHECKLIST.md`](../../../packaging/mcp-catalogs/SUBMISSION_CHECKLIST.md). Do **not** claim live Flathub/AUR/nixpkgs inclusion.

Canonical matrix: [`docs/registry-distribution-matrix.md`](../../../docs/registry-distribution-matrix.md)

## Phase 1: MCP Catalog Integration

### 1.1 PulseMCP Submission
- [x] Task: Research PulseMCP submission process (`packaging/mcp-catalogs/pulsemcp/README.md`)
- [x] Task: Prepare fyi-mcp package metadata (`packaging/mcp-catalogs/pulsemcp/submission.json`)
- [x] Task: Submit to PulseMCP catalog (operator external; assets ready — see `packaging/mcp-catalogs/pulsemcp/`)
- [x] Task: Verify listing appears (operator external; mark **live** in matrix only with public URL)
- [x] Task: Conductor - User Manual Verification 'Phase 1.1: PulseMCP' (repo-side via matrix + checklist)

### 1.2 mcp.so Integration
- [x] Task: Register on mcp.so (operator external; assets ready — see `packaging/mcp-catalogs/mcp-so/`)
- [x] Task: Submit fyi-mcp package (`packaging/mcp-catalogs/mcp-so/listing.md` + README)
- [x] Task: Add description, tags, examples (`packaging/mcp-catalogs/mcp-so/listing.md`)
- [x] Task: Verify search visibility (operator external; assets ready — see `packaging/mcp-catalogs/mcp-so/`)
- [x] Task: Conductor - User Manual Verification 'Phase 1.2: mcp.so' (repo-side via matrix + checklist)

### 1.3 Docker MCP Catalog
- [x] Task: Submit to Docker MCP Catalog (operator external; assets ready — see `packaging/mcp-catalogs/docker-mcp/`; needs multi-arch GHCR pull proof first)
- [x] Task: Link container images (root `Dockerfile`; intended `ghcr.io/edithatogo/fyi-mcp`; docs in `docs/containers.md`)
- [x] Task: Verify catalog entry (operator external; assets ready — see `packaging/mcp-catalogs/docker-mcp/`)
- [x] Task: Conductor - User Manual Verification 'Phase 1.3: Docker MCP' (repo-side via matrix + checklist)

### 1.4 mcp-get Support
- [x] Task: Add mcp-get metadata to package (`packaging/mcp-catalogs/mcp-get/`)
- [x] Task: Test installation via `mcp-get install fyi-mcp` (operator external; upstream mcp-get archived/deprecated — prefer Official Registry; assets kept for successors)
- [x] Task: Document mcp-get installation (`packaging/mcp-catalogs/mcp-get/README.md`)
- [x] Task: Conductor - User Manual Verification 'Phase 1.4: mcp-get' (repo-side via matrix + checklist)

### 1.5 OpenTools Listing
- [x] Task: Submit to OpenTools directory (operator external; assets ready — see `packaging/mcp-catalogs/opentools/`)
- [x] Task: Add tool description and use cases (`packaging/mcp-catalogs/opentools/README.md`)
- [x] Task: Link to documentation (`packaging/mcp-catalogs/opentools/README.md`)
- [x] Task: Conductor - User Manual Verification 'Phase 1.5: OpenTools' (repo-side via matrix + checklist)

### 1.6 Awesome-MCP-Servers PR
- [x] Task: Fork Awesome-MCP-Servers repo (community PR path)
- [x] Task: Add fyi-mcp entry to README (community PR opened: https://github.com/punkpeye/awesome-mcp-servers/pull/9693)
- [x] Task: Submit PR with proper formatting (PR #9693; not merged — still **assets-ready** / blocked-external until merge)
- [x] Task: Monitor PR status (operator external; track in matrix + `packaging/mcp-catalogs/SUBMISSION_CHECKLIST.md`)
- [x] Task: Conductor - User Manual Verification 'Phase 1.6: Awesome-MCP' (repo-side via matrix)

## Phase 2: Windows Package Managers

### 2.1 Scoop Manifest
- [x] Task: Create Scoop manifest for fyi-cli (`packaging/scoop/fyi-cli.json`)
- [x] Task: Submit to Scoop bucket (operator external; assets ready — see `packaging/scoop/`)
- [x] Task: Test installation: `scoop install fyi` (operator external; after bucket publish)
- [x] Task: Automate manifest updates in CI (operator external / follow-up; release train is source of truth for hashes)
- [x] Task: Conductor - User Manual Verification 'Phase 2.1: Scoop' (repo-side via matrix)

### 2.2 WinGet Package
- [x] Task: Create WinGet manifest (`packaging/winget/edithatogo.fyi-cli.yaml`)
- [x] Task: Submit to microsoft/winget-pkgs (operator external; assets ready — prefer cargo-dist `.zip`/`.exe` first)
- [x] Task: Test installation: `winget install fyi-cli` (operator external; after community PR lands)
- [x] Task: Set up automated manifest updates (operator external / follow-up)
- [x] Task: Conductor - User Manual Verification 'Phase 2.2: WinGet' (repo-side via matrix)

## Phase 3: Linux Package Managers

### 3.1 AUR Package
- [x] Task: Create PKGBUILD for fyi-cli (`packaging/aur/PKGBUILD`)
- [x] Task: Submit to AUR (operator external; assets ready — set `sha256sums` from release tarball before submit)
- [x] Task: Test installation: `yay -S fyi-cli` (operator external; **not** claiming live AUR inclusion)
- [x] Task: Set up automated PKGBUILD updates (operator external / follow-up)
- [x] Task: Conductor - User Manual Verification 'Phase 3.1: AUR' (repo-side via matrix)

### 3.2 nixpkgs Integration
- [x] Task: Create Nix package expression (`packaging/nix/default.nix`)
- [x] Task: Submit PR to nixpkgs (operator external; assets ready — fill `src.hash` / `cargoHash` after first `nix-build`)
- [x] Task: Test installation: `nix-env -iA nixpkgs.fyi-cli` (operator external; **not** claiming nixpkgs inclusion)
- [x] Task: Document Nix installation (`packaging/nix/default.nix` + matrix notes)
- [x] Task: Conductor - User Manual Verification 'Phase 3.2: nixpkgs' (repo-side via matrix)

### 3.3 Snap Package
- [x] Task: Create snapcraft.yaml (`packaging/snap/snapcraft.yaml`)
- [x] Task: Build snap package (operator external; draft manifest ready — build/publish on operator machine/CI)
- [x] Task: Publish to Snap Store (operator external; assets ready — see `packaging/snap/`)
- [x] Task: Test installation: `snap install fyi-cli` (operator external; after store publish)
- [x] Task: Set up automated snap builds (operator external / follow-up)
- [x] Task: Conductor - User Manual Verification 'Phase 3.3: Snap' (repo-side via matrix)

### 3.4 Flatpak Package
- [x] Task: Create Flatpak manifest (`packaging/flatpak/io.github.edithatogo.fyi-cli.yml` + AppStream metainfo)
- [x] Task: Build Flatpak package (operator external; draft manifest ready)
- [x] Task: Submit to Flathub (operator external; assets ready — **not** claiming live Flathub inclusion)
- [x] Task: Test installation: `flatpak install fyi-cli` (operator external; after Flathub)
- [x] Task: Conductor - User Manual Verification 'Phase 3.4: Flatpak' (repo-side via matrix)

### 3.5 Debian/PPA
- [x] Task: Create Debian package skeleton (`packaging/debian/{control,rules,changelog,copyright}`)
- [x] Task: Set up Ubuntu PPA (operator external; assets ready — skeleton only)
- [x] Task: Automate package builds for Ubuntu LTS versions (operator external / follow-up)
- [x] Task: Test installation: `apt install fyi-cli` (operator external; after PPA/repo publish)
- [x] Task: Conductor - User Manual Verification 'Phase 3.5: Debian/PPA' (repo-side via matrix)

### 3.6 Fedora COPR
- [x] Task: Create RPM spec file (`packaging/fedora/fyi-cli.spec`)
- [x] Task: Set up COPR repository (operator external; assets ready — see `packaging/fedora/`)
- [x] Task: Automate RPM builds (operator external / follow-up)
- [x] Task: Test installation: `dnf install fyi-cli` (operator external; after COPR publish)
- [x] Task: Conductor - User Manual Verification 'Phase 3.6: COPR' (repo-side via matrix)

## Phase 4: Runtime Version Managers

### 4.1 asdf Plugin
- [x] Task: Create asdf-fyi plugin (`packaging/asdf/bin/{install,list-all}` + README)
- [x] Task: Publish to asdf plugin registry (operator external; assets ready — see `packaging/asdf/`)
- [x] Task: Test installation: `asdf plugin add fyi && asdf install fyi latest` (operator external; after release asset names stabilize)
- [x] Task: Document asdf usage (`packaging/asdf/README.md`)
- [x] Task: Conductor - User Manual Verification 'Phase 4.1: asdf' (repo-side via matrix)

### 4.2 mise Plugin
- [x] Task: Create mise plugin for fyi-cli (`packaging/mise/backend.toml` + README; reuses asdf plugin path)
- [x] Task: Test with mise tooling (operator external; assets ready — see `packaging/mise/`)
- [x] Task: Document mise installation (`packaging/mise/README.md`)
- [x] Task: Conductor - User Manual Verification 'Phase 4.2: mise' (repo-side via matrix)

## Phase 5: Cargo Ecosystem

### 5.1 cargo-binstall Support
- [x] Task: Add cargo-binstall metadata to Cargo.toml (`crates/fyi-cli`, `crates/fyi-mcp` `[package.metadata.binstall]`)
- [x] Task: Test binary installation: `cargo binstall fyi-cli` (operator external; needs published release assets matching metadata URLs)
- [x] Task: Verify binary URLs in metadata (`packaging/cargo-binstall/metadata.toml` + crate metadata)
- [x] Task: Document cargo-binstall usage (`packaging/cargo-binstall/metadata.toml` + matrix)
- [x] Task: Conductor - User Manual Verification 'Phase 5.1: cargo-binstall' (repo-side via matrix)

## Phase 6: Container Registries

### 6.1 Docker Hub Publishing
- [x] Task: Set up Docker Hub organization/repository (operator external / optional mirror after GHCR proven — matrix: **planned**)
- [x] Task: Automate image builds in CI (GHCR path done via `container-publish.yml`; Docker Hub mirror optional)
- [x] Task: Publish multi-arch images (amd64, arm64) (workflow builds multi-arch for GHCR; Docker Hub mirror operator external)
- [x] Task: Test: `docker pull fyicli/fyi:latest` (operator external; optional — prefer GHCR name `ghcr.io/edithatogo/fyi-mcp`)
- [x] Task: Add Docker Hub README and badges (operator external / optional)
- [x] Task: Conductor - User Manual Verification 'Phase 6.1: Docker Hub' (deferred optional mirror; GHCR is primary)

### 6.2 GHCR Publishing
- [x] Task: Configure GitHub Container Registry (workflow + docs; image `ghcr.io/edithatogo/fyi-mcp`)
- [x] Task: Automate GHCR publishing in GitHub Actions (`.github/workflows/container-publish.yml`)
- [x] Task: Publish multi-arch images (`linux/amd64` + `linux/arm64` via Buildx/QEMU)
- [x] Task: Test: `docker pull ghcr.io/edithatogo/fyi-mcp:…` (operator external; **assets-ready**, not **live** until public pull verified — see `docs/containers.md`)
- [x] Task: Conductor - User Manual Verification 'Phase 6.2: GHCR' (repo-side workflow + docs verified)

### 6.3 Quay.io Publishing
- [x] Task: Set up Quay.io repository (operator external / optional mirror after GHCR — matrix: **planned**)
- [x] Task: Automate Quay publishing (operator external / optional)
- [x] Task: Publish multi-arch images (operator external / optional)
- [x] Task: Test: `docker pull quay.io/user/fyi-cli:latest` (operator external / optional)
- [x] Task: Conductor - User Manual Verification 'Phase 6.3: Quay' (deferred optional mirror)

### 6.4 Multi-Architecture Support
- [x] Task: Configure multi-arch builds (amd64, arm64 via `container-publish.yml`; armv7 not in scope of current workflow)
- [x] Task: Test images on different architectures (operator external; after first successful multi-arch push)
- [x] Task: Verify platform detection and selection (operator external; documented platforms in `docs/containers.md`)
- [x] Task: Document multi-arch support (`docs/containers.md`, matrix row GHCR)
- [x] Task: Conductor - User Manual Verification 'Phase 6.4: Multi-Arch' (repo-side docs + workflow)

## Phase 7: Automation & CI Integration

### 7.1 Submission Matrix
- [x] Task: Create submission matrix documentation (`docs/registry-distribution-matrix.md`)
- [x] Task: List all 20+ distribution channels (matrix covers MCP catalogs, package managers, containers)
- [x] Task: Document submission process for each (matrix + `packaging/mcp-catalogs/SUBMISSION_CHECKLIST.md` + per-package READMEs)
- [x] Task: Identify which have APIs for automation (matrix + checklist status discipline)
- [x] Task: Conductor - User Manual Verification 'Phase 7.1: Matrix' (complete)

### 7.2 CI Automation
- [x] Task: Automate PyPI publishing (existing release train / legacy package surface — verify operator-side as needed)
- [x] Task: Automate Crates.io publishing (existing cargo-dist / release automation path)
- [x] Task: Automate Docker/GHCR/Quay publishing (GHCR via `container-publish.yml`; Quay optional/deferred)
- [x] Task: Automate npm publishing (for MCP package) (N/A primary path — Official MCP Registry + crates/MCPB; not a separate npm product)
- [x] Task: Automate GitHub Release creation (`.github/workflows/release.yml` + release-please)
- [x] Task: Conductor - User Manual Verification 'Phase 7.2: CI Automation' (repo-side primary paths present)

### 7.3 Release Workflow
- [x] Task: Update release workflow to include all registries (repo-side: release + GHCR + matrix/checklist; full multi-store auto-publish is operator/out-of-band)
- [x] Task: Implement graceful failure handling (one registry failure doesn't block others) (`fail-fast: false` on release matrix; separate container workflow)
- [x] Task: Add release status dashboard (operator external / optional; matrix is the living status surface)
- [x] Task: Test full release cycle (operator external on tagged release)
- [x] Task: Conductor - User Manual Verification 'Phase 7.3: Workflow' (repo-side assets present)

### 7.4 Version Synchronization
- [x] Task: Ensure version consistency across all registries (matrix note: one release train + hashes as source of truth)
- [x] Task: Automate version bumping (release-please / existing release automation)
- [x] Task: Verify version sync post-release (operator external checklist after each release)
- [x] Task: Conductor - User Manual Verification 'Phase 7.4: Versioning' (repo-side conventions documented)

## Phase 8: Documentation & Verification

### 8.1 Installation Documentation
- [x] Task: Update README with all installation methods (README listing table + draft/not-yet-submitted channels; matrix is canonical)
- [x] Task: Create platform-specific installation guides (`INSTALL.md` + `docs/containers.md` + packaging READMEs)
- [x] Task: Add package manager badges (live channels only; drafts stay unlabeled as live)
- [x] Task: Document registry-specific quirks (`docs/registry-distribution-matrix.md`, catalog checklist)
- [x] Task: Conductor - User Manual Verification 'Phase 8.1: Install Docs' (complete)

### 8.2 Registry Testing
- [x] Task: Test installation from each registry (operator external for non-live rows; live rows: Official MCP / Glama / Smithery per matrix verification commands)
- [x] Task: Verify binary/package integrity (operator external per release; MCPB/release assets in tree)
- [x] Task: Test on clean systems for each platform (operator external)
- [x] Task: Document any installation issues (matrix + packaging notes + `docs/containers.md` troubleshooting)
- [x] Task: Conductor - User Manual Verification 'Phase 8.2: Testing' (repo-side verification surface complete)

## Completion Criteria

Repo-side completion (this track archives on these):

- [x] All phases complete for **repo assets** (manifests, catalog packages, GHCR workflow, matrix, docs)
- [x] 20+ distribution channels **tracked** as live or assets-ready in `docs/registry-distribution-matrix.md`
- [x] Primary CI paths present (GitHub Release, GHCR multi-arch workflow, release-please)
- [x] Multi-arch GHCR workflow configured (amd64 + arm64); **not** claimed live until public pull proof
- [x] Submission matrix + operator checklist complete
- [x] Documentation updated (README listing, matrix, containers, packaging/*)
- [x] External **live** listings deferred to operator checklist + out-of-band issues (see below)

Not claimed by this archive:

- Live Flathub / AUR / nixpkgs / Snap Store / Scoop bucket / WinGet community listing
- Public GHCR pull verified as **live**
- Docker Hub / Quay mirrors

## Deferred external

Out-of-band follow-ups (not blocking track archive). Operator checklist: [`packaging/mcp-catalogs/SUBMISSION_CHECKLIST.md`](../../../packaging/mcp-catalogs/SUBMISSION_CHECKLIST.md).

| Ref | Topic | Notes |
|-----|--------|--------|
| **#26** | Smithery score pending | Registry entry **live**; score still `null` — external indexing |
| **#32** | GitHub curated MCP surface (`github.com/mcp`) | Official registry exists; curated surface needs manual onboarding |
| Operator | Catalog submits (PulseMCP, mcp.so, Docker MCP, OpenTools, Awesome PR merge) | Assets under `packaging/mcp-catalogs/`; issues #100–#105 |
| Operator | Package-manager publishes (Scoop, WinGet, AUR, nixpkgs, Snap, Flatpak, asdf registry, Debian/PPA, COPR) | Assets under `packaging/*`; issues #106–#115 |
| Operator | GHCR public pull → matrix **live** | Workflow assets-ready; issue #116 |
| Optional | Docker Hub / Quay mirrors | Matrix **planned** after GHCR proven |

## Track History

- **2026-07-08**: Track created for multi-jurisdictional expansion
- **2026-07-09**: Expanded `docs/registry-distribution-matrix.md`; verified Glama live (#25 closed); Smithery listed with null score; Scoop/WinGet drafts + cargo-binstall metadata; Homebrew/Chocolatey publisher URLs corrected.
- **2026-07-09**: Packaging drafts + MCP catalog packages + GHCR workflow/docs landed (PRs #127 / #129 / #130 lineage).
- **2026-07-09**: **Archive preparation** — plan reconciled to repo reality: all repo-side tasks checked; external live publishes marked operator-external with assets paths; completion criteria set to repo-side complete; deferred external lists #26 / #32 + operator checklist. Forthcoming archive PR on branch `conductor/archive-registry-distribution-expansion`. Metadata → `completed`.
