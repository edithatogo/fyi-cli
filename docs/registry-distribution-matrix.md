# Registry distribution matrix

Implementation-oriented submission matrix for **fyi-cli** / **fyi-mcp** distribution.
Update this file when a submission lands or when release assets change.

Last reviewed: **2026-07-09**

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
| Glama | **live** | Listed as [fyi-mcp by edithatogo](https://glama.ai/mcp/servers/edithatogo/fyi-cli); search `author:edithatogo` returns the server; scores license A / quality A / maintenance B | #25 |
| Smithery | **live** (score pending) | Registry: `edithatogo/fyi-mcp` via `namespace=edithatogo`; page `https://smithery.ai/server/@edithatogo/fyi-mcp`; **score still `null`** | #26 |
| GitHub curated MCP surface (`github.com/mcp`) | **blocked-external** | Official registry entry exists; curated GitHub surface still requires manual onboarding | #32 |
| PulseMCP | **planned** | Submit package metadata | #100 |
| mcp.so | **planned** | Register + submit | #101 |
| Docker MCP Catalog | **assets-ready** | Dockerfile builds `fyi-mcp` image; submit when multi-arch publish is live | #102 |
| mcp-get | **planned** | Add install metadata | #103 |
| OpenTools | **planned** | Directory submission | #104 |
| Awesome-MCP-Servers | **assets-ready** | Community PR opened: https://github.com/punkpeye/awesome-mcp-servers/pull/9693 | #105 |

## Package managers

| Target | Status | Repo artifacts | Issue |
|--------|--------|----------------|-------|
| Crates.io | **assets-ready** / publish flow | cargo-dist / release automation | — |
| PyPI (legacy `fyi-cli`) | **live** / maintained as legacy | `pyproject.toml` | — |
| Homebrew | **assets-ready** | `packaging/homebrew/fyi-cli.rb` (placeholders for SHA/URLs — update per release) | — |
| Chocolatey | **assets-ready** | `packaging/chocolatey/fyi-cli.nuspec` | — |
| Scoop | **assets-ready** | `packaging/scoop/fyi-cli.json` | #106 |
| WinGet | **assets-ready** (draft) | `packaging/winget/edithatogo.fyi-cli.yaml` — prefer cargo-dist `.zip`/`.exe` before community PR | #107 |
| AUR | **planned** | PKGBUILD not yet authored | #108 |
| nixpkgs | **planned** | expression not yet authored | #109 |
| Snap | **planned** | snapcraft.yaml not yet authored | #110 |
| Flatpak | **planned** | manifest not yet authored | #111 |
| asdf / mise | **planned** | plugin repo not yet authored | #112 |
| cargo-binstall | **assets-ready** | `packaging/cargo-binstall/metadata.toml` (+ crate metadata to wire) | #113 |
| Debian / PPA | **planned** | packaging not yet authored | #114 |
| Fedora / COPR | **planned** | packaging not yet authored | #115 |

## Containers

| Target | Status | Notes | Issue |
|--------|--------|-------|-------|
| Dockerfile (local / CI build) | **assets-ready** | Root `Dockerfile` builds release `fyi-mcp` | #116 |
| GHCR / Docker Hub / Quay multi-arch | **planned** | Wire publish job after release tagging | #116 |

## Implementation notes

1. Prefer **one release train** (GitHub Releases + cargo-dist) as the source of truth for hashes.
2. MCPB assets (`fyi-mcp-*-win32.mcpb`) are valid for MCP installs; WinGet/Scoop UX is better with plain CLI zip/exe — track as a release packaging improvement if needed.
3. Do not mark external catalog rows **live** without a public URL or API proof in this file.
4. Conductor track: `.conductor/tracks/registry-distribution-expansion/` (epic #45).

## Verification commands (operator)

```bash
# Official MCP registry
curl -sS "https://registry.modelcontextprotocol.io/v0/servers?search=fyi-mcp" | jq .

# Glama directory API
curl -sS "https://glama.ai/api/mcp/v1/servers/edithatogo/fyi-cli" | jq .

# Smithery registry (namespace listing)
curl -sS "https://registry.smithery.ai/servers?namespace=edithatogo" | jq .
```
