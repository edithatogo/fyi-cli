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
- [x] Task: Reconcile rendering and keyboard contracts using deterministic offline fixtures.
- [x] Task: Complete the bounded read-only Ratatui presentation with typed input snapshots, honest metrics, timelines, privacy and resize behavior.
- [~] Task: Integrate a bounded read-only local metadata adapter and CLI launch against merged #434/#440/#442; timelines remain explicitly unavailable in this increment.
- [ ] Task: Verify terminal startup/cleanup, asynchronous input and original performance budgets with runtime evidence.
- [ ] Task: Record actual user/manual terminal verification; the historical completion checkbox had no supporting evidence.

2026-11-14 audit: `tui` currently returns `unsupported_command`. The existing
`AppState::new()` contains sample records and counts; tables do not scroll with
selection and keyboard handling consumes key releases. The initial regression
checkpoint is tests only. The combined CLI/MCP/TUI track is not being closed.


Implementation evidence: [TUI presentation acceptance](tui-acceptance-evidence.md)
and [integration contract](../../../docs/tui-presentation-contract.md), PR #435.
The focused suite passes 39 tests with 14 complete-buffer fixtures. The two
presentation tasks are complete; dispatch, terminal lifecycle, asynchronous input,
manual verification and the original performance budgets remain open. Historical
CLI/MCP checkboxes above are inherited records, not reverified by this workstream.

## Local runtime increment — 2026-11-14

Baseline: `39591dcf91bb50ed847b0d36bdc3bd670a7e60a7`, including merged
#435 presentation and #434/#440/#442 local APIs. No inaccessible bundle or frozen
local-retirement change is assumed integrated. This increment adds a bounded
SQLx metadata projection, an owned presentation adapter, asynchronous input and
terminal lifecycle handling. No schema, migration, source, MCP, localization,
shared CI or shared Conductor registry change is planned.

- [~] Executable red/green CLI, adapter, lifecycle and pseudo-terminal contracts.
- [ ] Static privacy-safe diagnostics and read-only existing-file access.
- [ ] Explicit manual accessibility and performance acceptance procedure/evidence.
- [ ] Exact-head hosted verification and exclusive integration slot requested from
  the sole `Issue fixes and PR merges` coordinator; no independent master refresh
  or merge while another owner holds the slot.
