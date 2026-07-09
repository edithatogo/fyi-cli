# Installation Guide

Install **fyi-cli** (FOI/OIA CLI) and **fyi-mcp** (Model Context Protocol server) on Windows, macOS, and Linux.

| | |
|--|--|
| **Version (this doc)** | `0.1.2` |
| **Homepage** | https://github.com/edithatogo/fyi-cli |
| **Last updated** | 2026-07-09 |
| **Primary stack** | Rust workspace (`fyi-cli`, `fyi-mcp`, `fyi-core`) |
| **Legacy** | Python package on PyPI (`fyi-cli` / `fyi_system`) — still installable, not extended |

**Status discipline:** many package-manager and catalog channels are **draft / assets-ready** only (manifests live under [`packaging/`](packaging/)). Do **not** treat them as published until the matrix says **live**.

| Doc | Purpose |
|-----|---------|
| [docs/registry-distribution-matrix.md](docs/registry-distribution-matrix.md) | Live vs draft matrix (MCP, package managers, containers) |
| [docs/installation-package-managers.md](docs/installation-package-managers.md) | Per-manager draft notes, local build commands, caveats |
| [packaging/mcp-catalogs/SUBMISSION_CHECKLIST.md](packaging/mcp-catalogs/SUBMISSION_CHECKLIST.md) | MCP catalog submission checklist |
| [docs/containers.md](docs/containers.md) | GHCR / Docker image for `fyi-mcp` |

---

## Recommended paths (quick)

