# Multi-registry release automation

Operator guide for shipping **fyi-cli** / **fyi-mcp** across GitHub Releases, GHCR, crates.io, and external package/MCP catalogs.

Canonical status matrix: [`registry-distribution-matrix.md`](./registry-distribution-matrix.md)  
MCP catalog checklist: [`packaging/mcp-catalogs/SUBMISSION_CHECKLIST.md`](../packaging/mcp-catalogs/SUBMISSION_CHECKLIST.md)  
Container details: [`containers.md`](./containers.md)

Last reviewed: **2026-07-09**

---

## Principles

1. **One release train** — GitHub tag + GitHub Release assets are the source of truth for version and binary hashes.
2. **Version synchronization** — crates, `server.json`, packaging drafts, and catalog assets should all mention the same semver (currently **0.1.2**).
3. **Graceful failure** — a single registry failure must not block the rest of the train (independent jobs / `continue-on-error` / manual external steps).
4. **Honest matrix status** — do not mark a row **live** without a public URL or API proof in the distribution matrix.

---

## Automated: tag-driven GitHub Release

Workflow: [`.github/workflows/release.yml`](../.github/workflows/release.yml)

| Trigger | Tags matching `v*` (e.g. `v0.1.2`) |
|---------|-------------------------------------|
| Builds | Matrix: Linux amd64, macOS amd64/arm64, Windows amd64 (`fail-fast: false`) |
| Assets | `fyi-cli-linux-amd64.tar.gz`, `fyi-cli-macos-*.tar.gz`, `fyi-cli-windows-amd64.exe.zip` (+ `.sha256`) |
| Crates.io | Separate job publishes `fyi-core` → `fyi-mcp` → `fyi-cli` with **`continue-on-error: true`** so a crates.io outage does not block the GitHub Release |
| GitHub Release | Aggregates build artifacts via `softprops/action-gh-release` |

### Operator steps

```bash
# Ensure workspace crate versions match the intended tag
# crates/fyi-{core,cli,mcp}/Cargo.toml → version = "X.Y.Z"

git tag -a v0.1.2 -m "v0.1.2"
git push origin v0.1.2
# Watch Actions → Release; confirm assets on the GitHub Releases page
```

Related: release-please may open version PRs ([`release-please.yml`](../.github/workflows/release-please.yml)); tag push still drives the binary release job above.

---

## Automated: GHCR multi-arch image

Workflow: [`.github/workflows/container-publish.yml`](../.github/workflows/container-publish.yml)

| Trigger | Tags `v*` or `fyi-mcp-v*`, or `workflow_dispatch` |
|---------|---------------------------------------------------|
| Image | `ghcr.io/<owner>/fyi-mcp` (public intent: `ghcr.io/edithatogo/fyi-mcp`) |
| Platforms | `linux/amd64`, `linux/arm64` |
| Auth | `GITHUB_TOKEN` → packages:write |

```bash
docker pull ghcr.io/edithatogo/fyi-mcp:v0.1.2
# or :latest after a tag publish
```

Do **not** mark GHCR **live** in the matrix until a public pull succeeds.

### Optional Docker Hub mirror

Not enabled by default. See comments in `container-publish.yml`: if repository secrets `DOCKERHUB_USERNAME` / `DOCKERHUB_TOKEN` (and optional variable `ENABLE_DOCKERHUB_MIRROR=true`) are configured, an optional job can push the same multi-arch image to Docker Hub. Prefer proving GHCR first; Hub/Quay remain **planned** in the matrix until configured.

---

## Manual / external: package managers & MCP catalogs

These paths are **assets-ready** drafts under `packaging/`. CI does not submit them for you.

### Preflight (every release)

```bash
# Packaging asset presence + version alignment (no cargo required)
python scripts/verify_packaging_assets.py

# Optional machine-readable report
python scripts/verify_packaging_assets.py --json
```

Also:

1. Bump version in crates + packaging files that pin `X.Y.Z`.
2. Refresh SHA-256 / installer URLs from the new GitHub Release (Homebrew, Scoop, WinGet, AUR `sha256sums`, `server.json` mcpb hash, etc.).
3. Follow [`packaging/mcp-catalogs/SUBMISSION_CHECKLIST.md`](../packaging/mcp-catalogs/SUBMISSION_CHECKLIST.md).
4. Update [`registry-distribution-matrix.md`](./registry-distribution-matrix.md) only after public proof.

### Catalog & PM checklist (link map)

