# Deterministic simulation testing applicability

Reconciled on 2026-11-14. The original analysis predated PR #414 and contained
historical gaps that were not an accurate account of the present implementation.

## Existing deterministic evidence

| Surface | Evidence | Enforced behavior |
| --- | --- | --- |
| Feed ingestion | `tests/test_monitor.py::TestIngestFeedDST` | Four tests: repeatable results from the same simulated entries, 50-entry execution below one second, empty feed and unavailable-feed fallback |
| Authority import/discovery | `tests/test_importers.py` | Local CSV/rows, idempotent database effects and `httpx.MockTransport` for URL-based acquisition, with request/response assertions |
| Acquisition receipts | `tests/test_acquisition_receipts.py` and acquisition conformance fixtures | Replayable metadata/digest and tampering assertions; source track owns further schema and runtime parity |
| Archive capture | `tests/test_archive_capture.py` | Mocked capture, WARC/WACZ and payload-preservation assertions |
| CDX and replay | `tests/test_internet_archive_cdx.py`, `tests/test_internet_archive_replay.py` | Existing bounded fixture-based source suites; unchanged by this CI track |

The four existing feed DST tests were independently executed during close-out.
Representative importer, API, receipt, remote-MCP and archive contracts also ran:
80 selected tests passed before the CI implementation was published. See
`acceptance-evidence.md` for commands and later hosted evidence.

The four older monitor smoke tests initially attempted live HTTP. Safe red tests
blocked those attempts before connection. They now use a checked-in empty RSS
fixture through the real parser under a class-local network guard; their original
assertions and all DST/reconciliation classes are unchanged. All 12 monitor tests
pass, and the final representative contract batch passes 88 tests.

## CI applicability

Deterministic suites remain in the full Python Tests gate. The dedicated
property/Hypothesis, Rust workspace, source-boundary and contract lanes are
preserved. Removing redundant full-suite Scalene execution from each PR does not
remove ordinary tests: full profiling remains scheduled/manual, and PR profiling
runs all eight existing benchmarks with explicit per-benchmark limits.

External HTTP service behavior is not a deterministic test fixture. Full hosted
source acquisition, source cutover, crash durability and downstream archive
parity are owned by the active source-adapter track and are not claimed here.

Local SQLite, cryptography, rendering and local process-event transformations
continue to use their existing unit, property or integration suites; they do not
require a new network simulation harness. No new simulation framework or runtime
dependency is introduced by this close-out.
