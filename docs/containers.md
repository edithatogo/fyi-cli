# Containers (`fyi-mcp`)

How to build, publish, and pull the **fyi-mcp** container image.

| Item | Value |
|------|-------|
| Dockerfile | [`Dockerfile`](../Dockerfile) (repository root) |
| Publish workflow | [`.github/workflows/container-publish.yml`](../.github/workflows/container-publish.yml) |
| Intended image | **`ghcr.io/edithatogo/fyi-mcp`** |
| Configured build targets (not verified public pulls) | `linux/amd64`, `linux/arm64` |
| Entrypoint | `/usr/local/bin/fyi-mcp` (defaults to stdio; set `FYI_MCP_TRANSPORT=http` for stateless HTTP JSON-RPC) |
| Issue | [#116](https://github.com/edithatogo/fyi-cli/issues/116) |

**Distribution status (14 November 2026): blocked.** The Dockerfile and publication
workflow exist; no current GHCR channel is submission-ready or verified live.
Credential-free public pulls on both configured architectures, immutable
digest/source binding, SBOM, authenticated provenance and isolated native
help/install evidence remain unverified. An image build or a successful metadata
inspection alone does not establish those controls. The
[submission runbook](../packaging/SUBMISSION_RUNBOOK.md) and
[distribution ledger](../packaging/registry-submissions.json) record the exact
operator steps and blockers. No publication workflow was dispatched in this audit.

Use the manual [Container Verification workflow](../.github/workflows/container-verification.yml)
to inspect a published tag. It requires both `linux/amd64` and `linux/arm64`; it does not change the
distribution ledger automatically.

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

The following are conditional operator examples after a release has been
published and qualified. No image tag or digest below is asserted available.
Use the immutable digest recorded in the release evidence for installations.

### Latest

```bash
docker pull ghcr.io/edithatogo/fyi-mcp:latest
```

### Explicit release tag

```bash
# Set this only to a tag whose publication and release binding were verified.
docker pull "ghcr.io/edithatogo/fyi-mcp:${QUALIFIED_GHCR_TAG}"
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

  ### Hosted HTTP JSON-RPC (connector-prep only)

  ```bash
  docker run --rm \
    -p 8080:8080 \
    -e FYI_MCP_TRANSPORT=http \
    -e FYI_MCP_HTTP_BEARER_TOKEN=replace-me \
    ghcr.io/edithatogo/fyi-mcp:latest

  curl -H "Authorization: Bearer replace-me" \
    http://127.0.0.1:8080/healthz
  ```

  This enables the opt-in stateless HTTP JSON-RPC listener documented in
  [`deploy/remote-mcp/README.md`](../deploy/remote-mcp/README.md). It is still
  not evidence of a hosted Connector deployment: terminate TLS at a trusted
  reverse proxy or platform ingress, keep write capability disabled, and do not
  claim external connector availability until the hosted deployment contract is
  verified.

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
- [`deploy/remote-mcp/README.md`](../deploy/remote-mcp/README.md)
- [`GLAMA.md`](../GLAMA.md)
- [`crates/fyi-mcp/README.md`](../crates/fyi-mcp/README.md)
