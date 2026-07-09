# Containers (`fyi-mcp`)

How to build, publish, and pull the **fyi-mcp** container image.

| Item | Value |
|------|-------|
| Dockerfile | [`Dockerfile`](../Dockerfile) (repository root) |
| Publish workflow | [`.github/workflows/container-publish.yml`](../.github/workflows/container-publish.yml) |
| Intended image | **`ghcr.io/edithatogo/fyi-mcp`** |
| Platforms | `linux/amd64`, `linux/arm64` |
| Entrypoint | `/usr/local/bin/fyi-mcp` (stdio MCP) |
| Issue | [#116](https://github.com/edithatogo/fyi-cli/issues/116) |

**Status discipline:** the Dockerfile and GHCR workflow are **assets-ready**. Do not treat GHCR as
**live** in [`registry-distribution-matrix.md`](./registry-distribution-matrix.md) until a public
`docker pull` succeeds for a published tag.

---

## Image behaviour

The multi-stage Dockerfile:

1. Builds `fyi-mcp` with `cargo build --release --locked --package fyi-mcp` on `rust:1-bookworm`
2. Copies the binary into `debian:bookworm-slim` with CA certificates
3. Defaults to an ephemeral SQLite database for sandbox / catalog inspection:

```dockerfile
ENV FYI_MCP_EPHEMERAL=1
ENV DATABASE_URL=sqlite::memory:
ENTRYPOINT ["/usr/local/bin/fyi-mcp"]
```

For persistent storage, override env and mount a volume (see below).

Related: Glama release notes in [`GLAMA.md`](../GLAMA.md).

---

## Local build

```bash
git clone https://github.com/edithatogo/fyi-cli.git
cd fyi-cli

docker build -t fyi-mcp:local -f Dockerfile .

# stdio server (interactive)
docker run --rm -i fyi-mcp:local
```

Multi-arch local build (requires Buildx + QEMU):

```bash
docker buildx create --use --name fyi-mcp-builder 2>/dev/null || docker buildx use fyi-mcp-builder
docker buildx build \
  --platform linux/amd64,linux/arm64 \
  -t fyi-mcp:local \
  -f Dockerfile \
  --load \
  .
```

Note: `--load` only loads one platform into the local Docker engine; use `--push` to a registry
for true multi-arch manifests.

---

## CI publish (GHCR)

Workflow: `.github/workflows/container-publish.yml`

**Triggers**

- Push of tags matching `fyi-mcp-v*` or `v*`
- Manual `workflow_dispatch` (optional `tag` input)

**Permissions**

- `contents: read`
- `packages: write` (GHCR via `GITHUB_TOKEN`)

**Tags applied**

| Tag | When |
|-----|------|
| git tag ref | on tag push |
| `latest` | on tag push |
| `sha-<shortsha>` | always |
| custom | when `workflow_dispatch` input `tag` is set |

**Image name**

```text
ghcr.io/<github.repository_owner>/fyi-mcp
```

For the public project owner this is:

```text
ghcr.io/edithatogo/fyi-mcp
```

---

## Pull examples (multi-arch)

Once a workflow run has pushed images (not claimed live until verified):

### Latest

```bash
docker pull ghcr.io/edithatogo/fyi-mcp:latest
```

### Explicit release tag

```bash
# examples — use a tag that actually exists on GHCR after publish
docker pull ghcr.io/edithatogo/fyi-mcp:fyi-mcp-v0.1.2
docker pull ghcr.io/edithatogo/fyi-mcp:v0.1.2
```

### Digest pin (supply-chain friendly)

```bash
# replace with the real digest from GHCR / workflow logs
docker pull ghcr.io/edithatogo/fyi-mcp@sha256:<digest>
```

### Force a platform

```bash
docker pull --platform linux/amd64 ghcr.io/edithatogo/fyi-mcp:latest
docker pull --platform linux/arm64 ghcr.io/edithatogo/fyi-mcp:latest
```

### Inspect multi-arch manifest

```bash
docker buildx imagetools inspect ghcr.io/edithatogo/fyi-mcp:latest
```

Expected: a manifest list containing `linux/amd64` and `linux/arm64` entries.

### Private / auth

Public packages may still require `docker login ghcr.io` depending on package visibility:

```bash
echo "$GHCR_TOKEN" | docker login ghcr.io -u USERNAME --password-stdin
docker pull ghcr.io/edithatogo/fyi-mcp:latest
```

GitHub Packages: grant `read:packages` (and SSO authorize if the org requires it).

---

## Run examples

### Ephemeral (default)

```bash
docker run --rm -i ghcr.io/edithatogo/fyi-mcp:latest
```

### Persistent SQLite on a volume

```bash
docker run --rm -i \
  -e FYI_MCP_EPHEMERAL=0 \
  -e DATABASE_URL=sqlite:/data/fyi_system.db \
  -v fyi-mcp-data:/data \
  ghcr.io/edithatogo/fyi-mcp:latest
```

### MCP client (`mcpServers` JSON)

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

---

## Docker MCP Catalog

Submission package and checklist:

- [`packaging/mcp-catalogs/docker-mcp/README.md`](../packaging/mcp-catalogs/docker-mcp/README.md)
- [`packaging/mcp-catalogs/SUBMISSION_CHECKLIST.md`](../packaging/mcp-catalogs/SUBMISSION_CHECKLIST.md)

Upstream registry: https://github.com/docker/mcp-registry

---

## Troubleshooting

| Symptom | Check |
|---------|--------|
| `denied` / `unauthorized` pulling GHCR | Package visibility; `docker login ghcr.io`; token scopes |
| Wrong architecture | `docker pull --platform …`; confirm Buildx platforms in workflow |
| Empty / hanging stdio session | Client must speak MCP over stdio; use `-i` / keep stdin open |
| Data lost between runs | Defaults are in-memory; set `FYI_MCP_EPHEMERAL=0` and a volume-backed `DATABASE_URL` |

---

## Related docs

- [`docs/registry-distribution-matrix.md`](./registry-distribution-matrix.md)
- [`GLAMA.md`](../GLAMA.md)
- [`crates/fyi-mcp/README.md`](../crates/fyi-mcp/README.md)
