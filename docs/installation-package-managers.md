# Package manager distribution status

Reviewed **14 November 2026**. Package metadata version: **0.1.2**.

**No current Rust package-manager or container channel is ready for submission
or verified as live.** The repository contains reviewable binary metadata and
blocked source-build templates. Their presence is not evidence that a registry
has accepted a package or that a clean installation works.

Use [INSTALL.md](../INSTALL.md#1-cargo-install-from-source) for the local Rust
workspace installation path. The public [PyPI package](https://pypi.org/project/fyi-cli/)
is legacy Python version **1.0.0**, frozen for migration; it is a different artifact.

The [submission runbook](../packaging/SUBMISSION_RUNBOOK.md) is the authoritative
operator guide for validation, submissions, updates, withdrawal and rollback.
The [submission ledger](../packaging/registry-submissions.json) records each
target's owner, blockers and exact next action.

## Which release the metadata describes

The observed [GitHub release v1.2.0](https://github.com/edithatogo/fyi-cli/releases/tag/v1.2.0)
contains four historical CLI archives for Linux x86_64, macOS x86_64,
macOS arm64 and Windows x64, plus SHA-256 sidecars. The source at its pinned
commit declares Rust package version **0.1.2**. There is no observed `v0.1.2`
tag/release, Linux arm64 CLI archive or standalone MCP archive in that release.

Homebrew, Chocolatey, Scoop and WinGet metadata now reference those actual
archives and API-observed hashes. They are blocked pending independent byte
verification, release/version reconciliation, SBOM/provenance and native
clean-install checks. They must not be described as current `master` binaries.
See [release evidence](../packaging/release-evidence.json).

## Channel inventory

All rows marked **blocked** lack at least one release, native-validation,
maintainer or submission prerequisite. No registry installation command is
provided here as a working public installation route.

| Channel | Reviewable files | Status and next prerequisite |
| --- | --- | --- |
| crates.io | Workspace Cargo manifests | **Blocked:** `fyi-cli` and `fyi-core` are occupied; project ownership and a matching release must be verified before publishing. |
| GHCR | Root Dockerfile and existing verification workflow | **Blocked:** credential-free amd64/arm64 pulls, digest/source binding, SBOM, provenance and clean smoke unverified. |
| Homebrew maintainer tap | [Formula](../packaging/homebrew/fyi-cli.rb) | **Blocked:** actual binary URLs/hashes are present; native audit/install/test and an owned tap remain. This is not a Homebrew/core source formula. |
| Chocolatey | [Package directory](../packaging/chocolatey/) | **Blocked:** Windows CLI ZIP metadata, licence, verification and uninstall notes exist; native package/install tests and moderation remain. |
| Scoop | [Manifest](../packaging/scoop/fyi-cli.json) | **Blocked:** x64 CLI ZIP and `fyi-cli.exe`, not MCPB; verify bytes and clean install before a bucket PR. |
| WinGet | [Manifest](../packaging/winget/edithatogo.fyi-cli.yaml) | **Blocked:** ZIP with nested portable `fyi-cli.exe`; native validation/install and community review remain. |
| cargo-binstall | [Policy metadata](../packaging/cargo-binstall/metadata.toml) | **Blocked:** all default strategies are disabled while crate identity and asset mapping remain unresolved. |
| AUR | [PKGBUILD](../packaging/aur/PKGBUILD) | **Blocked:** immutable source is pinned; source/vendor hashes, clean Arch build and generated `.SRCINFO` remain. |
| nixpkgs | [Expression](../packaging/nix/default.nix) | **Blocked:** actual source NAR/vendor hashes, a pinned builder, a maintainer and native Linux tests remain. |
| Snap | [Manifest and notes](../packaging/snap/) | **Blocked:** provide hashed vendoring; validate native builds and strict confinement; operator must verify publisher/name. |
| Flathub | [Private Flatpak notes](../packaging/flatpak/README.md) | **Not applicable:** current Flathub policy excludes this console product. |
| Private Flatpak | [Template](../packaging/flatpak/io.github.edithatogo.fyi-cli.yml) | **Blocked:** source/vendor hashes, exact SDK/runtime qualification and sandbox installation remain. |
| asdf / mise | [asdf](../packaging/asdf/) / [mise](../packaging/mise/) | **Blocked:** approved version/checksum inventory and a standalone plugin repository are absent. |
| Debian / PPA | [Source-package files](../packaging/debian/) | **Blocked:** dependency/licence closure, offline native build, Lintian/autopkgtest and maintainer signing remain. A PPA is distinct from Debian archive acceptance. |
| Fedora / COPR | [Spec](../packaging/fedora/fyi-cli.spec) | **Blocked:** source/vendor hashes, SRPM/mock validation and an owned COPR project remain. Fedora archive review is separate. |

### Crate names and version managers

Do not install this project through the occupied crates.io name, including the
`cargo:fyi-cli` mise backend. The asdf installer has no Cargo fallback and accepts
only versions/platforms whose hashes are in its reviewed approval inventory;
the current empty inventory intentionally prevents installation. The scripts
under `packaging/asdf/` are not a published plugin repository.
`packaging/mise/backend.toml` is an inventory, not a registered mise backend.

Binstall's disabled strategies protect its default resolution path. An operator
can explicitly override strategies upstream, so this is not an absolute block
against manual overrides. Resolve crate ownership and qualify a synchronized
release before enabling exact per-target metadata.

### Source-build templates

Source templates require immutable source, a matching Cargo.lock, independently
recorded source/vendor hashes, dependency licence review and qualified native
builders. Frozen commands fail when prerequisites or tests fail. Unresolved
`@…@` hash tokens are explicit blockers and must not enter submitted packages.
No native build or installation success is inferred from repository tests.

## Operator validation

Start with the repository checks documented in the
[runbook](../packaging/SUBMISSION_RUNBOOK.md#repository-validation-and-submission-gate).
They check consistency; they do not publish or qualify a release.

```bash
python scripts/verify_packaging_assets.py
python scripts/validate_registry_submission_ledger.py
python scripts/validate_install_smoke_matrix.py
python scripts/release_preflight.py --json
```

The submission-readiness gate currently exits with failure and records the
remaining prerequisites:

```bash
python scripts/verify_packaging_assets.py --require-submission-ready --target homebrew
```

Follow the runbook's target-specific native build/install/uninstall steps only
after qualifying the release. Smoke execution uses a clean user environment,
no credentials or production data, networking disabled, and exact help-only
commands. Record real platform, version, digest and exit-status evidence.
A successful metadata parse or synthetic archive test is not that evidence.

## MCP catalogs and related guides

MCP/AI catalog statuses remain separately maintained in the
[distribution matrix](registry-distribution-matrix.md#mcp-registries--catalogs)
and [MCP catalog checklist](../packaging/mcp-catalogs/SUBMISSION_CHECKLIST.md).
A catalog listing does not establish an OS package-manager installation.

- [Installation guide](../INSTALL.md)
- [Container operator guide](containers.md)
- [Submission runbook](../packaging/SUBMISSION_RUNBOOK.md)
- [Distribution matrix](registry-distribution-matrix.md)
