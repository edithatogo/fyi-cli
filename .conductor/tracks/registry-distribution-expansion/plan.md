# Plan: registry-distribution-expansion

## Phase 1: MCP Catalog Integration

### 1.1 PulseMCP Submission
- [ ] Task: Research PulseMCP submission process
- [ ] Task: Prepare fyi-mcp package metadata
- [ ] Task: Submit to PulseMCP catalog
- [ ] Task: Verify listing appears
- [ ] Task: Conductor - User Manual Verification 'Phase 1.1: PulseMCP' (Protocol in workflow.md)

### 1.2 mcp.so Integration
- [ ] Task: Register on mcp.so
- [ ] Task: Submit fyi-mcp package
- [ ] Task: Add description, tags, examples
- [ ] Task: Verify search visibility
- [ ] Task: Conductor - User Manual Verification 'Phase 1.2: mcp.so' (Protocol in workflow.md)

### 1.3 Docker MCP Catalog
- [ ] Task: Submit to Docker MCP Catalog
- [ ] Task: Link container images
- [ ] Task: Verify catalog entry
- [ ] Task: Conductor - User Manual Verification 'Phase 1.3: Docker MCP' (Protocol in workflow.md)

### 1.4 mcp-get Support
- [ ] Task: Add mcp-get metadata to package
- [ ] Task: Test installation via `mcp-get install fyi-mcp`
- [ ] Task: Document mcp-get installation
- [ ] Task: Conductor - User Manual Verification 'Phase 1.4: mcp-get' (Protocol in workflow.md)

### 1.5 OpenTools Listing
- [ ] Task: Submit to OpenTools directory
- [ ] Task: Add tool description and use cases
- [ ] Task: Link to documentation
- [ ] Task: Conductor - User Manual Verification 'Phase 1.5: OpenTools' (Protocol in workflow.md)

### 1.6 Awesome-MCP-Servers PR
- [ ] Task: Fork Awesome-MCP-Servers repo
- [ ] Task: Add fyi-mcp entry to README
- [ ] Task: Submit PR with proper formatting
- [ ] Task: Monitor PR status
- [ ] Task: Conductor - User Manual Verification 'Phase 1.6: Awesome-MCP' (Protocol in workflow.md)

## Phase 2: Windows Package Managers

### 2.1 Scoop Manifest
- [x] Task: Create Scoop manifest for fyi-cli
- [ ] Task: Submit to Scoop bucket
- [ ] Task: Test installation: `scoop install fyi`
- [ ] Task: Automate manifest updates in CI
- [ ] Task: Conductor - User Manual Verification 'Phase 2.1: Scoop' (Protocol in workflow.md)

### 2.2 WinGet Package
- [x] Task: Create WinGet manifest
- [ ] Task: Submit to microsoft/winget-pkgs
- [ ] Task: Test installation: `winget install fyi-cli`
- [ ] Task: Set up automated manifest updates
- [ ] Task: Conductor - User Manual Verification 'Phase 2.2: WinGet' (Protocol in workflow.md)

## Phase 3: Linux Package Managers

### 3.1 AUR Package
- [ ] Task: Create PKGBUILD for fyi-cli
- [ ] Task: Submit to AUR
- [ ] Task: Test installation: `yay -S fyi-cli`
- [ ] Task: Set up automated PKGBUILD updates
- [ ] Task: Conductor - User Manual Verification 'Phase 3.1: AUR' (Protocol in workflow.md)

### 3.2 nixpkgs Integration
- [ ] Task: Create Nix package expression
- [ ] Task: Submit PR to nixpkgs
- [ ] Task: Test installation: `nix-env -iA nixpkgs.fyi-cli`
- [ ] Task: Document Nix installation
- [ ] Task: Conductor - User Manual Verification 'Phase 3.2: nixpkgs' (Protocol in workflow.md)

### 3.3 Snap Package
- [ ] Task: Create snapcraft.yaml
- [ ] Task: Build snap package
- [ ] Task: Publish to Snap Store
- [ ] Task: Test installation: `snap install fyi-cli`
- [ ] Task: Set up automated snap builds
- [ ] Task: Conductor - User Manual Verification 'Phase 3.3: Snap' (Protocol in workflow.md)

### 3.4 Flatpak Package
- [ ] Task: Create Flatpak manifest
- [ ] Task: Build Flatpak package
- [ ] Task: Submit to Flathub
- [ ] Task: Test installation: `flatpak install fyi-cli`
- [ ] Task: Conductor - User Manual Verification 'Phase 3.4: Flatpak' (Protocol in workflow.md)

### 3.5 Debian/PPA
- [ ] Task: Create Debian package (.deb)
- [ ] Task: Set up Ubuntu PPA
- [ ] Task: Automate package builds for Ubuntu LTS versions
- [ ] Task: Test installation: `apt install fyi-cli`
- [ ] Task: Conductor - User Manual Verification 'Phase 3.5: Debian/PPA' (Protocol in workflow.md)

### 3.6 Fedora COPR
- [ ] Task: Create RPM spec file
- [ ] Task: Set up COPR repository
- [ ] Task: Automate RPM builds
- [ ] Task: Test installation: `dnf install fyi-cli`
- [ ] Task: Conductor - User Manual Verification 'Phase 3.6: COPR' (Protocol in workflow.md)

## Phase 4: Runtime Version Managers

