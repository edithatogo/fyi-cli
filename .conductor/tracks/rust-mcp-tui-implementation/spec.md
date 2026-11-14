# Specification: rust-mcp-tui-implementation (Phase 2)

## Overview
This track implements the primary user-facing interfaces for the new Rust application: the CLI parser (`clap`), the Model Context Protocol (MCP) server integration (`fyi-mcp`), and the high-performance terminal UI (`ratatui`).

## Functional Requirements
1. **Command Line Interface (`clap`):**
   - Provide standard fyi commands (`init-db`, `register-request`, `list-requests`, `mcp-server`, `tui`).
   - Use clap's derive macros for configuration, argument validation, and automated shell autocompletion.
2. **Model Context Protocol (MCP) Server:**
   - Expose core FYI request management functions as JSON-RPC MCP Tools.
   - Implement MCP server daemon allowing external AI agents to securely query database states and fetch requests.
3. **Interactive TUI Dashboard (`ratatui`):**
   - Design a full-screen interactive dashboard displaying request statuses, drafts, timelines, and alert reports.
   - Handle input events asynchronously to prevent rendering blocks.

## Non-Functional Requirements
- **Interactivity:** Fluid UI rendering > 30 FPS.
- **Resource Footprint:** System RAM `< 20MB`.

## Acceptance Criteria
- CLI parses command inputs correctly.
- MCP server runs in standard I/O mode and responds to protocol messages.
- TUI boots and is navigatable via keyboard shortcuts.


## TUI acceptance reconciliation — 2026-11-14

The original Phase 2.3 completion claim did not establish CLI boot or asynchronous
input: the `tui` command is currently deliberately unsupported. This work retains
Ratatui and the existing `AppState` / `draw_ui` entry points while completing the
bounded read-only presentation separately from the request-management adapter.

Presentation acceptance requires supplied requests/statuses, draft metadata,
genuine supplied timelines, honest snapshot metrics, explicit loading/empty/error
states, default full redaction, local filtering, keyboard navigation and readable
small-terminal rendering. Rendering and interaction receive no database, network
or keyring handle and can emit no write/export/credential command. Synthetic
fixtures and deterministic complete-buffer/interaction/resize tests establish
these requirements. Unsupported prototype save/delete/autosave/keyring helpers,
export promises and demonstration defaults are removed from this unwired module.

See [the concrete input/effect contract](../../../docs/tui-presentation-contract.md)
and [acceptance evidence](tui-acceptance-evidence.md). A later isolated adapter
must open existing local data read-only, supply actual authority metadata, preserve
unknown/partial data, and verify terminal lifecycle and input handling before
changing CLI dispatch. Buffer tests do not establish the original >30 FPS or
<20 MB RAM budgets. Those runtime requirements remain open, and the combined
CLI/MCP/TUI track is not complete.
