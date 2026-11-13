# Contract Testing Applicability Analysis

Contract testing applicability for fyi-cli API boundaries.
Generated during Conductor track `quality-frontier-testing-cicd-20261113` (issue #280).

## Existing Contract Test Infrastructure

| Test File | Covers | Status |
|-----------|--------|--------|
| `tests/test_api_contract.py` | FYI.org.nz API request/response schemas | Active |
| `tests/test_api_contract_inventory.py` | API contract inventory (surfaces, coverage, risk) | Active |
| `tests/test_australian_capability_contracts.py` | Australian jurisdiction capability contracts | Active |
| `tests/test_harness_contract.py` | Test harness layer contracts | Active |
| `tests/test_remote_mcp_contract.py` | Remote MCP compatibility contracts | Active |
| `scripts/api_contract_inventory.py` | Contract inventory generation script | Active |

## API Boundaries and Contract Coverage

| API Boundary | Module | CLI Command | Contract Tests | Status |
|-------------|--------|-------------|----------------|--------|
| FYI.org.nz request page | `src/fyi_system/fetch.py` | `fetch-request-page` | Yes (`test_api_contract.py`) | Covered |
| FYI.org.nz discover bodies | `src/fyi_system/discovery.py` | `discover-bodies` | Yes (`test_api_contract.py`) | Covered |
| FYI.org.nz search feed | `src/fyi_system/discovery.py` | `discover` | Yes (`test_api_contract.py`) | Covered |
| FYI.org.nz numeric ID probing | `src/fyi_system/discovery.py` | `discover --backfill-ids` | Partial | **Partial** |
| Internet Archive CDX | `src/fyi_system/internet_archive_cdx.py` | `internet-archive-cdx` | Yes (`test_internet_archive_cdx.py`) | Covered |
| Internet Archive replay | `src/fyi_system/internet_archive_replay.py` | `internet-archive-replay` | Yes (`test_internet_archive_replay.py`) | Covered |
| Historical index | `src/fyi_system/historical_index.py` | `historical-index` | Yes (`test_historical_index.py`) | Covered |
| Australian capabilities | `src/fyi_system/australian_capabilities.py` | N/A (library) | Yes (`test_australian_capability_contracts.py`) | Covered |
| Remote MCP | `crates/fyi-mcp/src/` | N/A (Rust MCP server) | Yes (`test_remote_mcp_contract.py`) | Covered |
| Import authorities (CSV/URL) | `src/fyi_system/importers.py` | `import-authorities` | No | **Gap** |
| Ingest feed (RSS) | `src/fyi_system/monitor.py` | `ingest-feed` | No | **Gap** |
| Archive capture | `src/fyi_system/archive_capture.py` | `capture` | Partial (`test_archive_capture.py`) | **Partial** |

## Contract Test Standards

All contract tests must:
1. Define input/output schemas explicitly
2. Validate response structure against schema
3. Test error responses (4xx, 5xx)
4. Test edge cases (empty responses, missing fields)
5. Be replayable with fixture data (no live network)

## Gaps Identified

1. **import-authorities** (`importers.py`): No contract tests for CSV parsing or URL-based import
2. **ingest-feed** (`monitor.py`): No contract tests for RSS feed parsing
3. **archive_capture** (`archive_capture.py`): Partial contract coverage; needs WARC/WACZ contract tests

## Recommendation

Add contract tests for import-authorities and ingest-feed first, then complete archive_capture contract coverage.
