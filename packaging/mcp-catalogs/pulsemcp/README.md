# PulseMCP submission package (`fyi-mcp`)

**Issue:** [#100](https://github.com/edithatogo/fyi-cli/issues/100)  
**Status:** **assets-ready** (not submitted / not verified live)  
**Package metadata:** [`submission.json`](./submission.json)

## What is PulseMCP?

[PulseMCP](https://www.pulsemcp.com/) is a daily-updated directory of MCP servers and clients.
It ingests entries from the [Official MCP Registry](https://registry.modelcontextprotocol.io/)
and also accepts manual submissions.

## Canonical identity

| Field | Value |
|-------|-------|
| Display name | FYI MCP |
| Binary / package | `fyi-mcp` |
| Official registry name | `io.github.edithatogo/fyi-mcp` |
| Version (current) | `0.1.2` |
| Repository | https://github.com/edithatogo/fyi-cli |
| MCP crate | `crates/fyi-mcp` |
| `server.json` | [`server.json`](../../../server.json) |
| License | MIT |

## Short listing copy

**Title:** FYI MCP  

**One-liner:** Multi-jurisdiction FOI/OIA request tracker for Alaveteli platforms, with local SQLite storage and MCP tools for requests, authorities, correspondence, offline sync, and health.

**Longer blurb:**

> FYI MCP (`fyi-mcp`) is a Rust Model Context Protocol server for managing Freedom of Information / Official Information requests against Alaveteli-based platforms (FYI.org.nz, WhatDoTheyKnow, RightToKnow, and more). It exposes request lifecycle, authority import, correspondence lookup, offline sync monitoring, conflict resolution, and database health tools over stdio. Data stays local (SQLite); optional ephemeral mode for sandboxed demos.

## Submission channels (operator)

Prefer in this order:

1. **Official MCP Registry** — already **live** as `io.github.edithatogo/fyi-mcp`. PulseMCP
   documents weekly ingest from the official registry; if the listing is missing after a week,
   use the manual path below.
2. **Manual submit:** https://www.pulsemcp.com/submit  
   - Type: MCP Server  
   - URL: `https://github.com/edithatogo/fyi-cli` (or `…/tree/master/crates/fyi-mcp`)
3. **Adjustments:** email `hello@pulsemcp.com` with the official registry name and desired
   metadata fixes.

## Pre-submit checklist

- [ ] `server.json` version matches the latest published release
- [ ] Official registry API still returns the server as active/latest
- [ ] Description / keywords in [`submission.json`](./submission.json) match `server.json` + README
- [ ] After submit: record the public PulseMCP URL in
      [`docs/registry-distribution-matrix.md`](../../../docs/registry-distribution-matrix.md)
- [ ] Only then flip status from **assets-ready** → **live**

## Related live listings (proof of existing ecosystem presence)

- Official MCP Registry: search `fyi-mcp` on https://registry.modelcontextprotocol.io/
- Glama: https://glama.ai/mcp/servers/edithatogo/fyi-cli
- Smithery: https://smithery.ai/server/@edithatogo/fyi-mcp

## Do not claim live

Until a public PulseMCP server page is verified, keep matrix status at **assets-ready** or
**planned**. Do not invent a listing URL.
