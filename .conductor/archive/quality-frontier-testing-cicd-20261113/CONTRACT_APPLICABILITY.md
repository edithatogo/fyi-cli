# Contract testing applicability and preserved boundaries

Reconciled on 2026-11-14 for issue #280. Contract tests already existed when the
CI close-out review began; this track preserves them and adds executable workflow
contracts for the demonstrated CI defects.

## Existing contracts

| Boundary | Representative evidence | Interpretation |
| --- | --- | --- |
| API request/response/error shapes | `tests/test_api_contract.py`, `tests/test_api_contract_inventory.py` | Existing schema assertions and boundary inventory; any failed assertion fails the test gate |
| Authority input and acquisition | `tests/test_importers.py` | CSV parsing, row validation, URL mock transport, idempotent database effects and receipt metadata |
| Acquisition receipt wire/integrity | Python receipt tests, `crates/fyi-core/tests/acquisition_receipts.rs`, `conformance/acquisition/` | Existing golden, digest and negative tests; not a claim of completed source-adapter parity |
| Archive payload preservation | `tests/test_archive_capture.py` | Existing mocked original-payload and WARC/WACZ checks; downstream parity remains separately owned |
| Remote MCP | `tests/test_remote_mcp_contract.py`, `scripts/verify_remote_mcp_contract.py` | Existing compatibility/guardrail contract |
| Layered harness | `tests/test_harness_contract.py`, `scripts/verify_test_harness.py` | Inventory validation runs before executable safe layers |

The historical statements that import-authority contracts and mutation thresholds
were absent were stale. The old plan also conflated contract assertions with the
mutation score: contract tests must pass their assertions; mutation analysis has
its separate unchanged threshold/exit policy.

## Added CI contracts

- `tests/test_workflow_hardening.py`: every direct remote action/reusable workflow
  is SHA-pinned; explicit permissions and concurrency; scoped writes, credential
  handling, lint failure behavior and dependency-review threshold.
- `tests/test_ci_gate_contract.py`: mandatory commands, time bounds, scoped OIDC,
  90% Rust coverage and the retained single harness invocation.
- `tests/test_extended_workflow_evidence.py`: scheduled/manual extended lanes,
  actual benchmark collection, mutation environment/thresholds, retained evidence
  and failing command exit preservation through log pipelines.
- `tests/test_ci_artifact_evidence.py`: Python, security and coverage provenance,
  reports and failure artifacts; executable failures prove logging cannot mask
  either test or coverage-threshold failure.

These contracts use existing pytest/PyYAML and shell infrastructure. They do not
introduce another Conductor track or governance system.

## Scope exclusions

Full JSON Schema/canonicalization parity, all source command receipt coverage,
attachment/redirect provenance, Wayback/CDX implementation and `fyi-archive`
cutover remain under `source-adapter-boundary-20260812` and issues #309/#310/#312.
This review neither edits that implementation nor marks its acceptance complete.
Live authority/API behavior is outside deterministic contract testing.
