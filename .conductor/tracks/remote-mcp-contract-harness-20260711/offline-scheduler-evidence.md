# Offline scheduler fixture repair for #169

## Finding and boundary

On 14 November 2026 automatic approval review stopped an unguarded full Python
run when legacy scheduler tests attempted external feed retrieval. That run is
not a pass and was not retried unchanged. Inspection identified three test
modules that called the real feed parser with external URLs; broad exception
handling and best-effort ingestion concealed the calls from their assertions.

This increment replaces only feed acquisition in those modules with synthetic
empty/unavailable responses. The real monitor, scheduler, SQLite database,
report and handover code still execute. There is no scheduler/source change,
shared fixture configuration, runtime/lock change or workflow edit. The active
owner PR file lists were checked for overlap before this isolated test repair.

## Test-first evidence

The isolated regression probe patches Python DNS resolution and socket
connection/send operations before loading the three legacy modules. Its child
has a minimal environment, disables third-party pytest plugin autoload and
counts attempted network access even when a test catches the raised exception.
Any attempt causes a nonzero exit after pytest finishes.

- **RED:** the 36 legacy tests reported passes, but the probe blocked **28
  network attempts** and exited 86, failing the outer assertion.
- **GREEN:** the same probe passes with the synthetic feed fixture. All **39
  scoped tests pass**, including two added real local-cycle assertions for
  successful-empty and unavailable feeds. Both assert actual output files,
  zero item counts and the persisted successful run record.
- The new helper and regression module pass Ruff checks and formatting; diff
  whitespace validation passes. Existing style debt in the three legacy test
  modules is not rewritten by this four-line-per-module fixture registration.
  Comparison against the unchanged baseline found no added Ruff diagnostics in
  those legacy modules.

Published RED checkpoint `5f45cec0719acd7315ff56743e1dee723b0ddd65` and
implementation `d1295c8d14e572707838641a82d309506feb9ab0` preserve the test-first
history. The implementation tree `4be43ac7cbb72fbc7ee559fa9fcabae5ce3a5731`
matches local implementation `c1314d013523c428adf9cd747ef8d5cd23f2396f` exactly.

The targeted command, using the unchanged locked development environment, is:

```sh
python -m pytest -q tests/test_scheduler-IASN2001657.py \
  tests/test_scheduler.py tests/test_scheduler_internals.py \
  tests/test_scheduler_offline_contract.py
python -m ruff check tests/scheduler_feed_fixture.py tests/test_scheduler_offline_contract.py
python -m ruff format --check tests/scheduler_feed_fixture.py tests/test_scheduler_offline_contract.py
```

## Full local verification and its limits

A separate, materially safer full run cleared inherited HTTP/HTTPS/ALL/FTP proxy
settings and installed a loopback-only Python DNS/socket guard before pytest.
The guard rejects external destinations before resolution/connection and fails
the overall run if an attempted access is swallowed by a test. This completed
with **1,515 passed, two skipped** in **55.63 seconds**, with **zero external
Python DNS/socket attempts**. The original rejected unguarded run is excluded.

Environment: Linux x86_64, Python **3.12.14**, MCP SDK **1.30.0**, development
dependencies from unchanged `uv.lock`, baseline master `76de6a2`. The existing
MCP executable tests used an explicitly supplied unmodified binary from that
baseline. This Python guard is not an operating-system or native-binary network
sandbox, and this result does not qualify the separately owned Python 3.15.1 or
Windows/macOS matrices. The committed focused subprocess probe is the enduring
regression sensor for the three identified modules.

## Integration and rollback

Focused PR [#458](https://github.com/edithatogo/fyi-cli/pull/458) contains this
repair. The sole integration coordinator owns the final base/head reconciliation,
required checks/review, merge and pruning. This repair precedes the actual
upstream MCP harness increment so its verification does not repeat the rejected
unguarded scheduler behavior. It changes neither a gate nor an exclusion.

Rolling back restores the hidden-network behavior and must retain a blocking
offline-test finding; it is not a reason to suppress the probe. Issues #169 and
#173 and their existing Conductor track remain open for the remaining source,
platform, migration and broader verification criteria.
