# Mutation testing applicability and enforced gates

Reconciled on 2026-11-14 for existing issue #280. This records the implemented
infrastructure and its boundaries; it does not claim an unmeasured mutation score.

## Implemented execution

| Lane | Command and gate | Execution and evidence |
| --- | --- | --- |
| Python | `uv run python mutation_test.py --threshold 70`; below 70%, a failing baseline or no applicable mutations fails | Weekly Sunday 02:30 UTC and manual dispatch; 45-minute job bound; per-file scores and failed pytest diagnostics in retained logs |
| Rust | `cargo mutants --workspace --all-features --timeout 60`; retain cargo-mutants' strict nonzero exit for survivors/timeouts | Same weekly/manual workflow and 45-minute job bound; full `mutants.out` and execution logs retained |

The earlier proposed 60% Rust threshold was not the implemented contract. Do not
replace the existing stricter exit behavior with that proposal. Python's existing
70% threshold, mutation operators and test exclusions are unchanged by this review.
The runner now uses its active Python interpreter, preserving the installed
virtual environment for subprocess tests.

Both jobs retain source/run/attempt/runtime provenance and logs for 14 days with
`if: always()`. Logging uses `pipefail`; a recorded failing test cannot become a
successful gate merely because `tee` succeeds. Ordinary command failures retain
partial evidence. Runner loss or termination of the entire job can prevent final
artifact upload; GitHub's job logs remain separate evidence where available.

## Applicability and boundaries

Mutation analysis is useful for conditional and transformation logic: security
redaction, normalization, evidence integrity, provider policy and local state
transitions. The existing commands define the actual analyzed scope; the Python
runner scans `src/fyi_system`, and Rust uses the workspace/all-features selection.
Earlier prose tables describing module exclusions were recommendations, not
exclusions enforced by those commands.

The Python runner retains its existing exclusions for scheduler internals, CLI
and workflow E2E, API-contract tests and its existing slow-test keyword filter.
Those suites remain in the authoritative ordinary CI test gates. Their exclusion
from this repeated mutation workload is not a general test exemption.

Receipt/schema/CDX/Wayback implementation and full downstream parity remain with
`source-adapter-boundary-20260812`. This track preserves those tests and does not
change application logic to improve a mutation score. The older
`mutation-testing-execution` and archived `research-grade-quality` tracks are
historical context, not new competing implementations.

## Verification

`tests/test_extended_workflow_evidence.py` verifies scheduling, bounds, thresholds,
interpreter selection, retention and real failing shell pipelines. The
acceptance-evidence record distinguishes these focused tests and hosted PR checks
from the expensive full campaigns, which are scheduled/manual rather than PR
requirements. No full-campaign score is inferred from a workflow-contract pass.
