# Acceptance evidence

## Reconciled baseline (2026-11-14)

Commit `a816dcd` had a pending plan and metadata despite the registry's
implemented label. Its Python contract sensor checked a hand-maintained
inventory without exercising registration, schemas or transports.

Reproduced with Rust 1.99.1: four executable stdio regression tests failed.
The unmodified official Python MCP SDK 1.29.0 failed all four initial client
scenarios, including protocol negotiation and HTTP Origin rejection. These
were local synthetic tests, not live service calls.

## Slice 1: wire contract

Implements supported-version negotiation, ping, strict JSON-RPC envelopes,
silent notifications, bounded legacy batches, private parse/dispatch errors,
authentication before HTTP parsing, browser-origin rejection, HTTP body bounds,
and bounded stdio framing. Existing application handlers and source transports
are unchanged.

Run `cargo test -p fyi-mcp --locked` for the unit and executable sensors.
The PR records final local and hosted-head evidence; no unobserved check is
marked complete here.

PR #438 implements the wire slice at head
`71ec8aa093b9f05265d737e70c46964eb38b1bbd`; 44 local tests passed with
Rust 1.99.1, and Clippy, formatting and the existing verifiers passed. All required final-head checks passed before squash merge
`19f3cf376101500036951f9a3d3733da49f0df8b`. The optional self-hosted
AI security run was queued and is not claimed as passing.

## Slice 2: confirmation and operation budgets

The original confirmation gate failed four regressions for token opacity,
duplicate preparation, changed payloads and bounded capacity. The original
policy failed oversized-response accounting. Additional review regressions
showed invalid confirmations consuming the final operation allowance and
completed replay failing after budget exhaustion.

The repaired gate uses OS-generated opaque tokens, monotonic expiry, immutable
scope/payload binding, atomic admission and bounded retained state. Policy
permits enforce per-instance concurrency, count attempts before I/O, account
rejected response bytes, enforce duration and release on cancellation. Synthetic
future tests cover success, private upstream failure, timeout, oversized output,
policy changes, invalid confirmation and changed commit payload. No live write
is part of this verification. Rust 1.99.1 verification passed 97 MCP tests
(92 unit/boundary and five executable), Clippy with warnings denied, formatting,
and the Conductor contract check.

Attachment tests reject traversal and observed in-root file/directory symlinks.
Process restart durability, attachment byte immutability and source raw-byte/
retry/fault contracts remain separate unverified dependencies.

## Remaining closure evidence

- Versioned executable tool/schema/annotation and result/error fixtures.
- Official SDK success on stdio and HTTP after all contract fixes.
- Confirmation expiry, scoped replay and concurrent consumption evidence.
- MCP argument validation, private runtime errors, and remote audit boundaries.
- Actual latency/RSS results, concurrency/time/byte budget enforcement.
- Deterministic timeout/disconnect/malformed upstream/cache/clock fault evidence,
  reusing acquisition-owned fixtures and transports.
- Mutation results and supported Windows/macOS/Linux plus Rust/MSRV matrix.
- Final-head required Actions, issue #173/epic #169 reconciliation, then archive
  only if all original acceptance criteria are met.

Source acquisition, SQLite core, localization, CLI dispatch, TUI, packaging,
and shared CI infrastructure remain with their assigned workstreams.
