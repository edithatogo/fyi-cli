# Registry distribution matrix

Implementation-oriented submission matrix for **fyi-cli** / **fyi-mcp** distribution.
Update this file when a submission lands or when release assets change.

Package/container evidence reviewed: **2026-11-14**; MCP/AI catalog rows retain their dated evidence below.

Home-page listing (README): see **Where fyi-cli / fyi-mcp is listed** in [`README.md`](../README.md).  
Release automation (tags, GHCR, external catalogs, version sync): [`release-multi-registry.md`](./release-multi-registry.md).
Release compatibility and integrity contract: [`packaging/release-compatibility.json`](../packaging/release-compatibility.json).
Packaging asset CI check: `python scripts/verify_packaging_assets.py`.
Machine-readable submission ledger: [`packaging/registry-submissions.json`](../packaging/registry-submissions.json), validated by `python scripts/validate_registry_submission_ledger.py`.
Package operator steps: [`packaging/SUBMISSION_RUNBOOK.md`](../packaging/SUBMISSION_RUNBOOK.md).
Observed release/assets: [`packaging/release-evidence.json`](../packaging/release-evidence.json).

## Legend

| Status | Meaning |
|--------|---------|
| **live** | Public listing verified |
| **assets-ready** | Repo has metadata/manifests; external submission pending or needs release asset shape |
| **blocked-external** | Waiting on third-party review/indexing/curation |
| **planned** | Tracked; not started |

## MCP registries & catalogs

