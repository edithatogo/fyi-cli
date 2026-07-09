# mcp.so listing draft — FYI MCP

**Status:** assets-ready (not submitted)  
**Issue:** [#101](https://github.com/edithatogo/fyi-cli/issues/101)

## Submit channel

mcp.so accepts community listings via GitHub issue / site **Submit** flow:

- Directory: https://mcp.so/
- Submit: use the site navigation **Submit** control (opens their GitHub issues flow)

Provide name, description, features, and connection / install information as below.

---

## Listing fields

| Field | Value |
|-------|-------|
| **Name** | FYI MCP |
| **Slug / package** | `fyi-mcp` |
| **Author** | edithatogo |
| **Repository** | https://github.com/edithatogo/fyi-cli |
| **Homepage** | https://github.com/edithatogo/fyi-cli |
| **Language** | Rust |
| **License** | MIT |
| **Transport** | stdio |
| **Official registry** | `io.github.edithatogo/fyi-mcp` |
| **Version** | 0.1.2 |
| **Tags / categories** | legal, foi, oia, alaveteli, privacy, sqlite, productivity |

### Description (short)

Multi-jurisdiction Freedom of Information / Official Information request tracker MCP server for Alaveteli platforms. Local SQLite storage; tools for requests, authorities, correspondence, offline sync, and health checks.

### Description (long)

FYI MCP (`fyi-mcp`) is a native Rust Model Context Protocol server for managing FOI/OIA requests against any Alaveteli deployment (FYI.org.nz, WhatDoTheyKnow, RightToKnow, FragDenStaat, and more). It keeps data local in SQLite and exposes a stdio MCP tool surface for request CRUD, authority import, correspondence lookup, offline sync monitoring, conflict resolution, and database health. Companion CLI: `fyi-cli` in the same repository.

### Features

- Track and manage FOI/OIA requests locally
- Authority list import and lookup
- Correspondence retrieval per request
- Offline sync status, queue monitor, and conflict resolution
- Database health check (`check_status`)
- Ephemeral in-memory mode for demos (`FYI_MCP_EPHEMERAL=1`)
- Published to the Official MCP Registry as `io.github.edithatogo/fyi-mcp`

### Tools (names)

`list_requests`, `retrieve_request`, `create_request`, `update_request`, `delete_request`, `list_authorities`, `import_authorities`, `sync_monitor`, `sync_conflicts`, `sync_resolve_conflict`, `sync_status`, `check_status`

### Connection / install information

**From source (recommended for non-Windows platforms):**

```bash
git clone https://github.com/edithatogo/fyi-cli.git
cd fyi-cli
cargo install --path crates/fyi-mcp
```

**MCP client config (stdio):**

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

**Windows MCPB (GitHub Release):**

- https://github.com/edithatogo/fyi-cli/releases/download/fyi-mcp-v0.1.2/fyi-mcp-0.1.2-win32.mcpb

**Container (when GHCR publish is live):**

```text
ghcr.io/edithatogo/fyi-mcp
```

See [`docs/containers.md`](../../../docs/containers.md).

### Related links

- Official MCP Registry name: `io.github.edithatogo/fyi-mcp`
- `server.json`: https://github.com/edithatogo/fyi-cli/blob/master/server.json
- Glama: https://glama.ai/mcp/servers/edithatogo/fyi-cli
- Smithery: https://smithery.ai/server/@edithatogo/fyi-mcp
- Awesome-MCP-Servers PR: https://github.com/punkpeye/awesome-mcp-servers/pull/9693

---

## Issue body template (paste into mcp.so submit issue)

```markdown
## Server name
FYI MCP (`fyi-mcp`)

## Repository
https://github.com/edithatogo/fyi-cli

## Description
Multi-jurisdiction FOI/OIA request tracker for Alaveteli platforms. Local SQLite storage with MCP tools for requests, authorities, correspondence, offline sync, and health. Official MCP Registry: `io.github.edithatogo/fyi-mcp`.

## Language / runtime
Rust (stdio MCP server binary)

## Install
```bash
cargo install --path crates/fyi-mcp
# or Windows MCPB from GitHub Releases
```

## MCP config
```json
{
  "mcpServers": {
    "fyi-mcp": {
      "command": "fyi-mcp",
      "args": [],
      "env": { "DATABASE_URL": "sqlite:fyi_system.db" }
    }
  }
}
```

## License
MIT

## Official registry
`io.github.edithatogo/fyi-mcp` @ 0.1.2
```

## Post-submit

1. Record the public mcp.so URL in `docs/registry-distribution-matrix.md`.
2. Flip status **assets-ready** → **live** only after the page is public.
