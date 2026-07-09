# Docker MCP Catalog submission package (`fyi-mcp`)

**Issue:** [#102](https://github.com/edithatogo/fyi-cli/issues/102)  
**Status:** **assets-ready** (image + Dockerfile ready; external catalog PR / multi-arch publish not claimed live)  
**Related:** [#116](https://github.com/edithatogo/fyi-cli/issues/116) container publish workflow

## Overview

The [Docker MCP Catalog](https://hub.docker.com/mcp) / [docker/mcp-registry](https://github.com/docker/mcp-registry)
is the curated catalog used by Docker Hub MCP and Docker Desktop MCP Toolkit.

This repo provides:

| Asset | Path / name |
|-------|-------------|
| Dockerfile (stdio `fyi-mcp`) | [`Dockerfile`](../../../Dockerfile) (repo root) |
| GHCR image name (intended) | **`ghcr.io/edithatogo/fyi-mcp`** |
| Multi-arch publish workflow | [`.github/workflows/container-publish.yml`](../../../.github/workflows/container-publish.yml) |
| Operator docs | [`docs/containers.md`](../../../docs/containers.md) |
| Glama/container env notes | [`GLAMA.md`](../../../GLAMA.md), [`glama.json`](../../../glama.json) |

## Image identity

```text
ghcr.io/edithatogo/fyi-mcp
```

Tags produced by the workflow (when a release tag is pushed or workflow is dispatched):

- `latest` (on git tag pushes)
- full git tag ref (e.g. `fyi-mcp-v0.1.2` / `v0.1.2`)
- `sha-<shortsha>`
- optional override via `workflow_dispatch` input `tag`

Platforms: `linux/amd64`, `linux/arm64`.

## Local build (verify Dockerfile)

```bash
docker build -t fyi-mcp:local -f Dockerfile .
docker run --rm -i fyi-mcp:local
# stdio MCP server; defaults FYI_MCP_EPHEMERAL=1 and DATABASE_URL=sqlite::memory:
```

## Run via Docker (MCP client)

```json
{
  "mcpServers": {
    "fyi-mcp": {
      "command": "docker",
      "args": [
        "run",
        "-i",
        "--rm",
        "ghcr.io/edithatogo/fyi-mcp:latest"
      ]
    }
  }
}
```

Persistent volume example (when not using ephemeral mode):

```bash
docker run --rm -i \
  -e FYI_MCP_EPHEMERAL=0 \
  -e DATABASE_URL=sqlite:/data/fyi_system.db \
  -v fyi-mcp-data:/data \
  ghcr.io/edithatogo/fyi-mcp:latest
```

## Official Docker MCP Registry submission notes

Upstream contribution guide: https://github.com/docker/mcp-registry (see `CONTRIBUTING.md`).

Typical options:

1. **Docker-built image (recommended by Docker)** — PR metadata into `docker/mcp-registry`;
   Docker builds, signs, and publishes under the `mcp/` namespace on Docker Hub.
2. **Self-published image** — point catalog metadata at
   `ghcr.io/edithatogo/fyi-mcp` after multi-arch images are actually pushed and pullable.

### Draft metadata for a future PR

Use when opening a PR against `docker/mcp-registry` (field names may follow their current schema;
adapt to CONTRIBUTING.md at submit time):

```yaml
name: fyi-mcp
title: FYI MCP
description: >
  Multi-jurisdiction FOI/OIA request tracker for Alaveteli platforms.
  Local SQLite storage; MCP tools for requests, authorities, correspondence,
  offline sync, and health checks.
repository: https://github.com/edithatogo/fyi-cli
dockerfile: Dockerfile
# Prefer Docker-built path per upstream docs; self-hosted image when GHCR is live:
image: ghcr.io/edithatogo/fyi-mcp
license: MIT
transport: stdio
# Official MCP Registry
official_name: io.github.edithatogo/fyi-mcp
```

## Pre-submit checklist

- [ ] `container-publish.yml` has successfully pushed multi-arch tags to GHCR
- [ ] `docker pull ghcr.io/edithatogo/fyi-mcp:latest` works for amd64 and arm64
- [ ] Entrypoint is `/usr/local/bin/fyi-mcp` (stdio)
- [ ] Default env is safe for sandbox (`FYI_MCP_EPHEMERAL=1` / in-memory SQLite)
- [ ] Open PR or submission per current `docker/mcp-registry` CONTRIBUTING
- [ ] Record public catalog URL; only then mark **live** in the distribution matrix

## Do not claim live

- Dockerfile + workflow = **assets-ready**
- GHCR images are **not** marked live until a public pull succeeds
- Docker MCP Catalog row stays **assets-ready** until the external listing exists