| Target | Repo assets | Operator action |
|--------|-------------|-----------------|
| PulseMCP | `packaging/mcp-catalogs/pulsemcp/` | Submit form / wait for registry ingest |
| mcp.so | `packaging/mcp-catalogs/mcp-so/` | Site Submit / GitHub issue |
| Docker MCP Catalog | `packaging/mcp-catalogs/docker-mcp/` + root `Dockerfile` | PR to [docker/mcp-registry](https://github.com/docker/mcp-registry) after GHCR pull works |
| mcp-get | `packaging/mcp-catalogs/mcp-get/` | Prefer Official Registry; keep assets for successors |
| OpenTools | `packaging/mcp-catalogs/opentools/` | Directory submit |
| Awesome-MCP-Servers | checklist + community PR | Track merge; do not claim live early |
| Homebrew / Chocolatey / Scoop / WinGet | `packaging/{homebrew,chocolatey,scoop,winget}/` | Update hashes; open tap/community PR |
| AUR / Nix / Snap / Flatpak | `packaging/{aur,nix,snap,flatpak}/` | Fill checksums; submit when ready |
| asdf / mise | `packaging/asdf/`, `packaging/mise/` | Publish plugin when asset names stable |
| cargo-binstall | `packaging/cargo-binstall/metadata.toml` + crate `[package.metadata.binstall]` | Relies on GitHub Release asset naming |
| Debian / Fedora | `packaging/debian/`, `packaging/fedora/` | PPA/COPR when maintainers ready |
| Conda | `conda/` + [`CONDA_PUBLISHING.md`](../CONDA_PUBLISHING.md) | Separate conda-publish workflow when used |

---

## Version synchronization expectations

| Surface | Where version lives |
|---------|---------------------|
| Crates (source of truth for semver) | `crates/fyi-cli/Cargo.toml`, `fyi-mcp`, `fyi-core` |
| Official MCP Registry package | root `server.json` |
| MCPB manifest | `packaging/mcpb/fyi-mcp/manifest.json` |
| Linux drafts | AUR `pkgver`, Snap `version`, Flatpak tag, Fedora `Version`, Debian changelog |
| Windows drafts | Scoop `version`, WinGet `PackageVersion`, Chocolatey nuspec |
| macOS draft | Homebrew `version` |
| Catalog blurbs | `packaging/mcp-catalogs/**` |

**Rule of thumb:** after a version bump, `python scripts/verify_packaging_assets.py` should pass with `--expected-version` matching the crate (or omit the flag to auto-discover from `crates/fyi-cli`).

Placeholder SHAs (`SKIP`, empty, or dummy hex) are expected in drafts until the release assets exist — version *strings* should still match.

---

## Graceful failure pattern

Distribution is intentionally **loosely coupled**:

| Layer | Failure isolation |
|-------|-------------------|
| Binary matrix (`release.yml` build) | `strategy.fail-fast: false` — one OS target does not cancel others |
| crates.io publish | `continue-on-error: true` per package; GitHub Release still proceeds |
| GHCR publish | Separate workflow; tag can still produce binaries if containers fail (and vice versa) |
| Conda / external catalogs | Manual or separate workflows; never gate the GitHub Release job |
| Matrix honesty | One blocked catalog does not delay marking another **live** |

**Operator pattern when one registry fails:**

1. Leave the failed target as **assets-ready** or **blocked-external** in the matrix.
2. File/track the issue (matrix Issue column).
3. Continue remaining submissions independently.
4. Re-run only the failed path (workflow_dispatch, re-publish, or re-submit PR).

Do **not** re-tag the whole release for a single external catalog hiccup unless the release assets themselves are wrong.

---

## CI guardrail

Workflow job **`packaging-assets`** in [`.github/workflows/ci.yml`](../.github/workflows/ci.yml) runs:

```bash
python scripts/verify_packaging_assets.py
```

on Ubuntu with system Python only (no Rust toolchain). Locally:

```bash
python scripts/verify_packaging_assets.py
```

See also [TOOLING.md](../TOOLING.md) (Packaging assets section).

---

## Related workflows

| Workflow | Role |
|----------|------|
| `release.yml` | Tag → binaries + GitHub Release + best-effort crates.io |
| `container-publish.yml` | Tag / dispatch → multi-arch GHCR `fyi-mcp` |
| `conda-publish.yml` | Optional conda path |
| `release-please.yml` | Automated version PR assist |
| `ci.yml` → `packaging-assets` | Presence + version string checks for packaging tree |
