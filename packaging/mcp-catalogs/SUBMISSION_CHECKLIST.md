# MCP catalog submission checklist

Operator checklist for third-party MCP catalogs covering **fyi-mcp**.  
Statuses must stay honest: **assets-ready** / **planned** until an external public URL exists.

Canonical matrix: [`docs/registry-distribution-matrix.md`](../../docs/registry-distribution-matrix.md)  
Home listing table: [`README.md`](../../README.md) (“Where fyi-cli / fyi-mcp is listed”)

**Last prepared:** 2026-07-09  
**Server version (current assets):** `0.1.2`  
**Official registry name:** `io.github.edithatogo/fyi-mcp`

---

## Shared preflight (all catalogs)

- [ ] `server.json` version, package URL, and SHA-256 match the latest GitHub Release
- [ ] Official MCP Registry API still shows the server as active/latest
- [ ] `crates/fyi-mcp` builds: `cargo build --release --locked --package fyi-mcp`
- [ ] Tool list / description quality still acceptable (see `GLAMA.md` TDQS notes)
- [ ] License MIT; README and registry descriptions aligned
- [ ] After **any** external submit: paste public URL into the distribution matrix and only then mark **live**

Verification helpers:

```bash
curl -sS "https://registry.modelcontextprotocol.io/v0/servers?search=fyi-mcp" | jq .
curl -sS "https://glama.ai/api/mcp/v1/servers/edithatogo/fyi-cli" | jq .
curl -sS "https://registry.smithery.ai/servers?namespace=edithatogo" | jq .
```

---

## Catalog rows

### PulseMCP — issue #100

| | |
|--|--|
| Package dir | [`pulsemcp/`](./pulsemcp/) |
| Assets | `README.md`, `submission.json` |
| Status target after packaging | **assets-ready** |
| Submit | https://www.pulsemcp.com/submit (or wait for official-registry ingest) |
| Adjustments | hello@pulsemcp.com |

- [ ] Confirm `submission.json` metadata
- [ ] Submit URL / wait for weekly ingest
- [ ] Record public PulseMCP page URL
- [ ] Matrix → **live** only with proof

### mcp.so — issue #101

| | |
|--|--|
| Package dir | [`mcp-so/`](./mcp-so/) |
| Assets | `README.md`, `listing.md` |
| Status target after packaging | **assets-ready** |
| Submit | https://mcp.so/ (site **Submit** / GitHub issue) |

- [ ] Paste issue body from `listing.md`
- [ ] Record public mcp.so URL
- [ ] Matrix → **live** only with proof

### Docker MCP Catalog — issue #102

| | |
|--|--|
| Package dir | [`docker-mcp/`](./docker-mcp/) |
| Assets | README + root `Dockerfile` + GHCR name `ghcr.io/edithatogo/fyi-mcp` |
| Status target after packaging | **assets-ready** |
| Upstream | https://github.com/docker/mcp-registry |
| Depends on | #116 multi-arch publish actually pullable (preferred for self-hosted image path) |

- [ ] Confirm multi-arch image pulls (if using GHCR path)
- [ ] Open PR / submission per upstream CONTRIBUTING
- [ ] Record Docker Hub / catalog URL
- [ ] Matrix → **live** only with proof

### mcp-get — issue #103

| | |
|--|--|
| Package dir | [`mcp-get/`](./mcp-get/) |
| Assets | README + install notes + draft installer metadata |
| Status target after packaging | **assets-ready** |
| Note | Upstream mcp-get is **archived/deprecated**; prefer Official Registry. Keep assets for successors / manual install docs. |

- [ ] Decide: successor registry still exists?
- [ ] If yes: submit draft metadata; record URL → **live**
- [ ] If no: leave **assets-ready** / document deprecation in matrix notes

### OpenTools — issue #104

| | |
|--|--|
| Package dir | [`opentools/`](./opentools/) |
| Assets | README + listing blurb |
| Status target after packaging | **assets-ready** |
| Submit | OpenTools site registry flow (opentools.com / opentools.ai) |

- [ ] Paste blurb + links
- [ ] Record public listing URL
- [ ] Matrix → **live** only with proof

### Awesome-MCP-Servers — issue #105

| | |
|--|--|
| Status | **assets-ready** (PR submitted; merge pending) |
| PR | https://github.com/punkpeye/awesome-mcp-servers/pull/9693 |
| Local reference entry | `awesome-mcp-servers/README.md` (Legal section) |

- [ ] Monitor PR #9693 review/merge
- [ ] On merge: matrix → **live** with permalink to merged README section
- [ ] If PR closed without merge: update matrix notes; re-open or revise entry

---

## Containers (#116) — dependency for Docker catalog

| | |
|--|--|
| Workflow | [`.github/workflows/container-publish.yml`](../../.github/workflows/container-publish.yml) |
| Docs | [`docs/containers.md`](../../docs/containers.md) |
| Image | `ghcr.io/edithatogo/fyi-mcp` |
| Dockerfile | repo root |

- [ ] Tag push `fyi-mcp-v*` or `v*` (or `workflow_dispatch`) succeeds
- [ ] `docker pull ghcr.io/edithatogo/fyi-mcp:latest` works
- [ ] Multi-arch: amd64 + arm64
- [ ] Only then treat GHCR row as **live** in the matrix

---

## Already live (do not re-submit blindly)

| Target | Evidence |
|--------|----------|
| Official MCP Registry | `io.github.edithatogo/fyi-mcp` + root `server.json` |
| Glama | https://glama.ai/mcp/servers/edithatogo/fyi-cli |
| Smithery | https://smithery.ai/server/@edithatogo/fyi-mcp |

---

## Status discipline

| Status | When to use |
|--------|-------------|
| **planned** | No repo assets yet |
| **assets-ready** | This checklist package / manifests exist; external step pending |
| **blocked-external** | Waiting on third-party review (e.g. GitHub curated MCP, Awesome PR review) |
| **live** | Public URL or API proof recorded in `docs/registry-distribution-matrix.md` |

**Never** mark a catalog **live** without proof in the matrix.
