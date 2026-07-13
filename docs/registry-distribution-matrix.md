# Registry distribution matrix

Implementation-oriented submission matrix for **fyi-cli** / **fyi-mcp** distribution.
Update this file when a submission lands or when release assets change.

Last reviewed: **2026-07-11**

Home-page listing (README): see **Where fyi-cli / fyi-mcp is listed** in [`README.md`](../README.md).  
Release automation (tags, GHCR, external catalogs, version sync): [`release-multi-registry.md`](./release-multi-registry.md).
Release compatibility and integrity contract: [`packaging/release-compatibility.json`](../packaging/release-compatibility.json).
Packaging asset CI check: `python scripts/verify_packaging_assets.py`.
Machine-readable submission ledger: [`packaging/registry-submissions.json`](../packaging/registry-submissions.json), validated by `python scripts/validate_registry_submission_ledger.py`.

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
| OpenAI Codex plugins | **assets-ready** | [`packaging/ai-plugins/codex/`](../packaging/ai-plugins/codex/) is validated; external submission and public evidence remain pending | #197 |
| Anthropic Claude Connectors Directory | **assets-ready** | [`packaging/ai-plugins/anthropic/`](../packaging/ai-plugins/anthropic/) is validated; external submission and public evidence remain pending | #198 |
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

| Target | Status | Repo artifacts | Issue |
|--------|--------|----------------|-------|
| Crates.io | **assets-ready** / publish flow | cargo-dist / release automation | — |
| PyPI (legacy `fyi-cli`) | **live** / maintained as legacy | `pyproject.toml` | — |
| Homebrew | **assets-ready** | `packaging/homebrew/fyi-cli.rb` (placeholders for SHA/URLs — update per release) | — |
| Chocolatey | **assets-ready** | `packaging/chocolatey/fyi-cli.nuspec` | — |
| Scoop | **assets-ready** | `packaging/scoop/fyi-cli.json` | #106 |
| WinGet | **assets-ready** (draft) | `packaging/winget/edithatogo.fyi-cli.yaml` — prefer cargo-dist `.zip`/`.exe` before community PR | #107 |
| AUR | **assets-ready** (draft) | `packaging/aur/PKGBUILD` — set `sha256sums` from release source tarball before AUR submit | #108 |
| nixpkgs | **assets-ready** (draft) | `packaging/nix/default.nix` — fill `src.hash` / `cargoHash` after first `nix-build` | #109 |
| Snap | **assets-ready** (draft) | `packaging/snap/snapcraft.yaml` — builds `fyi-cli` + `fyi-mcp` from tagged source | #110 |
| Flatpak | **assets-ready** (draft) | `packaging/flatpak/io.github.edithatogo.fyi-cli.yml` (+ AppStream metainfo) | #111 |
| asdf / mise | **assets-ready** (draft) | `packaging/asdf/bin/{install,list-all}`, `packaging/mise/backend.toml` | #112 |
| cargo-binstall | **assets-ready** | `packaging/cargo-binstall/metadata.toml` (+ crate metadata to wire) | #113 |
| Debian / PPA | **assets-ready** (draft) | `packaging/debian/{control,rules,changelog,copyright}` skeleton | #114 |
| Fedora / COPR | **assets-ready** (draft) | `packaging/fedora/fyi-cli.spec` | #115 |

## Containers

| Target | Status | Notes | Issue |
|--------|--------|-------|-------|
| Dockerfile (local / CI build) | **assets-ready** | Root `Dockerfile` builds release `fyi-mcp`; operator docs in [`docs/containers.md`](./containers.md) | #116 |
| GHCR multi-arch publish | **assets-ready** | Workflow [`.github/workflows/container-publish.yml`](../.github/workflows/container-publish.yml) builds `linux/amd64`+`linux/arm64` → `ghcr.io/edithatogo/fyi-mcp` on `fyi-mcp-v*` / `v*` tags (or `workflow_dispatch`). **Not live** until a public pull is verified | #116 |
| Docker Hub / Quay | **planned** | Optional mirrors after GHCR is proven | #116 |

MCP catalog submission checklist (all catalogs + Awesome PR): [`packaging/mcp-catalogs/SUBMISSION_CHECKLIST.md`](../packaging/mcp-catalogs/SUBMISSION_CHECKLIST.md).

## Implementation notes

1. Prefer **one release train** (GitHub Releases + cargo-dist) as the source of truth for hashes.
2. MCPB assets (`fyi-mcp-*-win32.mcpb`) are valid for MCP installs; WinGet/Scoop UX is better with plain CLI zip/exe — track as a release packaging improvement if needed.
3. Do not mark external catalog rows **live** without a public URL or API proof in this file.
4. Catalog submission packages live under `packaging/mcp-catalogs/{pulsemcp,mcp-so,docker-mcp,mcp-get,opentools}/`.
5. Conductor track: `.conductor/tracks/registry-distribution-expansion/` (epic #45).

## Verification commands (operator)

```bash
# Official MCP registry
curl -sS "https://registry.modelcontextprotocol.io/v0/servers?search=fyi-mcp" | jq .

# Glama directory API
curl -sS "https://glama.ai/api/mcp/v1/servers/edithatogo/fyi-cli" | jq .

# Smithery registry (namespace listing)
curl -sS "https://registry.smithery.ai/servers?namespace=edithatogo" | jq .
```
