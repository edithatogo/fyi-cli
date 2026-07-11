# FYI MCP

Model Context Protocol server for FYI request management.

[![smithery badge](https://smithery.ai/badge/edithatogo/fyi-mcp)](https://smithery.ai/servers/edithatogo/fyi-mcp)

## Registry

- mcp-name: `io.github.edithatogo/fyi-mcp`

## Build

```bash
cargo build --release --package fyi-mcp
```

## Run

```bash
cargo run --package fyi-mcp
```

Remote capabilities are disabled by default. See
[`docs/remote-mcp-security.md`](../../docs/remote-mcp-security.md) before
enabling an explicit instance allowlist; remote write capability is not
available from this release.

## Glama

The repository root contains the Dockerfile used for Glama release builds. It
builds the `fyi-mcp` binary and starts it over stdio with an ephemeral SQLite
database so registry inspection can list and call tools without external state.

See [../../GLAMA.md](../../GLAMA.md) for the release checklist.
