# API Contract Inventory

Generated from checked-in source and test evidence for the `api-contract-hardening-20260630` Conductor track.

## Contract Matrix

| Surface | Contract | Coverage | Risk |
| --- | --- | --- | --- |
| rust_api_payloads: Rust API payload structs | Serde JSON shapes for Alaveteli request, correspondence, create, update, and action responses. | Round-trip payload tests, optional field compatibility, and wiremock success responses. Evidence: `crates/fyi-core/src/api.rs`; tests: `crates/fyi-core/src/api.rs::tests`. | medium |
| rust_sync_client: Rust sync API client | Search JSON/RSS, request CRUD, correspondence JSON/multipart writes, validated state updates, authority discovery/filtering, prefilled URLs, authority-scoped feeds, health/version, watched feeds, retry queues, and merge/conflict preservation. | WireMock coverage includes successful and malformed feeds, multipart and size rejection, state validation, authority filtering, URL encoding, health/version, HTTP 401/403/404/429/5xx failures, push retry, scheduler, and conflict merge behavior. Evidence: `crates/fyi-core/src/sync.rs`; tests: `crates/fyi-core/src/sync.rs::tests`. | medium |
| rust_cli_sync_surface: Rust CLI sync commands | CLI presentation for sync status, pull, push, conflicts, and conflict resolution. | Parser and E2E coverage for sync command shapes and database-backed output. Evidence: `crates/fyi-cli/src/main.rs`, `crates/fyi-cli/tests/cli_tests.rs`, `crates/fyi-cli/tests/e2e_tests.rs`; tests: `crates/fyi-cli/src/main.rs::tests`, `crates/fyi-cli/tests/cli_tests.rs`, `crates/fyi-cli/tests/e2e_tests.rs`. | medium |
| rust_mcp_sync_surface: Rust MCP API-adjacent tools | JSON-RPC tools exposing request CRUD, authority import, sync status, sync monitor, conflicts, and resolution. | In-process JSON-RPC tests for tool listing, request flows, sync status, conflict, and monitor tools. Evidence: `crates/fyi-mcp/src/main.rs`; tests: `crates/fyi-mcp/src/main.rs::tests`. | medium |
| archive_public_web: Archive public-web endpoints | FYI public request pages, attachments, discovery feeds, diff manifests, and archive health JSON. | Mocked discovery, capture, diff, and health tests; live discovery smoke test remains environment-gated. Evidence: `src/fyi_system/discovery.py`, `src/fyi_system/archive_capture.py`, `src/fyi_system/archive_diff.py`, `src/fyi_system/archive_health.py`; tests: `tests/test_discovery.py`, `tests/test_discovery_smoke.py`, `tests/test_archive_capture.py`, `tests/test_archive_diff.py`, `tests/test_archive_health.py`. | medium |

## Contract Gaps And Residual Risk

- **closed** `rust_sync_client`: HTTP 401/403/404/429/5xx responses are covered by mocked non-secret error contract tests.
- **closed** `rust_sync_client`: Malformed JSON, missing required fields, and unexpected optional fields are covered by regression tests.
- **closed** `rust_sync_client`: Multipart correspondence uploads are bounded to 50 MiB per attachment and reject oversized files before network I/O.
- **low** `rust_sync_client`: State vocabulary is validated before PUT; graph-level transitions require the current remote state, which is not part of the current payload.
- **closed** `rust_cli_sync_surface`: Sync failures preserve text output and now emit structured JSON errors; unit coverage protects both modes.
- **informational** `rust_mcp_sync_surface`: MCP currently exposes local SQLite tools rather than remote SyncClient operations, so upstream API error normalization is not an applicable boundary.
- **medium** `archive_public_web`: Live public-web smoke remains opt-in to avoid network-dependent CI.

## Phase 1 Next Actions

1. Keep live FYI smoke tests opt-in with `FYI_LIVE_SMOKE=1`.
2. Run mocked contract tests before release.
3. Refresh fixtures when FYI/Alaveteli response shapes change.
