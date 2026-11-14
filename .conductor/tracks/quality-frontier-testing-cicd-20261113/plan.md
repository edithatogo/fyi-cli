# Plan

## Phase 1: Applicability and exclusions

- [x] Confirm DST applicability: which code paths benefit from deterministic simulation (document exclusions). [checkpoint: b250b49]
- [x] Confirm MT applicability: which units are suitable for mutation testing (document exclusions). [checkpoint: 329e6b2]
- [x] Confirm contract testing applicability: which API boundaries need contract tests. [checkpoint: 8842043]
- [x] Document all exclusions in applicability documents (DST, MT, Contract). [checkpoint: 8842043]

## Phase 2: Deterministic simulation testing (DST)

- [x] Add deterministic, replayable test suites with bounded runtime. [checkpoint: 8cdb32e]
- [x] Add thresholds for DST pass/fail criteria. (Bounded runtime <1s enforced in DST tests)
- [x] Integrate DST into CI as a fast lane. (DST tests use mock transport; run with regular pytest suite)

## Phase 3: Mutation testing (MT)

- [x] Add mutation testing infrastructure (cargo-mutants for Rust, mutation_test.py for Python). [checkpoint: dd476fd]
- [x] Add mutation score thresholds (70% minimum). [checkpoint: dd476fd]
- [x] Integrate MT into CI as an expensive/scheduled lane (weekly + workflow_dispatch). [checkpoint: dd476fd]

## Phase 4: Contract testing

- [x] Add contract tests for API boundaries (Python/Rust cross-runtime parity). [Existing: test_acquisition_receipts.py, test_importers.py, test_api_contract.py, crates/fyi-core/tests/acquisition_receipts.rs with golden fixture conformance/acquisition/receipt-v1-python.json]
- [x] Add contract test thresholds. (Thresholds enforced via mutation_test.py --threshold 70 and CI workflow)

## Phase 5: CI hardening

- [x] Pin all GitHub Actions to full commit SHAs (pinned_actions). Historical checkpoint 253428b did not cover all workflows; the 2026-11-14 review pins the 14 remaining floating references and validates all 24 workflows.
- [x] Add dependency_review workflow for PRs. [checkpoint: 253428b]
- [x] Enforce actionlint findings as failures with a read-only local reporter; historical checkpoint 253428b installed the action but left its fail-open defaults. Matching actionlint 1.17.1 and ShellCheck 0.11.0 pass locally after repairing four unquoted shell expansions.
- [x] Add least-privilege, concurrency, immutable actions. All 24 workflows now have explicit permissions/concurrency and full remote-action pins, with regression contracts; required hosted acceptance is recorded in Phase 6.
- [x] Verify expensive lanes and bounded PR checks; repair virtualenv mutation execution and benchmark collection, retain full authoritative tests. Weekly/manual mutation and full profiling remain; all eight benchmarks run in the bounded PR smoke.
- [x] Retain CI evidence (artifacts, logs and source/run provenance), including failure paths. Mutation, profiling, Python Tests, Security Audit and Code Coverage upload for 14 days; executable failing-pipeline tests preserve gate exit codes.
- [x] Remove the four unintended live reads in the original quality track's monitor smoke tests. The real RSS parser uses checked-in fixture bytes behind a network guard; all original test bodies and DST/reconciliation classes remain AST-identical.

## Phase 6: Verification and closure

- [ ] Independently verify the final commits with targeted local tests and all required hosted checks. The historical 55 Python / 13 Rust test counts do not establish current acceptance.
- [ ] Record implementation PRs, exact final heads, CI run IDs, merge commits, risks and exclusions in acceptance evidence.
- [ ] Reconcile metadata, plan and registry against merged PR #431, then archive through the established `.conductor/archive/` procedure.
- [x] Verify issue #280 is already closed (2026-11-13); preserve its state and reconcile its existing #320/#321 evidence with this track's #414 implementation and the current review.

## Independent close-out review — 2026-11-14

Baseline: `062c2647e9dc5300fee249cdd7b81a3d164807bc` (merged PR #430).
PR #431 initially owned the open registry/execution-order updates. It merged as
`a816dcd425a7b6277761a18faeae1685202eefde` during verification; the implementation
was reconciled with that merged baseline without changing its files. Source acquisition,
receipts, Wayback/CDX, downstream parity and issues #309/#310/#312 remain with
`source-adapter-boundary-20260812`.

Implementation sequence under this existing track:

1. Focused workflow permissions, immutable references, concurrency, time bounds
   and fail-closed linting, with regression contracts.
2. Extended-lane execution and retained provenance/failure evidence, preserving
   Python's 70% mutation threshold, Rust's strict cargo-mutants exit semantics,
   the 90% Rust coverage gate and all source-boundary tests.
3. Independent final-head verification, merge/branch reconciliation and archive.

The standalone harness-inventory step is redundant: `verify_test_harness.py --run`
always validates the inventory before executing its layers. Remove only that
duplicate validation; keep every test layer and mandatory gate.

Local verification after combining both slices: 157 workflow/harness regression
tests, 88 existing monitor/DST/API/receipt/archive contract tests and the actual
eight-benchmark PR workload passed. Same-version actionlint/ShellCheck passed all
24 workflows. Independent review of the final evidence and monitor fixture
passed 142 targeted tests with no must-fix finding. Full mutation campaigns and
scheduled full-suite Scalene execution are not inferred from these results.

## Task Order

Work sequentially: Phase 1 → Phase 2 → Phase 3 → Phase 4 → Phase 5 → Phase 6.
