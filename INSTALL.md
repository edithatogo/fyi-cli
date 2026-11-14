# Installation Guide

Install **fyi-cli** (FOI/OIA CLI) and **fyi-mcp** (Model Context Protocol server) on Windows, macOS, and Linux.

| | |
|--|--|
| **Version (this doc)** | `0.1.2` |
| **Homepage** | https://github.com/edithatogo/fyi-cli |
| **Last updated** | 2026-11-14 |
| **Primary stack** | Rust workspace (`fyi-cli`, `fyi-mcp`, `fyi-core`) |
| **Legacy** | Python package (`fyi-cli` / `fyi_system`) — frozen for existing consumers during migration |

**Distribution status:** Rust package-manager and container channels are **blocked** pending release qualification and native evidence. Reviewable metadata under [`packaging/`](packaging/) is not a published package. MCP/AI catalog statuses are maintained separately in the distribution matrix.

| Doc | Purpose |
|-----|---------|
| [docs/registry-distribution-matrix.md](docs/registry-distribution-matrix.md) | Dated channel evidence and blockers (MCP, package managers, containers) |
| [docs/installation-package-managers.md](docs/installation-package-managers.md) | Per-manager packet scope and outstanding validation |
| [packaging/mcp-catalogs/SUBMISSION_CHECKLIST.md](packaging/mcp-catalogs/SUBMISSION_CHECKLIST.md) | MCP catalog submission checklist |
| [docs/containers.md](docs/containers.md) | GHCR / Docker image for `fyi-mcp` |

---

## Recommended paths (quick)

