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

At the Phase 2.3 baseline, the `tui` command returned `unsupported_command`.
PR #435 verified the bounded presentation using Ratatui and the existing
`AppState` and `draw_ui` entry points.

Presentation acceptance requires supplied requests/statuses, draft metadata,
genuine supplied timelines, honest snapshot metrics, explicit loading/empty/error
states, default full redaction, local filtering, keyboard navigation and readable
small-terminal rendering. Rendering and interaction receive no database, network
or keyring handle and can emit no write/export/credential command. Synthetic
fixtures and deterministic complete-buffer/interaction/resize tests establish
these requirements. Unsupported prototype save/delete/autosave/keyring helpers,
export promises and demonstration defaults are removed from this unwired module.

PR #448 integrated the adapter and CLI launch at `c12a023`. The adapter opens
existing local data read-only, supplies authority metadata and preserves unknown
and partial data. The Linux PTY tests verify keyboard navigation and terminal
cleanup. The adapter reports timelines as unavailable, so verification with
actual timeline data remains open. Manual terminal and assistive-technology
checks and the original >30 FPS and <20 MB RAM budgets also remain open.
[Presentation contract](../../../docs/tui-presentation-contract.md) · [Acceptance evidence](tui-acceptance-evidence.md)
