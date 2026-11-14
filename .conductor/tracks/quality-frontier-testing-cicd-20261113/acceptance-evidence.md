# Independent quality-frontier acceptance evidence

Review date: 2026-11-14. Track: `quality-frontier-testing-cicd-20261113`.
Status: **in progress; final-head hosted verification and archive pending**.

## Baseline and scope

The reviewed baseline is `062c2647e9dc5300fee249cdd7b81a3d164807bc`
([PR #430](https://github.com/edithatogo/fyi-cli/pull/430)).
[PR #414](https://github.com/edithatogo/fyi-cli/pull/414) merged the initial quality
implementation as `ff34702113d07ccaf3bce9f13303c96376ea550c`.
[Issue #280](https://github.com/edithatogo/fyi-cli/issues/280) is already closed;
its comment also records the earlier #320/#321 security work.

The repository contained DST, contract, mutation, dependency-review, security,
privacy and coverage gates before this review. The completed metadata was not
sufficient acceptance evidence: Phase 5 remained partly unchecked and the audit
found real workflow defects. This review reuses the existing track.

Source acquisition, receipt contracts, CDX/Wayback and downstream archive parity
remain owned by `source-adapter-boundary-20260812`. No application source or
receipt/CDX/Wayback test is changed. PR #431 merged its execution-order and registry
edits as `a816dcd425a7b6277761a18faeae1685202eefde`; both implementation slices were
reconciled with that merged baseline. The registry remains unchanged by these
implementation slices. The existing solo-maintainer-security, test-acceleration and
frontier-rust-convergence tracks retain their separate scope.

## Workflow-hardening slice

Branch: `fix/quality-frontier-workflow-hardening`.

- Pin 14 remaining floating action references across publication, documentation,
  maintenance and release workflows to the same resolved versions.
- Limit GitHub write permissions to jobs that require them; retain CodeQL,
  Codecov OIDC, release attestation, Pages and package-publication permissions.
- Add concurrency and explicit job time bounds. Cancel superseded PR execution
  only; publication and scheduled runs are not interrupted by the new controls.
- Configure actionlint with local reporting, no filtering and failure on every
  finding. Fix four exposed SC2086 shell expansions by quoting paths/redirects.
- Remove only the redundant harness inventory invocation. The retained `--run`
  command validates inventory before running every configured safe test layer.
- Preserve the exact 90% Rust coverage gate, mandatory commands, security
  severity thresholds and dependency-review functionality.

### Local evidence

The CI-gate regression baseline was **4 failed / 1 passed**, and the general
workflow-hardening baseline was **42 failed / 37 passed**, before implementation.

After implementation:

```text
uv run pytest -q tests/test_workflow_hardening.py tests/test_ci_gate_contract.py
  tests/test_action_runtime_pins.py tests/test_python_security_fuzzing_workflow.py
  tests/test_receipt_portability_workflow.py tests/test_dependency_automation.py
  tests/test_frontier_conductor.py tests/test_harness_contract.py
111 passed

uv run ruff check tests/test_ci_gate_contract.py tests/test_workflow_hardening.py
PASS
uv run ruff format --check tests/test_ci_gate_contract.py tests/test_workflow_hardening.py
PASS
python scripts/verify_frontier_conductor.py
PASS
git diff --check
PASS
```

The pinned reviewdog action uses kjanat/actionlint 1.17.1. That version and
ShellCheck 0.11.0 were verified locally against all 24 workflows; exit status 0.
Downloaded binary SHA-256 checksums:

- actionlint: `6bfe0f49f0e108b9b23732baf7de63bfa4d89174fa40f9cb98feb9676df3318f`
- ShellCheck: `8c3be12b05d5c177a04c29e3c78ce89ac86f1595681cab149b65b97c4e227198`

Independent read-only review found no must-fix defect and separately ran 101
focused regressions. Hosted verification is still required; this is not a claim
that release or publication jobs were executed.

### Hosted verification and merge

[PR #433](https://github.com/edithatogo/fyi-cli/pull/433) merged as
`797e76cf120988372f4df1f2efb9c9e164c69050`. Its exact final head was
`e9765d84bed06cf1cecaea25db5e3ca21666904c`, including merged PR #431.
All 12 live protected-branch required checks and all 14 relevant additional
quality/security gates passed before merge. The merge API was given that exact
expected head; no check was waived. The remote branch's deletion was verified
through the Git refs API, and its local branch was pruned.

| Workflow | Successful final-head run |
| --- | --- |
| CI | [43552345893](https://github.com/edithatogo/fyi-cli/actions/runs/43552345893) |
| CodeQL | [43552345779](https://github.com/edithatogo/fyi-cli/actions/runs/43552345779) |
| Workflow lint | [43552345772](https://github.com/edithatogo/fyi-cli/actions/runs/43552345772) |
| Dependency review | [43552345815](https://github.com/edithatogo/fyi-cli/actions/runs/43552345815) |
| Python security/fuzzing | [43552345807](https://github.com/edithatogo/fyi-cli/actions/runs/43552345807) |
| Privacy/secrets | [43552345774](https://github.com/edithatogo/fyi-cli/actions/runs/43552345774) |
| Conductor/MSRV | [43552345792](https://github.com/edithatogo/fyi-cli/actions/runs/43552345792) |
| Keyring isolation | [43552345780](https://github.com/edithatogo/fyi-cli/actions/runs/43552345780) |
| Profiling | [43552345821](https://github.com/edithatogo/fyi-cli/actions/runs/43552345821) |

`ci-verification.json` retains the required app/context identities, complete
check snapshot, run identifiers, final head, merge commit and optional-lane
distinctions. Optional queued/pending self-hosted AI security was not counted as
a passing check; the Python 3.15 diagnostics do not establish tool compatibility
merely because their reporting workflow completes successfully.

### Immutable reference provenance

Resolved through the upstream GitHub Git references API on 2026-11-14. Annotated
tags were dereferenced to commits. Existing versions were preserved.

| Action/ref | Commit |
| --- | --- |
| `actions/checkout@v6` | `d23441a48e516b6c34aea4fa41551a30e30af803` |
| `actions/checkout@v5` | `fbc6f3992d24b796d5a048ff273f7fcc4a7b6c09` |
| `docker/setup-qemu-action@v4` | `99012661954931238ded8c8b007157a8430204e1` |
| `docker/setup-buildx-action@v4` | `f87e5991a6d7451dcb8d9637bfbc97413f497069` |
| `docker/build-push-action@v7` | `c3c9e263c25d99ce0380d002d59b67737d91b0dc` |
| `actions/upload-pages-artifact@v5` | `fc324d3547104276b827a68afc52ff2a11cc49c9` |
| `actions/deploy-pages@v5` | `368f82528645a54fb793d4d04e342629a3f51346` |
| `googleapis/release-please-action@v4` | `5c625bfb5d1ff62eadeeb3772007f7f66fdcf071` |
| `conda-incubator/setup-miniconda@v3` | `fc2d68f6413eb2d87b895e92f8584b5b94a10167` |
| `actions/github-script@v8` | `ed597411d8f924073f98dfc5c65a23a2325f34cd` |
| `dtolnay/rust-toolchain@stable` | `b725eda525362e7cf34af9a7243cac1e3f01a7ec` |
| `actions/attest-build-provenance@v2` | `e8998f949152b193b063cb0ec769d69d929409be` |

## Merge verification policy

Live branch metadata confirms these 12 required contexts: Rustfmt, Clippy Lints,
Workspace Tests, Package Smoke, Security Audit, Code Coverage, Layered Harness
Contract, Python Tests, CodeQL, Analyze (python), Analyze (javascript-typescript),
Packaging Assets. Each must report success for the exact final PR head. Relevant
additional security, workflow-contract and dependency-review checks must also
pass. Queued, skipped, cancelled or stale checks are not success evidence.

The optional Python 3.15 compatibility sensors and self-hosted Offline AI
security lane are reported separately. The latter requires an
`ai-security-offline` runner and was queued at baseline; the live protected
branch required-context list does not include it. Its security implementation
and permissions are preserved. No branch protection is modified.

## Remaining before close-out

1. Run final-head hosted gates, record PR/run/merge identifiers and prune merged
   branches.
2. Confirm hosted artifact retention, reconcile all acceptance items and archive
   against the merged registry. Historical applicability documents are now
   reconciled with the implemented gates and source-track exclusions.

## Extended-lane and retained-evidence slice

Branch: `fix/quality-frontier-evidence`.

### Demonstrated defects and repairs

1. Python mutation invoked system Python after `uv sync` installed dependencies
   into `.venv`. The workflow now uses `uv run python`; its pytest subprocess uses
   `sys.executable`, and failed-test stdout/stderr with short tracebacks is kept.
2. Mutation jobs uploaded no reports. They now retain setup/runtime metadata,
   stdout/stderr, per-file Python results and Rust `mutants.out` on ordinary
   success/failure paths for 14 days.
3. Profiling reran the full suite under Scalene on every PR, yet its benchmark
   invocation did not collect any `benchmark_*` method. PRs now run all eight
   existing benchmark functions with explicit short bounds; full Scalene and
   ordinary benchmark measurement are weekly/manual.
4. Profiling uploads were success-only. The new always-run upload includes
   metadata, setup/runtime logs, benchmark JSON and JUnit XML, including partial
   evidence on failure.
5. Python Tests, Security Audit and Code Coverage lacked retained diagnostic
   artifacts. Each now initializes provenance before setup and keeps reports,
   installation/test/security logs and LCOV as applicable. The event SHA, source
   PR head and actual checkout SHA are distinguished, with run/attempt/URL and
   tool-version evidence.
6. Four legacy monitor smoke tests made live reads despite being part of ordinary
   CI. Their last change was in the original quality PR #414; active source PR
   #432 does not modify this file. A class-scoped network guard first reproduced
   all four attempts without making a request. The real parser now receives a
   local empty-RSS fixture; the original four test bodies and full DST and
   reconciliation classes remain AST-identical. Populated-feed behavior remains
   covered by the existing unchanged DST tests.

No new dependency, mutation operator, application behavior, coverage exclusion,
security exception or branch-protection change is introduced. Python mutation
remains 70%; cargo-mutants retains its stricter existing failure behavior; Rust
line coverage remains 90%. `pipefail` prevents artifact logging from masking any
of these gates.

### Verification

- Extended-workflow red baseline: 8 failed / 1 passed; green: 9 passed.
- Core-CI evidence red baseline: 10 failed / 3 passed; green: 13 passed.
- Monitor safe red: four network-guard failures; green: all 12 monitor tests pass.
- Combined workflow/harness contracts: **157 passed**.
- Existing monitor/DST/importer/API/receipt/MCP/archive contracts: **88 passed**.
- Actual PR benchmark workload: **8 passed**, with JSON and JUnit outputs;
  independently reproduced in 1.52 seconds.
- Independent final evidence/monitor review: **142 passed**, no must-fix finding.
- Ruff/formatting for the new workflow-contract files, all-workflow actionlint
  1.17.1/ShellCheck 0.11.0 and `git diff --check`: passed.

The dynamic regressions run the actual checked-in shell commands against failing
stub tools. They prove nonzero test/security/mutation exits and a below-threshold
coverage report still fail after successful logging. They do not pretend to be
full Rust or mutation executions. Required hosted runs provide separate evidence.

### Execution boundaries

Full mutation campaigns and scheduled full-suite Scalene runs are expensive
scheduled/manual lanes; no full score or profile is claimed from a contract-test
pass. Existing bounded Atheris PR/extended lanes and their failure corpus/log
retention are unchanged. CodeQL's security-analysis output and the shared privacy
scanner remain independent security evidence. A whole-run termination can prevent
an `always()` upload, so retained job logs and ordinary failure-path artifacts
are not described as crash-proof storage.
