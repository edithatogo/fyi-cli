# API Contract Inventory

Generated from checked-in source and test evidence for the `api-contract-hardening-20260630` Conductor track.

## Contract Matrix

| Surface | Contract | Coverage | Risk |
| --- | --- | --- | --- |
| rust_api_payloads: Rust API payload structs | Serde JSON shapes for Alaveteli request, correspondence, create, update, and action responses. | Round-trip payload tests, optional field compatibility, and wiremock success responses. Evidence: `crates/fyi-core/src/api.rs`; tests: `crates/fyi-core/src/api.rs::tests`. | medium |
| rust_sync_client: Rust sync API client | GET /api/v2/request.json, GET /api/v2/request/{id}.json, POST /api/v2/request, watched feed pulls, retry queues, and merge/conflict preservation. | Mocked successful pull, feed pull, health, push retry, scheduler, and conflict merge behavior. Evidence: `crates/fyi-core/src/sync.rs`; tests: `crates/fyi-core/src/sync.rs::tests`. | high |
| rust_cli_sync_surface: Rust CLI sync commands | CLI presentation for sync status, pull, push, conflicts, and conflict resolution. | Parser and E2E coverage for sync command shapes and database-backed output. Evidence: `crates/fyi-cli/src/main.rs`, `crates/fyi-cli/tests/cli_tests.rs`, `crates/fyi-cli/tests/e2e_tests.rs`; tests: `crates/fyi-cli/src/main.rs::tests`, `crates/fyi-cli/tests/cli_tests.rs`, `crates/fyi-cli/tests/e2e_tests.rs`. | medium |
| rust_mcp_sync_surface: Rust MCP API-adjacent tools | JSON-RPC tools exposing request CRUD, authority import, sync status, sync monitor, conflicts, and resolution. | In-process JSON-RPC tests for tool listing, request flows, sync status, conflict, and monitor tools. Evidence: `crates/fyi-mcp/src/main.rs`; tests: `crates/fyi-mcp/src/main.rs::tests`. | medium |
| archive_public_web: Archive public-web endpoints | FYI public request pages, attachments, discovery feeds, diff manifests, and archive health JSON. | Mocked discovery, capture, diff, and health tests; live discovery smoke test remains environment-gated. Evidence: `src/fyi_system/discovery.py`, `src/fyi_system/archive_capture.py`, `src/fyi_system/archive_diff.py`, `src/fyi_system/archive_health.py`; tests: `tests/test_discovery.py`, `tests/test_discovery_smoke.py`, `tests/test_archive_capture.py`, `tests/test_archive_diff.py`, `tests/test_archive_health.py`. | medium |

## High-Risk Untested Paths

- **high** `rust_sync_client`: HTTP 401/403/429/5xx responses are not yet asserted as typed, normalized, non-secret contract errors.
- **high** `rust_sync_client`: Malformed JSON and missing required sync payload fields need explicit regression tests.
- **medium** `rust_cli_sync_surface`: CLI output has sync coverage, but API failure presentation needs end-to-end assertions.
- **medium** `rust_mcp_sync_surface`: MCP database errors are surfaced, but upstream API error normalization is indirect until sync error types are added.
- **medium** `archive_public_web`: Public-web archive fixtures cover mocked paths; release checklists should keep live smoke tests opt-in and polite.

## Phase 1 Next Actions

1. Add malformed and partial response tests for Rust sync parsing.
2. Add mocked HTTP failure tests for 401/403, 404, 429, and 5xx.
3. Reuse this matrix as the release checklist input for fixtures.
