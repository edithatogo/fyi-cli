# Rust dependency rationalisation — fyi-cli (2026-11-13)

**Status:** scoped Conductor adoption contract; code substitutions not yet implemented or certified by this file.

## Repository-specific decisions

- Reconcile existing `frontier-rust-convergence-20261113` and `source-adapter-boundary-20260812` and PR #402; do not replace their milestones.
- **Verified:** core uses Tokio, SQLx, Reqwest, Serde, Thiserror 1.x, Anyhow, Proptest; CLI uses Clap, Ratatui, Tracing and Tokio `full` features.
- **Audit first:** duplicate versions/features across Tokio, SQLx, Reqwest, Rustls, crypto, errors; do not equate compatible version ranges with duplicated builds.
- **Evaluate:** Thiserror 1->2 only with error golden tests; trim Tokio `full` only after mapping actual runtime capabilities in core/CLI/MCP.
- **Extend:** receipt/replay property testing, redacted Insta snapshots and bounded provider benchmarks.
- **Retain:** Tor/security boundaries, Clap, Ratatui, HTTP/SQL infrastructure. Defer Rayon, Polars, Crossbeam and PyO3 without demonstrated value.

## Delivery and acceptance contract
- Reconcile existing Conductor tracks and ownership before editing code; do not duplicate completed tasks.
- Write a failing behavioural/regression test first for each runtime change; preserve APIs, schema versions, stdout/stderr, exit codes, error codes and MSRV or explicitly version breaking changes.
- Run the existing repo gates: targeted tests, full workspace tests, fmt, clippy and contract/interop checks. Record actual passing CI SHA; this document is planning, not validation.
- Inventory duplicate crate versions/features with `cargo tree -d` and `cargo tree -e features`; review unused dependencies, `cargo audit`, `cargo deny`, licence, MSRV, SBOM and release implications where applicable.
- Benchmark performance proposals with fixed fixtures and baseline/candidate receipts, including correctness, memory, CPU and determinism.
- Decision for each candidate must be recorded as adopt, retain, reject or defer with evidence, risks and rollback.
- Do not expose identifiable, secret, copyrighted or otherwise restricted raw data in tracing, snapshots, benchmarks or CI artifacts.
