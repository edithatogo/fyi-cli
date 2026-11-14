# Plan: rust-mcp-tui-implementation (Phase 2)

## Phase 2.1: Command-Line Interface (`clap`)
- [x] Task: Write failing arguments verification tests for CLI inputs
- [x] Task: Implement CLI commands and shell completions using `clap` in `fyi-cli`
- [x] Task: Conductor - User Manual Verification 'Phase 2.1: CLI Parsing' (Protocol in workflow.md)

## Phase 2.2: Model Context Protocol (MCP) Server Daemon
- [x] Task: Write failing tool call JSON-RPC request-response tests for the MCP protocol
- [x] Task: Implement MCP tools registry and JSON-RPC transport in `fyi-mcp`
- [x] Task: Conductor - User Manual Verification 'Phase 2.2: MCP Server' (Protocol in workflow.md)


## Phase 2.3: Terminal UI Dashboard (`ratatui`)
- [~] Task: Reconcile rendering and keyboard contracts using deterministic offline fixtures.
- [~] Task: Complete the bounded read-only Ratatui presentation with typed input snapshots, honest metrics, timelines, privacy and resize behavior.
- [ ] Task: Integrate a read-only local-data adapter and terminal launch after the request-management branch has merged; `main.rs` is owned by that workstream meanwhile.
- [ ] Task: Verify terminal startup/cleanup, asynchronous input and original performance budgets with runtime evidence.
- [ ] Task: Record actual user/manual terminal verification; the historical completion checkbox had no supporting evidence.

2026-11-14 audit: `tui` currently returns `unsupported_command`. The existing
`AppState::new()` contains sample records and counts; tables do not scroll with
selection and keyboard handling consumes key releases. The initial regression
checkpoint is tests only. The combined CLI/MCP/TUI track is not being closed.

