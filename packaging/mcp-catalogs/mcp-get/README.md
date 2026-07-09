# mcp-get install notes (`fyi-mcp`)

**Issue:** [#103](https://github.com/edithatogo/fyi-cli/issues/103)  
**Status:** **assets-ready** (install documentation prepared; external registry not claimed live)

## Context

Historically, [mcp-get](https://mcp-get.com/) / [`michaellatman/mcp-get`](https://github.com/michaellatman/mcp-get)
provided a CLI + community registry for installing MCP servers. The upstream project has been
**archived / deprecated** (read-only; maintainers point users toward broader ecosystem registries).

This package still records **install notes** so:

1. Operators have a single place for “how to install `fyi-mcp`” suitable for any installer CLI.
2. If a successor tool reuses mcp-get-style package metadata, the fields below can be pasted quickly.
3. Issue #103 stays trackable without inventing a live mcp-get listing.

**Prefer today:** Official MCP Registry (`io.github.edithatogo/fyi-mcp`), Smithery, Glama, source
install, MCPB release assets, and (when published) `ghcr.io/edithatogo/fyi-mcp`.

## Package identity

| Field | Value |
|-------|-------|
| Name | `fyi-mcp` |
| Display name | FYI MCP |
| Official registry | `io.github.edithatogo/fyi-mcp` |
| Version | `0.1.2` |
| Repository | https://github.com/edithatogo/fyi-cli |
| Homepage | https://github.com/edithatogo/fyi-cli |
| License | MIT |
| Runtime | native binary (Rust), stdio |
| Categories | legal, productivity |

## Install notes (all channels)

### A. From source (canonical)

```bash
git clone https://github.com/edithatogo/fyi-cli.git
cd fyi-cli
cargo build --release --locked --package fyi-mcp
# binary: target/release/fyi-mcp
cargo install --path crates/fyi-mcp
```

### B. MCPB (Windows release asset)

```text
https://github.com/edithatogo/fyi-cli/releases/download/fyi-mcp-v0.1.2/fyi-mcp-0.1.2-win32.mcpb
```

SHA-256 (also in root `server.json`):

```text
87ef957d9c4dbf30322e7872ea4f23f92f8640344ceac2bb8a3b5fa22bc7fc2f
```

### C. Container (when GHCR publish is live)

```bash
docker pull ghcr.io/edithatogo/fyi-mcp:latest
docker run --rm -i ghcr.io/edithatogo/fyi-mcp:latest
```

See [`docs/containers.md`](../../../docs/containers.md).

### D. Client configuration (stdio)

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

Ephemeral / demo:

```json
{
  "mcpServers": {
    "fyi-mcp": {
      "command": "fyi-mcp",
      "args": [],
      "env": {
        "FYI_MCP_EPHEMERAL": "1",
        "DATABASE_URL": "sqlite::memory:"
      }
    }
  }
}
```

## Draft installer metadata (mcp-get-style)

If submitting to a package registry that expects npm/npx-style or generic package fields:

```json
{
  "name": "fyi-mcp",
  "description": "FOI/OIA request tracker MCP server for Alaveteli platforms (local SQLite).",
  "vendor": "edithatogo",
  "sourceUrl": "https://github.com/edithatogo/fyi-cli",
  "homepage": "https://github.com/edithatogo/fyi-cli",
  "license": "MIT",
  "runtime": "binary",
  "command": "fyi-mcp",
  "args": [],
  "env": {
    "DATABASE_URL": "sqlite:fyi_system.db"
  },
  "officialRegistry": "io.github.edithatogo/fyi-mcp",
  "install": {
    "cargo": "cargo install --path crates/fyi-mcp",
    "mcpb": "https://github.com/edithatogo/fyi-cli/releases/download/fyi-mcp-v0.1.2/fyi-mcp-0.1.2-win32.mcpb",
    "docker": "ghcr.io/edithatogo/fyi-mcp"
  }
}
```

## Operator checklist

- [ ] Confirm whether any mcp-get successor still accepts submissions
- [ ] If yes: submit using the draft metadata above and record the public URL
- [ ] If no: leave matrix status **assets-ready** / **planned** and rely on Official Registry + docs
- [ ] Never mark **live** without a public install page or registry entry

## Related

- Official `server.json`: [`server.json`](../../../server.json)
- MCPB manifest: [`packaging/mcpb/fyi-mcp/manifest.json`](../../mcpb/fyi-mcp/manifest.json)
- Distribution matrix: [`docs/registry-distribution-matrix.md`](../../../docs/registry-distribution-matrix.md)
