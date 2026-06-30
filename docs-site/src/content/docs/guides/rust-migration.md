---
title: Rust Core Migration
description: Guide to the FYI Request System rewrite in Rust.
---

## Overview

To improve performance, memory efficiency, and provide native Model Context Protocol (MCP) integrations, the core components of the FYI Request System have been ported from Python to Rust.

The rewrite is structured as a Cargo Workspace containing three primary crates:

1. **`fyi-core`** (`crates/fyi-core`): Shared core module implementing API clients, database structures, encryption engines, and Tor connection pools.
2. **`fyi-cli`** (`crates/fyi-cli`): The command-line client parsing subcommands with `clap` and rendering the terminal UI using `ratatui`.
3. **`fyi-mcp`** (`crates/fyi-mcp`): A native Model Context Protocol (MCP) daemon serving JSON-RPC over stdin/stdout.

Published MCP cross-references:

- [GitHub MCP Registry](https://registry.modelcontextprotocol.io/servers/io.github.edithatogo/fyi-mcp)
- [Smithery server page](https://smithery.ai/servers/edithatogo/fyi-mcp)
- Bundle manifest: `server.json`

---

## Workspace Structure

```
C:\Users\60217257\OneDrive - Flinders\repos\legal-nz\fyi-cli\
├── Cargo.toml                  # Workspace root
└── crates/
    ├── fyi-core/               # Shared engine library
    │   ├── Cargo.toml
    │   └── src/
    │       ├── api.rs          # Alaveteli API payloads
    │       ├── db.rs           # SQLite async migrations & pools
    │       ├── security.rs     # AES-GCM + keyring + zeroize
    │       └── tor.rs          # arti Tor routing SOCKS5 server
    ├── fyi-cli/                # Command Line Tool
    │   ├── Cargo.toml
    │   └── src/
    │       ├── main.rs         # clap CLI parser
    │       └── tui.rs          # ratatui terminal dashboard
    └── fyi-mcp/                # Model Context Protocol Daemon
        ├── Cargo.toml
        └── src/
            └── main.rs         # JSON-RPC server
```

---

## Verifications & Testing

Quality is guaranteed using modern verification suites:

- **Property-based Verification**: Generative validation of database operations and cryptographic algorithms using `proptest`.
- **E2E Mock Testing**: Integration testing of client/server networks using `wiremock`.
- **Heap Allocation Profiling**: Dynamic allocation checks using the `dhat` heap profiler.
