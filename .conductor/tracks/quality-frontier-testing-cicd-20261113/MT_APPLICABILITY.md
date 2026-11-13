# MT Applicability Analysis

Mutation Testing (MT) applicability for fyi-cli.
Generated during Conductor track `quality-frontier-testing-cicd-20261113` (issue #280).

## Existing Infrastructure

- **Python**: `mutation_test.py` - simple mutation testing script with boolean, comparison, and None mutations
- **Rust**: `cargo mutants` via `.github/workflows/mutation.yml`
- **CI**: `.github/workflows/mutation.yml` - scheduled weekly (Sunday 02:30), manual dispatch supported
- **Rust job**: `cargo mutants --workspace --all-features --timeout 60`
- **Python job**: `python mutation_test.py`

## Code Units Suitable for MT

These units contain pure logic, conditionals, comparisons, and boolean logic that benefit from mutation testing:

### Python

| Unit | Module | Suitable Mutations |
|------|--------|-------------------|
| Redaction/sanitization | `src/fyi_system/security.py` | boolean, comparison, None, string logic |
| URL/entry normalization | `src/fyi_system/discovery.py` | comparison, None, string parsing |
| Request/response handling | `src/fyi_system/fetch.py` | boolean, comparison, status code logic |
| Evidence delta | `src/fsi_system/evidence_delta.py` | comparison, None, delta logic |
| State normalization | `src/fyi_system/reporting.py` | comparison, None, state transitions |
| Credentials | `src/fyi_system/credentials.py` | boolean, comparison, validation |

### Rust

| Unit | Module | Suitable Mutations |
|------|--------|-------------------|
| Hash chain/provenance | `crates/fyi-core/src/provenance.rs` | boolean, comparison, arithmetic |
| Receipt validation | `crates/fyi-core/src/acquisition_receipts.rs` | boolean, comparison, digest logic |
| Provider contract | `crates/fyi-core/src/provider_contract.rs` | boolean, comparison, trait logic |
| Endorsed route | `crates/fyi-core/src/endorsed_route.rs` | boolean, comparison, allowlist logic |

## Code Units Excluded from MT

These units perform I/O, external calls, or are primarily glue code:

| Unit | Module | Reason for Exclusion |
|------|--------|---------------------|
| Database | `src/fyi_system/db.py` | SQLite I/O; mutations won't be caught by unit tests |
| Feed ingestion | `src/fyi_system/monitor.py` | Network I/O (RSS); use integration tests |
| CLI argument parsing | `src/fyi_system/cli.py` | Mostly argparse glue; test via CLI integration |
| Dashboard rendering | `src/fyi_system/dashboard.py` | HTML rendering; use snapshot tests |
| Rust DB | `crates/fyi-core/src/db.rs` | Database I/O; use integration tests |
| Rust sync | `crates/fyi-core/src/sync.rs` | Network orchestration; use integration tests |

## MT Score Thresholds

Recommended thresholds:
- **Python**: >= 70% mutation score for logic-heavy modules
- **Rust**: >= 60% mutation score for logic-heavy modules

## Gaps Identified

1. No mutation score thresholds enforced in CI (workflow runs but doesn't fail on low score)
2. No per-module mutation score reporting
3. Python `mutation_test.py` is a simple implementation; consider `mutmut` for more comprehensive mutation operators

## Recommendation

Add mutation score threshold enforcement to CI, starting with Python `mutation_test.py` and then Rust `cargo mutants`.