| Goal | Method | Status |
|------|--------|--------|
| Fastest from source | `cargo install --path crates/fyi-cli` | Works from a clone |
| Binary without compiling (when crates.io + release assets align) | `cargo binstall fyi-cli` | **assets-ready** (metadata in-repo) |
| Prebuilt CLI / MCPB | [GitHub Releases](https://github.com/edithatogo/fyi-cli/releases) | Releases live; asset names evolve with cargo-dist |
| MCP in AI clients | Official MCP Registry / Glama / Smithery | **live** (see below) |
| Legacy Python CLI | `pip install fyi-cli` | **live** (legacy) |
| Container MCP | `ghcr.io/edithatogo/fyi-mcp` | **assets-ready** (not claimed live until pull verified) |

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

See [QUICKSTART.md](QUICKSTART.md) and [CONFIGURATION.md](CONFIGURATION.md).

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

- **Python:** 3.10+ (3.11+ recommended)
- **pip** (or `uv` / `pipx`)

```bash
python --version   # or python3 --version
pip --version
```

---

## 1. Cargo install (from source or crates.io)

### From a local clone (always works)

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

### From crates.io

When the crate version you want is published:

```bash
cargo install fyi-cli --locked
# MCP crate name if published separately:
# cargo install fyi-mcp --locked
```

If `cargo install fyi-cli` fails with “could not find”, use a git/path install or GitHub Releases instead. Publication status is tracked in [docs/registry-distribution-matrix.md](docs/registry-distribution-matrix.md).

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

**Status:** **assets-ready** — package metadata lives in crate `Cargo.toml` files and [`packaging/cargo-binstall/metadata.toml`](packaging/cargo-binstall/metadata.toml). Resolution depends on matching GitHub Release asset names.

```bash
# Install cargo-binstall once (if needed)
cargo install cargo-binstall

# Then:
cargo binstall fyi-cli
# cargo binstall fyi-mcp
```

If binstall cannot find a suitable archive, fall back to `cargo install` or manual release download.

---

## 3. GitHub Releases (prebuilt binaries / MCPB)

**Status:** Releases are **live** at https://github.com/edithatogo/fyi-cli/releases  

Download assets for your platform from the release matching **v0.1.2** / **fyi-mcp-v0.1.2** (or newer). Asset naming follows cargo-dist and packaging drafts; names can include target triples or short platform labels. Prefer the release notes for the exact file list.

Typical patterns (examples — check the release page for what actually shipped):

| Kind | Examples |
|------|----------|
| CLI archives | `fyi-cli-<version>-<target>.tar.gz` / `.zip` |
| Installers | shell / PowerShell installers from cargo-dist |
| Windows MCP package | `fyi-mcp-0.1.2-win32.mcpb` (MCPB for MCP clients) |

### Manual install sketch

**Linux / macOS:**

```bash
# After downloading and extracting the archive that contains fyi-cli:
chmod +x fyi-cli
sudo mv fyi-cli /usr/local/bin/   # or any directory on PATH
fyi-cli --help
```

**Windows (PowerShell):**

```powershell
# After extracting fyi-cli.exe to a permanent folder, e.g. $env:LOCALAPPDATA\fyi-cli
$dest = Join-Path $env:LOCALAPPDATA "fyi-cli"
New-Item -ItemType Directory -Force -Path $dest | Out-Null
# Copy-Item .\fyi-cli.exe $dest\
# Add $dest to user PATH if needed, then:
fyi-cli --help
```

cargo-dist may also publish one-liner installers on the release; use those when present and trusted.

---

## 4. Legacy PyPI package

**Status:** **live** — https://pypi.org/project/fyi-cli/  

This installs the **legacy Python** implementation (`fyi_system`), not the Rust workspace. Suitable for existing scripts; new development should prefer Rust `fyi-cli` / `fyi-mcp`.

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

## 5. Package managers (mostly draft / assets-ready)

**None of the following should be assumed published** unless the [distribution matrix](docs/registry-distribution-matrix.md) marks them **live**. Drafts are under [`packaging/`](packaging/). Platform-specific commands and caveats: **[docs/installation-package-managers.md](docs/installation-package-managers.md)**.

| Manager | Draft location | Intended command (when published) | Status |
|---------|----------------|-----------------------------------|--------|
| **Scoop** (Windows) | [`packaging/scoop/fyi-cli.json`](packaging/scoop/fyi-cli.json) | `scoop install fyi-cli` (after bucket submit) | **assets-ready** (currently MCPB-oriented) |
| **WinGet** (Windows) | [`packaging/winget/edithatogo.fyi-cli.yaml`](packaging/winget/edithatogo.fyi-cli.yaml) | `winget install edithatogo.fyi-cli` | **draft** — prefer plain `.zip`/`.exe` before community PR |
| **Homebrew** (macOS/Linux) | [`packaging/homebrew/fyi-cli.rb`](packaging/homebrew/fyi-cli.rb) | `brew install fyi-cli` (tap/formula PR pending) | **assets-ready** (SHA placeholders) |
| **Chocolatey** (Windows) | [`packaging/chocolatey/`](packaging/chocolatey/) | `choco install fyi-cli` | **assets-ready** (draft URLs/checksums) |
| **AUR** (Arch) | [`packaging/aur/PKGBUILD`](packaging/aur/PKGBUILD) | `yay -S fyi-cli` / `makepkg -si` | **draft** |
| **nix** | [`packaging/nix/default.nix`](packaging/nix/default.nix) | local `nix-build` / future nixpkgs | **draft** |
| **Snap** | [`packaging/snap/snapcraft.yaml`](packaging/snap/snapcraft.yaml) | `snap install fyi-cli` | **draft** |
| **Flatpak** | [`packaging/flatpak/`](packaging/flatpak/) | `flatpak install …` | **draft** |
| **Debian / PPA** | [`packaging/debian/`](packaging/debian/) | `apt install fyi-cli` | **draft** skeleton |
| **Fedora / COPR** | [`packaging/fedora/fyi-cli.spec`](packaging/fedora/fyi-cli.spec) | `dnf install fyi-cli` | **draft** |
| **asdf** | [`packaging/asdf/`](packaging/asdf/) | `asdf install fyi-cli 0.1.2` (local plugin) | **draft** |
| **mise** | [`packaging/mise/`](packaging/mise/) | `mise install fyi-cli@0.1.2` (local plugin) | **draft** |
| **cargo-binstall** | [`packaging/cargo-binstall/`](packaging/cargo-binstall/) | `cargo binstall fyi-cli` | **assets-ready** |

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

**Status:** Dockerfile + publish workflow are **assets-ready**. Image name intended: **`ghcr.io/edithatogo/fyi-mcp`**. Do not treat GHCR as live until a public `docker pull` for a published tag is verified.

Full operator guide: **[docs/containers.md](docs/containers.md)**.

```bash
# Local build
docker build -t fyi-mcp:local -f Dockerfile .
docker run --rm -i fyi-mcp:local

# After a successful GHCR publish (when verified live):
# docker pull ghcr.io/edithatogo/fyi-mcp:latest
# docker pull ghcr.io/edithatogo/fyi-mcp:v0.1.2
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
| `cargo install fyi-cli` | `cargo install fyi-cli --locked --force` |
| `cargo binstall` | `cargo binstall fyi-cli` (or reinstall after new release) |
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
| `cargo binstall` finds no package | Release asset names may not match metadata yet — use `cargo install` or Releases |
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
