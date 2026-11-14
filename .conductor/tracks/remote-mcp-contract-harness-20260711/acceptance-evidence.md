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
