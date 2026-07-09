# Package manager install notes (draft / assets-ready)

Platform-specific notes for installing **fyi-cli** / **fyi-mcp** via package managers and version managers.

| | |
|--|--|
| **Version** | `0.1.2` |
| **Homepage** | https://github.com/edithatogo/fyi-cli |
| **Last updated** | 2026-07-09 |
| **Parent guide** | [INSTALL.md](../INSTALL.md) |
| **Status matrix** | [registry-distribution-matrix.md](./registry-distribution-matrix.md) |

> **Important:** Almost every channel below is **draft** or **assets-ready** only. Manifests live in-repo under [`packaging/`](../packaging/). Do **not** treat `brew install`, `winget install`, `choco install`, Snap Store, Flathub, AUR, nixpkgs, etc. as publicly available until the matrix marks the row **live** with evidence.

**Preferred production paths today:** Cargo (path/git/crates.io), [GitHub Releases](https://github.com/edithatogo/fyi-cli/releases), and live MCP registries. See [INSTALL.md](../INSTALL.md).

---

## Status legend

| Status | Meaning |
|--------|---------|
| **live** | Public install works from the official registry |
| **assets-ready** | Repo has manifests/metadata; external submit or release asset shape pending |
| **draft** | Skeleton only; hashes/URLs placeholders or not submitted |
| **planned** | Tracked; not started |

---

## Windows

### Scoop — **assets-ready**

| | |
|--|--|
| Manifest | [`packaging/scoop/fyi-cli.json`](../packaging/scoop/fyi-cli.json) |
| Version in draft | `0.1.2` |
| Current asset | Points at published Windows **MCPB** (`fyi-mcp-0.1.2-win32.mcpb`) — MCP-oriented, not a full CLI zip story |

**When a bucket is published:**

```powershell
# Example only — bucket name TBD after community/self-hosted bucket publish
scoop bucket add <bucket> <repo-url>
scoop install fyi-cli
```

**Local testing of the manifest (operator):**

```powershell
# From a clone — Scoop can install from a local JSON path in some workflows;
# prefer copying into a private bucket for real testing.
scoop install .\packaging\scoop\fyi-cli.json   # may require bucket layout; adjust as needed
```

**Caveats**

- Prefer plain CLI `.zip`/`.exe` from cargo-dist for Scoop UX; the current draft tracks MCPB because that asset is what release automation has shipped for Windows MCP.
- Update `hash` / `url` on every release before submit.

### WinGet — **draft / assets-ready**

| | |
|--|--|
| Manifest | [`packaging/winget/edithatogo.fyi-cli.yaml`](../packaging/winget/edithatogo.fyi-cli.yaml) |
| PackageIdentifier | `edithatogo.fyi-cli` |
| Version | `0.1.2` |

**Intended (after microsoft/winget-pkgs accept):**

```powershell
winget install edithatogo.fyi-cli
```

**Caveats**

- Draft installer URL currently references the MCPB asset; WinGet portable installs work better with a plain `fyi-cli` `.zip`/`.exe`.
- Do not open a community PR until `InstallerUrl` / `InstallerSha256` match a stable cargo-dist Windows CLI asset.
- Validate with `winget validate` / `winget install --manifest` locally before submission.

### Chocolatey — **assets-ready** (draft URLs)

| | |
|--|--|
| Package | [`packaging/chocolatey/fyi-cli.nuspec`](../packaging/chocolatey/fyi-cli.nuspec) |
| Install script | [`packaging/chocolatey/tools/chocolateyinstall.ps1`](../packaging/chocolatey/tools/chocolateyinstall.ps1) |

**Intended (after community or org push):**

```powershell
choco install fyi-cli
```

**Local pack/test (maintainers):**

```powershell
cd packaging/chocolatey
# Update version, URL, checksum in tools/chocolateyinstall.ps1 first
choco pack
# choco install fyi-cli -s . -y
```

**Caveats**

- Install script may still contain placeholder checksums / older URL patterns — align with the GitHub Release before any public package.
- nuspec version and script `$version` must match.

---

## macOS / Linux (Homebrew)

### Homebrew — **assets-ready**

| | |
|--|--|
| Formula draft | [`packaging/homebrew/fyi-cli.rb`](../packaging/homebrew/fyi-cli.rb) |
| Version | `0.1.2` |
| Homepage | https://github.com/edithatogo/fyi-cli |

**Intended (after tap or homebrew-core):**

```bash
brew install fyi-cli
# or: brew install edithatogo/tap/fyi-cli   # if using a personal/org tap
```

**Local formula test:**

```bash
brew install --build-from-source ./packaging/homebrew/fyi-cli.rb
# or copy into a tap repository and `brew install --formula path/to/fyi-cli.rb`
```

**Caveats**

- `sha256` values are **PLACEHOLDER_…** — must be filled from release assets before publish.
- URL pattern in the formula assumes named tarballs (`fyi-cli-macos-amd64.tar.gz`, etc.); confirm against actual Release assets.
- Formula installs binary as `fyi` (symlink/name in formula); Rust default binary name is `fyi-cli` — keep docs and formula in sync when submitting.

---

## Arch Linux (AUR) — **draft**

| | |
|--|--|
| PKGBUILD | [`packaging/aur/PKGBUILD`](../packaging/aur/PKGBUILD) |
| pkgver | `0.1.2` |
| Builds | `fyi-cli` + `fyi-mcp` from tagged source |

**Local build (no AUR publish required):**

```bash
cd packaging/aur
# Set sha256sums from: curl -sL <source-url> | sha256sum
makepkg -si
```

**After AUR publish (example helpers):**

```bash
yay -S fyi-cli
# or: paru -S fyi-cli
```

**Caveats**

- `sha256sums=('SKIP')` is intentional for the draft only — never submit to AUR with `SKIP` without a deliberate VCS package design.
- Source expects tag `v0.1.2` archive layout `fyi-cli-${pkgver}/`.

---

## Nix — **draft**

| | |
|--|--|
| Expression | [`packaging/nix/default.nix`](../packaging/nix/default.nix) |
| Version | `0.1.2` |

**Local build:**

```bash
nix-build -E 'with import <nixpkgs> {}; callPackage ./packaging/nix/default.nix {}'
```

**Caveats**

- `src.hash` and `cargoHash` are placeholders / `lib.fakeHash` — fill after the first failed build prints the expected hashes.
- Not in nixpkgs until an upstream PR lands; matrix row stays draft/assets-ready until then.

---

## Snap — **draft**

| | |
|--|--|
| Manifest | [`packaging/snap/snapcraft.yaml`](../packaging/snap/snapcraft.yaml) |
| Version | `0.1.2` |
| Apps | `fyi-cli`, `fyi-mcp` |

**Local build:**

```bash
cd packaging/snap   # or point snapcraft at the yaml from repo root per your layout
snapcraft
# sudo snap install --dangerous fyi-cli_*.snap
```

**Intended (after Snap Store review):**

```bash
sudo snap install fyi-cli
```

**Caveats**

- `grade: devel` in the draft — raise for stable store listing.
- Strict confinement plugs (`home`, `network`, …) may need iteration for real workloads.

---

## Flatpak — **draft**

| | |
|--|--|
| Manifest | [`packaging/flatpak/io.github.edithatogo.fyi-cli.yml`](../packaging/flatpak/io.github.edithatogo.fyi-cli.yml) |
| Metainfo | [`packaging/flatpak/io.github.edithatogo.fyi-cli.metainfo.xml`](../packaging/flatpak/io.github.edithatogo.fyi-cli.metainfo.xml) |
| App ID | `io.github.edithatogo.fyi-cli` |

**Local build/install:**

```bash
flatpak-builder --user --install --force-clean build-dir \
  packaging/flatpak/io.github.edithatogo.fyi-cli.yml
flatpak run io.github.edithatogo.fyi-cli --help
```

**Caveats**

- CLI/MCP tool without a GUI; Flathub acceptance may need a clear CLI-app story.
- Not on Flathub until submitted and approved.

---

## Debian / Ubuntu (source package skeleton) — **draft**

| | |
|--|--|
| Files | [`packaging/debian/`](../packaging/debian/) (`control`, `rules`, `changelog`, `copyright`) |
| Binary packages | `fyi-cli`, `fyi-mcp` |

**Maintainer-oriented build sketch** (requires a full Debian source tree layout; this is a skeleton):

```bash
# After integrating packaging/debian into a proper source package:
# dpkg-buildpackage -us -uc
# sudo apt install ./fyi-cli_*.deb ./fyi-mcp_*.deb
```

**Caveats**

- No PPA or Debian archive package is claimed available.
- Prefer Cargo/Releases for end users until a repo is published.

---

## Fedora / COPR — **draft**

| | |
|--|--|
| Spec | [`packaging/fedora/fyi-cli.spec`](../packaging/fedora/fyi-cli.spec) |
| Version | `0.1.2` |
| Subpackage | `fyi-mcp` |

**Local RPM build sketch:**

```bash
# After Source0 is fetched and paths match rpmbuild layout:
rpmbuild -ba packaging/fedora/fyi-cli.spec
```

**Intended (after COPR/Fedora):**

```bash
sudo dnf install fyi-cli
# sudo dnf install fyi-mcp
```

---

## asdf — **draft**

| | |
|--|--|
| Plugin scripts | [`packaging/asdf/`](../packaging/asdf/) |
| README | [`packaging/asdf/README.md`](../packaging/asdf/README.md) |

**Local plugin from this repo:**

```bash
mkdir -p ~/.asdf/plugins/fyi-cli
cp -r packaging/asdf/* ~/.asdf/plugins/fyi-cli/

asdf install fyi-cli 0.1.2
asdf global fyi-cli 0.1.2
fyi-cli --help
```

**Caveats**

- Not a published community plugin repo until someone hosts it.
- Prebuilt URL patterns must track Release asset names; install script may fall back to Cargo.

---

## mise — **draft**

| | |
|--|--|
| Notes | [`packaging/mise/README.md`](../packaging/mise/README.md) |
| Metadata | [`packaging/mise/backend.toml`](../packaging/mise/backend.toml) |

**Options:**

```bash
# 1) asdf-compatible plugin from this tree
mise plugins install fyi-cli "$PWD/packaging/asdf"
mise install fyi-cli@0.1.2
mise use -g fyi-cli@0.1.2

# 2) cargo backend (when crate is on crates.io)
mise use cargo:fyi-cli@0.1.2

# 3) ubi / GitHub releases when asset names are stable — see packaging/mise/README.md
```

---

## cargo-binstall — **assets-ready**

Documented also in [INSTALL.md](../INSTALL.md#2-cargo-binstall-prebuilt-no-local-compile).

| | |
|--|--|
| Reference metadata | [`packaging/cargo-binstall/metadata.toml`](../packaging/cargo-binstall/metadata.toml) |
| Wired in | `crates/fyi-cli/Cargo.toml`, `crates/fyi-mcp/Cargo.toml` (`package.metadata.binstall`) |

```bash
cargo install cargo-binstall   # once
cargo binstall fyi-cli
# cargo binstall fyi-mcp
```

Windows MCP override may resolve the **MCPB** asset for `fyi-mcp` — that is intentional for MCP packaging, not a full CLI zip.

---

## MCP-related packaging (not OS package managers)

These are not apt/brew equivalents; listed so operators do not confuse them with OS packages.

| Surface | Status | Docs |
|---------|--------|------|
| Official MCP Registry | **live** | [`server.json`](../server.json), [INSTALL.md § MCP](../INSTALL.md#6-mcp-registries-catalogs-and-fyi-mcp-binary) |
| Glama / Smithery | **live** | matrix + INSTALL |
| PulseMCP, mcp.so, Docker MCP Catalog, … | **assets-ready** | [`packaging/mcp-catalogs/SUBMISSION_CHECKLIST.md`](../packaging/mcp-catalogs/SUBMISSION_CHECKLIST.md) |
| GHCR `fyi-mcp` image | **assets-ready** | [containers.md](./containers.md) |

---

## Keeping drafts honest

1. Prefer **one release train** (GitHub Releases + cargo-dist) as the source of truth for URLs and hashes.  
2. After each release, update Scoop/WinGet/Homebrew/Chocolatey hashes and any `PLACEHOLDER` fields.  
3. Never mark a matrix row **live** without a public install proof URL/API result in [registry-distribution-matrix.md](./registry-distribution-matrix.md).  
4. End-user docs ([INSTALL.md](../INSTALL.md), README) should keep draft commands clearly labeled.

---

## Related

- [INSTALL.md](../INSTALL.md)  
- [docs/registry-distribution-matrix.md](./registry-distribution-matrix.md)  
- [docs/containers.md](./containers.md)  
- [packaging/mcp-catalogs/SUBMISSION_CHECKLIST.md](../packaging/mcp-catalogs/SUBMISSION_CHECKLIST.md)  
- [README.md](../README.md) — “Where fyi-cli / fyi-mcp is listed”