### 4.1 asdf Plugin
- [ ] Task: Create asdf-fyi plugin
- [ ] Task: Publish to asdf plugin registry
- [ ] Task: Test installation: `asdf plugin add fyi && asdf install fyi latest`
- [ ] Task: Document asdf usage
- [ ] Task: Conductor - User Manual Verification 'Phase 4.1: asdf' (Protocol in workflow.md)

### 4.2 mise Plugin
- [ ] Task: Create mise plugin for fyi-cli
- [ ] Task: Test with mise tooling
- [ ] Task: Document mise installation
- [ ] Task: Conductor - User Manual Verification 'Phase 4.2: mise' (Protocol in workflow.md)

## Phase 5: Cargo Ecosystem

### 5.1 cargo-binstall Support
- [x] Task: Add cargo-binstall metadata to Cargo.toml
- [ ] Task: Test binary installation: `cargo binstall fyi-cli`
- [x] Task: Verify binary URLs in metadata
- [x] Task: Document cargo-binstall usage
- [ ] Task: Conductor - User Manual Verification 'Phase 5.1: cargo-binstall' (Protocol in workflow.md)

## Phase 6: Container Registries

### 6.1 Docker Hub Publishing
- [ ] Task: Set up Docker Hub organization/repository
- [ ] Task: Automate image builds in CI
- [ ] Task: Publish multi-arch images (amd64, arm64)
- [ ] Task: Test: `docker pull fyicli/fyi:latest`
- [ ] Task: Add Docker Hub README and badges
- [ ] Task: Conductor - User Manual Verification 'Phase 6.1: Docker Hub' (Protocol in workflow.md)

### 6.2 GHCR Publishing
- [ ] Task: Configure GitHub Container Registry
- [ ] Task: Automate GHCR publishing in GitHub Actions
- [ ] Task: Publish multi-arch images
- [ ] Task: Test: `docker pull ghcr.io/user/fyi-cli:latest`
- [ ] Task: Conductor - User Manual Verification 'Phase 6.2: GHCR' (Protocol in workflow.md)

### 6.3 Quay.io Publishing
- [ ] Task: Set up Quay.io repository
- [ ] Task: Automate Quay publishing
- [ ] Task: Publish multi-arch images
- [ ] Task: Test: `docker pull quay.io/user/fyi-cli:latest`
- [ ] Task: Conductor - User Manual Verification 'Phase 6.3: Quay' (Protocol in workflow.md)

### 6.4 Multi-Architecture Support
- [ ] Task: Configure multi-arch builds (amd64, arm64, armv7)
- [ ] Task: Test images on different architectures
- [ ] Task: Verify platform detection and selection
- [ ] Task: Document multi-arch support
- [ ] Task: Conductor - User Manual Verification 'Phase 6.4: Multi-Arch' (Protocol in workflow.md)

## Phase 7: Automation & CI Integration

### 7.1 Submission Matrix
- [x] Task: Create submission matrix documentation
- [x] Task: List all 20+ distribution channels
- [x] Task: Document submission process for each
- [x] Task: Identify which have APIs for automation
- [ ] Task: Conductor - User Manual Verification 'Phase 7.1: Matrix' (Protocol in workflow.md)

### 7.2 CI Automation
- [ ] Task: Automate PyPI publishing (existing, verify)
- [ ] Task: Automate Crates.io publishing (existing, verify)
- [ ] Task: Automate Docker/GHCR/Quay publishing
- [ ] Task: Automate npm publishing (for MCP package)
- [ ] Task: Automate GitHub Release creation
- [ ] Task: Conductor - User Manual Verification 'Phase 7.2: CI Automation' (Protocol in workflow.md)

### 7.3 Release Workflow
- [ ] Task: Update release workflow to include all registries
- [ ] Task: Implement graceful failure handling (one registry failure doesn't block others)
- [ ] Task: Add release status dashboard
- [ ] Task: Test full release cycle
- [ ] Task: Conductor - User Manual Verification 'Phase 7.3: Workflow' (Protocol in workflow.md)

### 7.4 Version Synchronization
- [ ] Task: Ensure version consistency across all registries
- [ ] Task: Automate version bumping
- [ ] Task: Verify version sync post-release
- [ ] Task: Conductor - User Manual Verification 'Phase 7.4: Versioning' (Protocol in workflow.md)

## Phase 8: Documentation & Verification

### 8.1 Installation Documentation
- [ ] Task: Update README with all installation methods
- [ ] Task: Create platform-specific installation guides
- [ ] Task: Add package manager badges
- [ ] Task: Document registry-specific quirks
- [ ] Task: Conductor - User Manual Verification 'Phase 8.1: Install Docs' (Protocol in workflow.md)

### 8.2 Registry Testing
- [ ] Task: Test installation from each registry
- [ ] Task: Verify binary/package integrity
- [ ] Task: Test on clean systems for each platform
- [ ] Task: Document any installation issues
- [ ] Task: Conductor - User Manual Verification 'Phase 8.2: Testing' (Protocol in workflow.md)

## Completion Criteria
- [ ] All phases complete
- [ ] 20+ distribution channels active
- [ ] 70%+ automation achieved
- [ ] All container registries publishing
- [ ] Multi-arch images tested
- [ ] CI release workflow complete
- [ ] Documentation updated

## Track History
- **2026-07-08**: Track created for multi-jurisdictional expansion
- **2026-07-09**: Expanded `docs/registry-distribution-matrix.md`; verified Glama live (#25 closed); Smithery listed with null score; Scoop/WinGet drafts + cargo-binstall metadata; Homebrew/Chocolatey publisher URLs corrected.