| Goal | Method | Status |
|------|--------|--------|
| Fastest from source | `cargo install --path crates/fyi-cli` | Works from a clone |
| Crates.io / cargo-binstall | Blocked | `fyi-cli` and `fyi-core` are occupied; ownership and a matching release must be verified |
| Historical CLI / separate MCPB assets | [GitHub Releases](https://github.com/edithatogo/fyi-cli/releases) | Public assets observed; native installation and integrity qualification remain |
| MCP in AI clients | Official MCP Registry / Glama / Smithery | **live** (see below) |
| Legacy Python CLI | `pip install fyi-cli` | **live** (legacy) |
| Container MCP | `ghcr.io/edithatogo/fyi-mcp` | **Blocked:** public multi-arch pulls, provenance and clean smoke unverified |

```bash
# Clone + install CLI from this workspace
git clone https://github.com/edithatogo/fyi-cli.git
cd fyi-cli
cargo install --path crates/fyi-cli --locked
fyi-cli --help

# Optional: MCP server binary
cargo install --path crates/fyi-mcp --locked
fyi-mcp --help
```

After install, typical first steps:

```bash
fyi-cli init-db
fyi-cli --help
```

The [legacy quick start](QUICKSTART.md) describes the Python application. Rust
request management, reporting and archive capture are not yet equivalent.

---

## Prerequisites

### Rust CLI / MCP (recommended)

- **Rust toolchain:** stable Rust (rustup recommended): https://rustup.rs/
- **OS:** Windows 10+, macOS 11+, or a recent Linux distribution
- **Disk:** enough space for a Cargo build (~several hundred MB for deps) or a small footprint if using prebuilt binaries only
- **Optional:** OpenSSL/dev headers on some Linux distros when building from source; Tor if you use Tor routing

```bash
rustc --version
cargo --version
```

### Legacy Python package only

- **Python:** use a qualified repository-local Python 3.15 for any retained development or automation
- **pip** (or `uv` / `pipx`)

```bash
python --version   # or python3 --version
pip --version
```

---

## 1. Cargo install from source

### From a local clone

```bash
git clone https://github.com/edithatogo/fyi-cli.git
cd fyi-cli

# CLI only
cargo install --path crates/fyi-cli --locked

# MCP server only
cargo install --path crates/fyi-mcp --locked

# Or build both without installing into ~/.cargo/bin
cargo build --workspace --release --locked
./target/release/fyi-cli --help
./target/release/fyi-mcp --help   # Windows: target\release\fyi-cli.exe
```

### Crates.io name collision

Do not install this project with `cargo install fyi-cli` or `cargo binstall fyi-cli`:
the existing `fyi-cli` and `fyi-core` index entries do not establish this project's
ownership or a matching 0.1.2 release. Resolve the package identity first.
Use the local clone above until this project's release identity is resolved and
verified. See the [distribution matrix](docs/registry-distribution-matrix.md).

### Development build (contributors)

```bash
git clone https://github.com/edithatogo/fyi-cli.git
cd fyi-cli
cargo build --workspace
cargo test --workspace
```

See [CONTRIBUTING.md](CONTRIBUTING.md).

---

## 2. cargo-binstall (prebuilt, no local compile)

[cargo-binstall](https://github.com/cargo-bins/cargo-binstall) downloads release binaries instead of compiling.

**Status:** blocked by the crates.io name collision and unqualified asset mapping.
All default Binstall strategies are disabled in the repository metadata. An explicit
operator strategy override can bypass that upstream default, so resolve crate
identity and qualify the release before enabling or attempting an install.
The policy metadata is not evidence that the current crate names resolve to this
project. See the [submission runbook](packaging/SUBMISSION_RUNBOOK.md).

---

## 3. GitHub Releases (historical CLI archives / separate MCPB)

The observed public CLI release is
[**v1.2.0**](https://github.com/edithatogo/fyi-cli/releases/tag/v1.2.0),
published **10 July 2026**. Its source commit,
`86fcbd8914037735b6f27e5a329d5f3013033efd`, declares Rust package version
**0.1.2**. The release tag and component version are different identifiers;
`v0.1.2` tag/release lookups returned 404 on 14 November 2026.
These archives are historical and do not represent current `master`.

| Platform | Observed CLI archive |
| --- | --- |
| Linux x86_64 | [fyi-cli-linux-amd64.tar.gz](https://github.com/edithatogo/fyi-cli/releases/download/v1.2.0/fyi-cli-linux-amd64.tar.gz) |
| macOS x86_64 | [fyi-cli-macos-amd64.tar.gz](https://github.com/edithatogo/fyi-cli/releases/download/v1.2.0/fyi-cli-macos-amd64.tar.gz) |
| macOS arm64 | [fyi-cli-macos-arm64.tar.gz](https://github.com/edithatogo/fyi-cli/releases/download/v1.2.0/fyi-cli-macos-arm64.tar.gz) |
| Windows x64 | [fyi-cli-windows-amd64.exe.zip](https://github.com/edithatogo/fyi-cli/releases/download/v1.2.0/fyi-cli-windows-amd64.exe.zip) |

Each archive has a `.sha256` sidecar. The eight observed release assets contain
no Linux arm64 CLI archive, standalone MCP server archive, shell/PowerShell
installer, `SHA256SUMS`, `release-manifest.json` or SBOM.

The repository's binary package metadata uses these actual URLs and GitHub
API-observed SHA-256 values. This audit has **not independently verified
downloaded bytes, native installation, a signed release or artifact provenance**.
The [release evidence record](packaging/release-evidence.json) and
[submission runbook](packaging/SUBMISSION_RUNBOOK.md) separate those outstanding
checks from the fact that the release page is public. Qualify the archive,
binary version and platform before installing or submitting it through a package
manager; a matching API hash does not establish authenticated provenance.

Historical Windows MCPB packages, including `fyi-mcp-0.1.2-win32.mcpb`, belong
to the separate
[`fyi-mcp-v0.1.2` release](https://github.com/edithatogo/fyi-cli/releases/tag/fyi-mcp-v0.1.2).
They are MCP client bundles, not CLI archives or standalone executable assets.
The current Scoop, WinGet and Chocolatey metadata references the CLI ZIP above.

---

## 4. Legacy PyPI package

**Status:** **live** — https://pypi.org/project/fyi-cli/  

This installs the **legacy Python** implementation (`fyi_system`), not the Rust workspace. It is retained for existing consumers while selected functionality migrates. The published PyPI release and the Git revision pinned by `fyi-archive` are separate artifacts.

```bash
pip install fyi-cli
# or: pipx install fyi-cli
# or: uv tool install fyi-cli

fyi --version   # Python entrypoint name may be `fyi` depending on packaging
```

Upgrade / remove:

```bash
pip install --upgrade fyi-cli
pip uninstall fyi-cli
```

Optional: Conda packaging notes remain in [CONDA_PUBLISHING.md](CONDA_PUBLISHING.md) (not the primary path).

---

## 5. Package managers (blocked pending release qualification)

**No current Rust package-manager packet is submission-ready, submitted or
verified live.** Reviewable metadata and blocked source templates live under
[`packaging/`](packaging/). The
[package-manager guide](docs/installation-package-managers.md) explains their
scope; the [submission runbook](packaging/SUBMISSION_RUNBOOK.md) gives exact
operator validation, submission and rollback steps.

| Manager | Reviewable files | Current status |
| --- | --- | --- |
| Scoop | [Manifest](packaging/scoop/fyi-cli.json) | **Blocked:** actual x64 CLI ZIP/hash; byte verification, clean install and bucket review remain. |
| WinGet | [Manifest](packaging/winget/edithatogo.fyi-cli.yaml) | **Blocked:** ZIP with nested portable `fyi-cli.exe`; native validation/install and community review remain. |
| Homebrew maintainer tap | [Formula](packaging/homebrew/fyi-cli.rb) | **Blocked:** concrete archive URLs/hashes; native audit/install/test and an owned tap remain. This is not a Homebrew/core formula. |
| Chocolatey | [Package directory](packaging/chocolatey/) | **Blocked:** concrete CLI ZIP metadata; native pack/install/uninstall and moderation remain. |
| AUR | [PKGBUILD](packaging/aur/PKGBUILD) | **Blocked:** source/vendor hashes, clean chroot build and `.SRCINFO` remain. |
| nixpkgs | [Expression](packaging/nix/default.nix) | **Blocked:** NAR/vendor hashes, pinned builder, native Linux tests and maintainer remain. |
| Snap | [Manifest](packaging/snap/snapcraft.yaml) | **Blocked:** hashed vendor input, native confinement tests and publisher/name verification remain. |
| Flathub | [Private Flatpak notes](packaging/flatpak/README.md) | **Not applicable:** current inclusion policy excludes this console product. |
| Private Flatpak | [Template](packaging/flatpak/) | **Blocked:** source/vendor hashes, qualified runtime and sandbox smoke remain. |
| Debian / PPA | [Source-package files](packaging/debian/) | **Blocked:** dependency/licence review, native validation and signing/sponsorship remain. |
| Fedora / COPR | [Spec](packaging/fedora/fyi-cli.spec) | **Blocked:** source/vendor hashes, SRPM/mock validation and operator project remain. |
| asdf / mise | [asdf](packaging/asdf/) / [mise](packaging/mise/) | **Blocked:** approval inventory and standalone plugin repository remain. No Cargo fallback or occupied-crate backend is offered. |
| cargo-binstall | [Policy metadata](packaging/cargo-binstall/) | **Blocked:** default strategies disabled until crate identity and release assets are qualified. |

---

## 6. MCP: registries, catalogs, and `fyi-mcp` binary

### Live listings (verified)

| Channel | Identifier | Link |
|---------|------------|------|
| **Official MCP Registry** | `io.github.edithatogo/fyi-mcp` @ `0.1.2` | [registry.modelcontextprotocol.io](https://registry.modelcontextprotocol.io/) · repo [`server.json`](server.json) |
| **Glama** | `edithatogo/fyi-cli` (`fyi-mcp`) | [glama.ai/mcp/servers/edithatogo/fyi-cli](https://glama.ai/mcp/servers/edithatogo/fyi-cli) |
| **Smithery** | `@edithatogo/fyi-mcp` | [smithery.ai/server/@edithatogo/fyi-mcp](https://smithery.ai/server/@edithatogo/fyi-mcp) |

Install / connect using your client’s registry UI or by pointing stdio at a local binary (below). Official registry packages currently include a Windows **MCPB** asset for `0.1.2`; see `server.json` for the exact URL and SHA-256.

### Run the MCP server binary yourself

```bash
# From source
cargo install --path crates/fyi-mcp --locked
fyi-mcp

# Or from a release MCPB / extracted binary on Windows
# (see GitHub Releases: fyi-mcp-0.1.2-win32.mcpb)
```

Example client config (stdio):

```json
{
  "mcpServers": {
    "fyi-mcp": {
      "command": "fyi-mcp",
      "args": []
    }
  }
}
```

### Catalog submissions (not all live)

Additional directories (PulseMCP, mcp.so, Docker MCP Catalog, etc.) are tracked as **assets-ready** packages under [`packaging/mcp-catalogs/`](packaging/mcp-catalogs/). Operator steps:

- [packaging/mcp-catalogs/SUBMISSION_CHECKLIST.md](packaging/mcp-catalogs/SUBMISSION_CHECKLIST.md)
- [docs/registry-distribution-matrix.md](docs/registry-distribution-matrix.md)

Do not claim those catalogs are listed publicly until the matrix row is **live** with a public URL.

---

## 7. Containers (GHCR)

**Status:** **blocked**. The intended image is **`ghcr.io/edithatogo/fyi-mcp`**. The Dockerfile and publication workflow exist, but public amd64/arm64 pulls, immutable digest/source binding, SBOM, provenance and isolated clean-install smoke remain unverified. See the [submission runbook](packaging/SUBMISSION_RUNBOOK.md).

Full operator guide: **[docs/containers.md](docs/containers.md)**.

```bash
# Local build
docker build -t fyi-mcp:local -f Dockerfile .
docker run --rm -i fyi-mcp:local

# After the operator qualifies an actual published immutable image:
# docker pull ghcr.io/edithatogo/fyi-mcp@sha256:<verified-digest>
```

---

## Verify installation

```bash
# Rust CLI
fyi-cli --help
fyi-cli init-db

# Rust MCP (speaks MCP on stdio — use with a client, or smoke-run carefully)
fyi-mcp --help 2>/dev/null || true

# Legacy Python (if installed)
fyi --version
```

---

## Upgrade

| Install method | Upgrade |
|----------------|---------|
| `cargo install --path …` | `git pull` then re-run `cargo install --path crates/fyi-cli --locked --force` |
| GitHub Releases | Download newer assets; replace binary on PATH |
| PyPI | `pip install --upgrade fyi-cli` |
| Package managers | Use the manager’s upgrade command once published |

Backup local DB/config before major upgrades when you care about existing request data (see [USER_GUIDE.md](USER_GUIDE.md)).

---

## Uninstall

```bash
# Cargo-installed binaries (default: ~/.cargo/bin)
rm "$(which fyi-cli)" "$(which fyi-mcp)" 2>/dev/null
# Windows: remove %USERPROFILE%\.cargo\bin\fyi-cli.exe and fyi-mcp.exe

# PyPI
pip uninstall fyi-cli

# Optional local data (destructive)
# rm -rf ~/.fyi   # only if you no longer need local databases/config
```

Package managers: use `scoop uninstall`, `winget uninstall`, `brew uninstall`, etc., once those packages are actually installed from a published source.

---

## Troubleshooting

| Symptom | What to try |
|---------|-------------|
| `cargo install` fails on OpenSSL/linker | Install system build deps (`build-essential` + `libssl-dev` on Debian/Ubuntu; Xcode CLT on macOS; MSVC Build Tools on Windows) |
| `cargo binstall` is unavailable | Default strategies are disabled; crate names are occupied and identity is unverified. Use the local workspace install while identity and release assets are qualified |
| Wrong binary (`fyi` vs `fyi-cli`) | Python package may expose `fyi`; Rust binary is `fyi-cli` |
| MCP client can’t start server | Confirm `fyi-mcp` is on PATH; for Docker see [docs/containers.md](docs/containers.md) |
| Permission errors on pip | Prefer `pip install --user` or `pipx` / a venv — avoid `sudo pip` |
| Draft `brew`/`winget`/`choco` commands fail | Expected until packages are published — use Cargo or Releases |

More: [TROUBLESHOOTING.md](TROUBLESHOOTING.md), [FAQ.md](FAQ.md).

---

## System requirements (summary)

| | Minimum | Notes |
|--|---------|--------|
| OS | Windows 10, macOS 11+, modern Linux | |
| Rust install | Stable rustc/cargo | For source / cargo paths |
| Python | 3.10+ | Legacy PyPI path only |
| RAM | ~512 MB runtime | Builds need more |
| Network | Optional | Needed for Alaveteli API / registry installs |

---

## Next steps

1. [QUICKSTART.md](QUICKSTART.md) — first commands  
2. [CONFIGURATION.md](CONFIGURATION.md) / [API_KEY_SETUP.md](API_KEY_SETUP.md) — API keys and config  
3. [USER_GUIDE.md](USER_GUIDE.md) — day-to-day use  
4. [crates/fyi-mcp/README.md](crates/fyi-mcp/README.md) — MCP server details  
5. [docs/registry-distribution-matrix.md](docs/registry-distribution-matrix.md) — what’s live vs draft  

**Support:** [GitHub Issues](https://github.com/edithatogo/fyi-cli/issues) · [Discussions](https://github.com/edithatogo/fyi-cli/discussions) · [Docs site](https://edithatogo.github.io/fyi-cli/)
