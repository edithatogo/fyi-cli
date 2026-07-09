# OpenTools directory submission (`fyi-mcp`)

**Issue:** [#104](https://github.com/edithatogo/fyi-cli/issues/104)  
**Status:** **assets-ready** (listing blurb ready; not submitted / not verified live)

## Overview

OpenTools ([opentools.com](https://opentools.com/) / [opentools.ai](https://opentools.ai/))
hosts a developer-facing registry of MCP servers for tool discovery by capability. Submission is
via their public registry / site process (no stable third-party API required for this package).

## Canonical identity

| Field | Value |
|-------|-------|
| Name | FYI MCP |
| Package / binary | `fyi-mcp` |
| Official registry | `io.github.edithatogo/fyi-mcp` |
| Repository | https://github.com/edithatogo/fyi-cli |
| Language | Rust |
| License | MIT |
| Transport | stdio |
| Version | 0.1.2 |

## Listing blurb (ready to paste)

### Title

FYI MCP — FOI/OIA request tracker for Alaveteli

### Short blurb (≤280 chars)

> Local-first MCP server for multi-jurisdiction Freedom of Information / Official Information requests on Alaveteli platforms (FYI.org.nz, WhatDoTheyKnow, RightToKnow, …). SQLite storage; tools for requests, authorities, correspondence, sync, and health.

### Long blurb

> **FYI MCP** (`fyi-mcp`) is a Rust Model Context Protocol server for managing FOI and OIA
> requests against Alaveteli-based transparency platforms. It keeps all operational data in local
> SQLite and exposes stdio tools for the full request lifecycle, authority import, correspondence
> lookup, offline sync monitoring, conflict resolution, and database health checks.
>
> Use it with any MCP client (Claude Desktop, Cursor, VS Code, etc.) to draft, track, and
> reconcile information requests without sending case data to a third-party SaaS. Companion CLI
> `fyi-cli` lives in the same repository. Published on the Official MCP Registry as
> `io.github.edithatogo/fyi-mcp`.

### Capability tags

`legal` · `foi` · `oia` · `alaveteli` · `privacy` · `sqlite` · `offline-sync` · `requests` · `correspondence`

### Install one-liners

```bash
# Source
cargo install --path crates/fyi-mcp

# Container (when GHCR publish is live)
docker pull ghcr.io/edithatogo/fyi-mcp:latest
```

### MCP config snippet

```json
{
  "mcpServers": {
    "fyi-mcp": {
      "command": "fyi-mcp",
      "args": [],
      "env": {
        "DATABASE_URL": "sqlite:fyi_system.db"
      }
    }
  }
}
```

### Links for the form

- Repo: https://github.com/edithatogo/fyi-cli
- Crate / server: https://github.com/edithatogo/fyi-cli/tree/master/crates/fyi-mcp
- Official registry name: `io.github.edithatogo/fyi-mcp`
- `server.json`: https://github.com/edithatogo/fyi-cli/blob/master/server.json
- Glama: https://glama.ai/mcp/servers/edithatogo/fyi-cli
- Smithery: https://smithery.ai/server/@edithatogo/fyi-mcp
- Docs: https://edithatogo.github.io/fyi-cli/

## Operator steps

1. Confirm version and links above match the latest release.
2. Submit via the current OpenTools registry / site submission flow.
3. Record the public listing URL in
   [`docs/registry-distribution-matrix.md`](../../../docs/registry-distribution-matrix.md).
4. Flip status **assets-ready** → **live** only after the page is public.

## Do not claim live

Do not invent an OpenTools URL. Keep **assets-ready** until externally listed.
