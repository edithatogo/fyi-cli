# DST Applicability Analysis

Deterministic Simulation Testing (DST) applicability for fyi-cli network commands.
Generated during Conductor track `quality-frontier-testing-cicd-20261113` (issue #280).

## Code Paths That Benefit from DST

These code paths make outbound HTTP requests and should have deterministic,
replayable simulation tests with bounded runtime and thresholds:

| Code Path | Module | CLI Command | Existing DST Tests | Status |
|-----------|--------|-------------|-------------------|--------|
| Fetch request page | `src/fyi_system/fetch.py` | `fetch-request-page` | Yes (`test_acquisition_receipts.py`) | Covered |
| Discover bodies | `src/fyi_system/discovery.py` | `discover-bodies` | Yes (`test_discovery.py`) | Covered |
| Discover feed/search | `src/fyi_system/discovery.py` | `discover` | Yes (`test_discovery.py`) | Covered |
| Backfill IDs (numeric probing) | `src/fyi_system/discovery.py` | `discover --backfill-ids` | Yes (`test_discovery.py`) | Covered |
| Internet Archive CDX | `src/fyi_system/internet_archive_cdx.py` | `internet-archive-cdx` | Yes (`test_internet_archive_cdx.py`) | Covered |
| Internet Archive replay | `src/fyi_system/internet_archive_replay.py` | `internet-archive-replay` | Yes (`test_internet_archive_replay.py`) | Covered |
| Historical index | `src/fyi_system/historical_index.py` | `historical-index` | Yes (`test_historical_index.py`) | Covered |
| Ingest feed (RSS) | `src/fyi_system/monitor.py` | `ingest-feed` | Partial | **Gap** - needs DST |
| Archive capture | `src/fyi_system/archive_capture.py` | `capture` | Partial (`test_archive_capture.py`) | **Gap** - needs bounded runtime |
| Import authorities (CSV/URL) | `src/fyi_system/importers.py` | `import-authorities` | Partial | **Gap** - needs DST |

## Code Paths Excluded from DST

These code paths perform local-only operations (no network I/O) and do not
benefit from deterministic simulation testing:

| Code Path | Module | Reason for Exclusion |
|-----------|--------|---------------------|
| Database operations | `src/fyi_system/db.py` | Local SQLite; use integration tests instead |
| Security/encryption | `src/fyi_system/security.py` | Local crypto; use unit/property tests |
| Agent runtime | `src/fyi_system/agent_runtime.py` | Local logic; use unit tests |
| Process events | `src/fyi_system/process_events.py` | Local event processing; use unit tests |
| Archive health | `src/fyi_system/archive_health.py` | Local file system reads; use integration tests |
| Dashboard | `src/fyi_system/dashboard.py` | Local rendering; use unit tests |
| Provenance | `src/fyi_system/provenance.rs` | Local hash chain; use Rust unit tests |
| Sync client | `crates/fyi-core/src/sync.rs` | Local orchestration; use Rust unit tests |

## DST Test Standards

All DST tests must:
1. Use `httpx.MockTransport` or equivalent for HTTP simulation
2. Use fixture files for replayable responses (not inline data)
3. Have bounded runtime (no wall-clock dependencies)
4. Be deterministic (same input → same output every run)
5. Include threshold assertions (status codes, byte counts, digest values)

## Gaps Identified

1. **ingest-feed** (`monitor.py`): No deterministic simulation test for RSS feed ingestion with bounded runtime.
2. **archive_capture** (`archive_capture.py`): Has mock tests but no bounded runtime threshold enforcement.
3. **import-authorities** (`importers.py`): No DST test for URL-based authority import.

## Recommendation

Prioritize DST test additions in order: ingest-feed → archive-capture bounded runtime → import-authorities.