| Target | Status | Evidence / next step | Issue |
|--------|--------|----------------------|-------|
| Official MCP Registry (`io.github.edithatogo/fyi-mcp`) | **live** | API shows `0.1.2` active/latest; `server.json` in repo | — |
| OpenAI plugin directory (Codex) | **assets-ready** | [`packaging/ai-plugins/codex/`](../packaging/ai-plugins/codex/) is validated; packet refreshed to the current plugin-directory model, but external submission and public evidence remain pending | #197 |
| Anthropic Claude Connectors Directory | **assets-ready** | [`packaging/ai-plugins/anthropic/`](../packaging/ai-plugins/anthropic/) is validated; hosted HTTPS deployment evidence plus external submission and public evidence remain pending | #198 |
| Glama | **live** | Listed as [fyi-mcp by edithatogo](https://glama.ai/mcp/servers/edithatogo/fyi-cli); search `author:edithatogo` returns the server; scores license A / quality A / maintenance B | #25 |
| Smithery | **live** (score pending external) | Registry: `edithatogo/fyi-mcp`; page [smithery.ai/server/@edithatogo/fyi-mcp](https://smithery.ai/server/@edithatogo/fyi-mcp). Fresh 2026-07-11 API check: `score=null`, `useCount=0`, `remote=false`; detail exposes 14 tools / 2 resources / 3 prompts. Playbook: [docs/external-registry-followups.md](external-registry-followups.md) | #26 |
| GitHub curated MCP surface (`github.com/mcp`) | **blocked-external** | Fresh 2026-07-11 check: OSS registry `io.github.edithatogo/fyi-mcp@0.1.2` active/latest; curated search has no server card and direct path is 404. Onboarding requests filed: [discussion #2844](https://github.com/github/github-mcp-server/discussions/2844), [comment on #1257](https://github.com/github/github-mcp-server/discussions/1257#discussioncomment-17584387). Playbook: [docs/external-registry-followups.md](external-registry-followups.md) | #32 |
| PulseMCP | **assets-ready** | Package: [`packaging/mcp-catalogs/pulsemcp/`](../packaging/mcp-catalogs/pulsemcp/) (`submission.json` + README). Submit via https://www.pulsemcp.com/submit or wait for official-registry weekly ingest; do not mark live without a public PulseMCP URL | #100 |
| mcp.so | **assets-ready** | Package: [`packaging/mcp-catalogs/mcp-so/`](../packaging/mcp-catalogs/mcp-so/) (`listing.md` + issue template). Submit via mcp.so **Submit** / GitHub issue | #101 |
| Docker MCP Catalog | **assets-ready** | Package: [`packaging/mcp-catalogs/docker-mcp/`](../packaging/mcp-catalogs/docker-mcp/); root `Dockerfile`; intended image `ghcr.io/edithatogo/fyi-mcp`. Open PR per [docker/mcp-registry](https://github.com/docker/mcp-registry) when multi-arch pull is verified | #102 |
| mcp-get | **assets-ready** | Package: [`packaging/mcp-catalogs/mcp-get/`](../packaging/mcp-catalogs/mcp-get/) install notes + draft metadata. Upstream mcp-get is archived/deprecated — prefer Official Registry; keep assets for successors | #103 |
| OpenTools | **assets-ready** | Package: [`packaging/mcp-catalogs/opentools/`](../packaging/mcp-catalogs/opentools/) listing blurb ready for directory submit | #104 |
| Awesome-MCP-Servers | **assets-ready** | Community PR opened (not merged): https://github.com/punkpeye/awesome-mcp-servers/pull/9693 — see also [`packaging/mcp-catalogs/SUBMISSION_CHECKLIST.md`](../packaging/mcp-catalogs/SUBMISSION_CHECKLIST.md) | #105 |

## Package managers

**No current Rust package-manager packet is submission-ready, submitted or verified
live.** The binary review packets bind actual historical `v1.2.0` CLI assets to
their source's Cargo version `0.1.2`; API-observed hashes are not downloaded-byte
or native-install verification. Release/version reconciliation, integrity and
platform evidence remain required. `v0.1.2` tag/release lookups returned 404.

For packages, **blocked** means recorded prerequisites remain; **assets-ready**
requires a qualified immutable release and completed native evidence;
**submitted** requires an actual submission URL; **live** additionally requires
public listing and verified clean installation. No status follows merely from
having a manifest. The legacy PyPI listing is separate from Rust distribution.

| Target | Status | Repo artifacts | Issue |
|--------|--------|----------------|-------|
| Crates.io | **blocked** | `fyi-cli` and `fyi-core` are occupied and have no matching 0.1.2 index release; verify project ownership or select new names before publishing | — |
| PyPI (legacy `fyi-cli`) | **live** / frozen for migration | Public release 1.0.0; the Git-pinned archive consumer is separate | — |
| Homebrew maintainer tap | **blocked** | `packaging/homebrew/fyi-cli.rb` has actual archive URLs/hashes; native audit/install/test and an owned tap remain. This binary formula is not eligible for Homebrew/core | — |
| Chocolatey | **blocked** | `packaging/chocolatey/` binds the x64 CLI ZIP; byte/native package verification and moderation remain | — |
| Scoop | **blocked** | `packaging/scoop/fyi-cli.json` binds the x64 CLI ZIP and `fyi-cli.exe`; clean validation/install and bucket review remain | #106 |
| WinGet | **blocked** | `packaging/winget/edithatogo.fyi-cli.yaml` uses a ZIP with nested portable `fyi-cli.exe`; native validation/install and community review remain | #107 |
| AUR | **blocked** | `packaging/aur/PKGBUILD` pins source; source/vendor hashes, clean chroot build and `.SRCINFO` remain | #108 |
| nixpkgs | **blocked** | `packaging/nix/default.nix` requires source NAR/vendor hashes, a pinned builder, native Linux tests and a maintainer | #109 |
| Snap | **blocked** | `packaging/snap/snapcraft.yaml` requires hashed vendoring, a separately qualified offline compiler per architecture, confined native builds and publisher/name verification | #110 |
| Flatpak / Flathub | **not-applicable** to Flathub; private Flatpak **blocked** | Current Flathub policy excludes this console product. `packaging/flatpak/` is a private template requiring hashes, runtime qualification and sandbox smoke | #111 |
| asdf / mise | **blocked** | `packaging/asdf/` has no approved install version or published plugin repository; `packaging/mise/backend.toml` is an inventory, not a registered backend | #112 |
| cargo-binstall | **blocked** | Default strategies are disabled. Resolve crate identity and qualify exact binary mappings before enabling a distribution path | #113 |
| Debian / PPA | **blocked** | `packaging/debian/` requires dependency/licence closure, native offline build, Lintian/autopkgtest and signing/sponsorship | #114 |
| Fedora / COPR | **blocked** | `packaging/fedora/fyi-cli.spec` requires source/vendor hashes, SRPM/mock builds and an owned COPR project; Fedora archive review is separate | #115 |

## Containers

| Target | Status | Notes | Issue |
|--------|--------|-------|-------|
| Dockerfile (local / CI build) | **blocked** for distribution | Root Dockerfile is available for local builds; reproducible source/image binding, integrity and clean installation still require evidence | #116 |
| GHCR multi-arch publish | **blocked** | Existing workflow targets `linux/amd64`+`linux/arm64` → `ghcr.io/edithatogo/fyi-mcp`; public pulls, immutable digests, SBOM, provenance and isolated native smoke remain unverified | #116 |
| Docker Hub / Quay | **not-applicable / deferred** | First qualify GHCR; require a documented consumer need, owner and digest-preserving replication before adding a mirror | #116 |

MCP catalog submission checklist (all catalogs + Awesome PR): [`packaging/mcp-catalogs/SUBMISSION_CHECKLIST.md`](../packaging/mcp-catalogs/SUBMISSION_CHECKLIST.md).

## Implementation notes

1. Qualify **one signed, versioned release** as the source for package bytes, hashes, SBOM and provenance. Current historical GitHub release metadata is observed, not qualified.
2. Scoop/WinGet use the observed Windows CLI ZIP. Separate historical MCPB packages must not substitute for a CLI executable or a standalone MCP archive.
3. Do not mark external catalog rows **live** without a public URL or API proof in this file.
4. Catalog submission packages live under `packaging/mcp-catalogs/{pulsemcp,mcp-so,docker-mcp,mcp-get,opentools}/`.
5. Package Conductor track: [`.conductor/tracks/package-registry-submissions_20260712/`](../.conductor/tracks/package-registry-submissions_20260712/) (epic [#194](https://github.com/edithatogo/fyi-cli/issues/194)). The prior expansion track is historical context.

## Verification commands (operator)

```bash
# Official MCP registry
curl -sS "https://registry.modelcontextprotocol.io/v0/servers?search=fyi-mcp" | jq .

# Glama directory API
curl -sS "https://glama.ai/api/mcp/v1/servers/edithatogo/fyi-cli" | jq .

# Smithery registry (namespace listing)
curl -sS "https://registry.smithery.ai/servers?namespace=edithatogo" | jq .
```
