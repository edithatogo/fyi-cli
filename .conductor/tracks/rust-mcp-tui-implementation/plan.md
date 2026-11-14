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
- [~] Task: Integrate a bounded read-only local metadata adapter and CLI launch against merged #434/#440/#442; implementation and hosted tests pass in #448, with coordinator integration pending. Timelines remain explicitly unavailable in this increment.
- [x] Task: Verify terminal startup/cleanup and asynchronous input with executable runtime and Linux PTY tests in #448.
- [ ] Task: Verify the original performance budgets with actual runtime measurements.
- [ ] Task: Record actual user/manual terminal verification; the historical completion checkbox had no supporting evidence.

2026-11-14 presentation baseline audit (before #435): `tui` returned
`unsupported_command`. `AppState::new()` contained sample records and counts;
tables did not scroll with selection and keyboard handling consumed key releases.
The initial regression checkpoint was tests only. The combined CLI/MCP/TUI track
is not being closed.


Implementation evidence: [TUI presentation acceptance](tui-acceptance-evidence.md)
and [integration contract](../../../docs/tui-presentation-contract.md), PR #435.
The focused suite passes 39 tests with 14 complete-buffer fixtures. The two
presentation tasks are complete. The runtime increment below verifies dispatch,
terminal lifecycle and asynchronous input in #448; integration, manual verification
and the original performance budgets remain open. Historical
CLI/MCP checkboxes above are inherited records, not reverified by this workstream.

## Local runtime increment — 2026-11-14

Baseline: `39591dcf91bb50ed847b0d36bdc3bd670a7e60a7`, including merged
#435 presentation and #434/#440/#442 local APIs. No inaccessible bundle or frozen
local-retirement change is assumed integrated. This increment adds a bounded
SQLx metadata projection, an owned presentation adapter, asynchronous input and
terminal lifecycle handling. No schema, migration, source, MCP, localization,
shared CI or shared Conductor registry change is planned.

- [x] Executable red/green CLI, adapter, lifecycle and pseudo-terminal contracts:
  24 new executable tests pass at `18cc6eeac934354ee9a25240af2b5be65965e2db`.
- [x] Static privacy-safe diagnostics and read-only existing-file access, including
  database byte/file preservation and panic/backtrace-negative tests.
- [x] Explicit manual accessibility and performance acceptance procedure in
  `docs/tui-runtime-acceptance.md`; actual operator and measurement evidence is open.
- [x] Exact-head hosted verification: all 12 app-matched required checks, Rust
  1.99.1 MSRV and Conductor pass at the recorded implementation head. Workspace:
  615 passed, one existing ignored worker; unchanged line-coverage gate: 94.34%.
- [ ] Exclusive final reconciliation/verification/merge slot requested from the
  sole `Issue fixes and PR merges` coordinator; no slot grant or independent master
  refresh/merge is assumed. Reverify the resulting final head before integration.
