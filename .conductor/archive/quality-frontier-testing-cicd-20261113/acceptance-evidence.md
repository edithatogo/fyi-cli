# Independent quality-frontier acceptance evidence

Review date: 2026-11-14. Track: `quality-frontier-testing-cicd-20261113`.
Status: **implementation acceptance verified; archived through the closing PR**.

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
focused regressions. Hosted verification is recorded below; release and publication jobs were not
executed as part of this review.

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

## Close-out decision

Both implementation slices are merged with exact-final-head gate evidence.
Hosted artifacts and pruning are independently verified below. The closing PR
records the archive transfer's own final checks, merge and branch pruning.

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
   The manual harness performance layer had the same zero-collection defect;
   its command now uses the same explicit selector, with an actual collection
   regression (zero tests/exit 5 before the fix; eight collected afterward).
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
7. After integrating the separately merged TUI baseline, the layered harness
   failed before running its first Rust test: rustup tried to recover a partially
   installed toolchain and rejected a conflicting `cargo-clippy` binary
   ([run 43555109525, job 132326089011](https://github.com/edithatogo/fyi-cli/actions/runs/43555109525/job/132326089011)).
   The origin of that partial installation is not established. The harness now
   uses a job-local Rustup home under `RUNNER_TEMP` and explicitly installs the
   repository's 1.99.1 toolchain and required components with the existing pinned
   Rust action. The unchanged harness command must pass on the new final head;
   the infrastructure failure is not waived or counted as success.

No new dependency, mutation operator, application behavior, coverage exclusion,
security exception or branch-protection change is introduced. Python mutation
remains 70%; cargo-mutants retains its stricter existing failure behavior; Rust
line coverage remains 90%. `pipefail` prevents artifact logging from masking any
of these gates.

### Verification

- Extended-workflow red baseline: 8 failed / 1 passed; manual-harness follow-up
  reproduced zero-test collection/exit 5; final green: 10 passed.
- Core-CI evidence red baseline: 10 failed / 3 passed; green: 13 passed.
- Monitor safe red: four network-guard failures; green: all 12 monitor tests pass.
- Harness setup regression baseline: **2 failed / 5 passed**; green: **7 passed**.
- Combined workflow/harness contracts: **160 passed**.
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

## Final acceptance and archival decision — 2026-11-14

All nine acceptance criteria for this existing CI-infrastructure track are met.
No additional track, dependency, application behavior, source-adapter claim,
coverage/mutation reduction, security exception or branch-protection change is
introduced. The original issue #280 remains closed; #282 remains outside scope.

| Criterion | Implementation and acceptance evidence | Decision |
| --- | --- | --- |
| Applicability and exclusions | Reconciled DST/MT/contract applicability documents; source-adapter and live-service boundaries remain explicit. | Met |
| Deterministic replay and bounds | Existing feed DST suite and fixture-based importer/archive/receipt suites; local RSS smoke fixture prevents unintended live reads. | Met |
| Mutation infrastructure and gates | Weekly/manual execution; Python 70%; strict Rust cargo-mutants exits; environment and failure-pipeline regression tests. | Met |
| API and cross-runtime contracts | Existing API/importer/receipt/archive/MCP contracts preserved; source-adapter parity completion is not claimed. | Met |
| Immutable actions | Every direct remote action/reusable workflow is commit-pinned; the hardening regression covers all workflow files. | Met |
| Dependency review | PR vulnerability review remains fail-closed at its existing high-severity threshold. | Met |
| Workflow lint | Local/no-filter actionlint fails on findings; exact final-head Workflow Lint gate passed. | Met |
| Bounded PR and expensive lanes | Full authoritative tests retained; bounded benchmark PR smoke; weekly/manual mutation and full profiling; retained ordinary-failure evidence. | Met |
| Permissions, concurrency and updates | Explicit permissions and concurrency; scoped write/OIDC privileges; existing Renovate/update infrastructure and mandatory security gates preserved. | Met |

### PR #437 exact-head verification

[PR #437](https://github.com/edithatogo/fyi-cli/pull/437) passed all 12 required
contexts and 14 additional quality/security
gates on `51e49d4b51d40dfbe4dcce3bca428e917719f202`. Its verified base was `d98c09ed98f0ee311713c24cef6abb28d98e8d3c`;
it merged as `78e62066cc823659593034cc9f23f69ab9a5f6d9`. Both remote and local implementation
branch pruning were verified. The complete identities, snapshots and artifact
inspections are retained in `ci-verification.json`.

| Workflow | Successful final-head run |
| --- | --- |
| Actionlint | [43557732042](https://github.com/edithatogo/fyi-cli/actions/runs/43557732042) |
| Dependency Review | [43557732064](https://github.com/edithatogo/fyi-cli/actions/runs/43557732064) |
| Profiling | [43557731993](https://github.com/edithatogo/fyi-cli/actions/runs/43557731993) |
| Test profile isolation | [43557732484](https://github.com/edithatogo/fyi-cli/actions/runs/43557732484) |
| Privacy and secrets | [43557732108](https://github.com/edithatogo/fyi-cli/actions/runs/43557732108) |
| Python Security and Fuzzing | [43557732039](https://github.com/edithatogo/fyi-cli/actions/runs/43557732039) |
| CodeQL | [43557731996](https://github.com/edithatogo/fyi-cli/actions/runs/43557731996) |
| Frontier Conductor and MSRV contracts | [43557732017](https://github.com/edithatogo/fyi-cli/actions/runs/43557732017) |
| CI | [43557732005](https://github.com/edithatogo/fyi-cli/actions/runs/43557732005) |

Inspected final-head evidence reports **1308 Python tests
passed** and **2 explicit opt-in live-smoke skips**;
skips are not counted as passing tests. All **8 benchmark
cases** passed. Rust line coverage is **92.58%**, above
the unchanged **90%** gate. Cargo audit/deny output and successful mandatory
security checks are retained; existing exceptions are unchanged.

Each artifact digest was verified and its embedded source/run provenance matched
the final head. The synthetic PR checkout SHA is recorded separately from the
source head. These are inspected artifacts, not merely successful upload steps.

| Evidence | Artifact | Verified digest | Expiry |
| --- | --- | --- | --- |
| Python test reports | [14099590309](https://github.com/edithatogo/fyi-cli/actions/runs/43557732005/artifacts/14099590309) | `sha256:a6bc7c3e12b95e062e80e84837d26824b4f5f9e1b3e78d233d527ba92dc37f74` | 2026-11-28T04:45:23Z |
| Rust coverage | [14099386223](https://github.com/edithatogo/fyi-cli/actions/runs/43557732005/artifacts/14099386223) | `sha256:7c8fe596a608d33262cb1cdca5b3f7487d87d688b019c8603aaf2b6edd7b22d2` | 2026-11-28T04:46:37Z |
| Security audit | [14099356004](https://github.com/edithatogo/fyi-cli/actions/runs/43557732005/artifacts/14099356004) | `sha256:e0edf8c9c34bc6eca39367d55a9709a377d08ad6a1044e08666f42581dc055aa` | 2026-11-28T04:49:30Z |
| Benchmark smoke | [14098574411](https://github.com/edithatogo/fyi-cli/actions/runs/43557731993/artifacts/14098574411) | `sha256:18cb4b2088e070cada0d516c71db5f94be36186e4ad04266f595221dfa644cb3` | 2026-11-28T04:41:52Z |

### Archive and remaining boundaries

The existing track is moved from `.conductor/tracks/quality-frontier-testing-cicd-20261113/`
to `.conductor/archive/quality-frontier-testing-cicd-20261113/`, and only its registry/index
entries are moved to completed status. PR #431's merged execution-order and
source-track records, subsequent unrelated baseline work, and all source-adapter
implementation remain untouched by this archive change.

The closing archive PR conversation is the authoritative record of that PR's
own final head, required checks, merge commit and branch pruning. Its future
merge hash cannot be embedded in the commit it identifies. Until that PR merges,
the archive transfer is proposed; the implementation acceptance above is verified.

No implementation blocker remains for this track. Full mutation campaigns and
full scheduled Scalene execution remain scheduled/manual; no full-campaign score
is inferred. Optional Python 3.15 diagnostics and the separately hosted AI
security lane remain distinct from mandatory success. Runner loss can prevent
final artifact upload, and the 14-day artifact retention is not permanent storage.
The committed digest/provenance/inspection summary remains after artifact expiry.
