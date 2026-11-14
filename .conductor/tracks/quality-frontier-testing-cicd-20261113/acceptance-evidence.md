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
source-boundary test is changed. PR #431 owns execution-order and registry edits;
the registry is deliberately untouched until reconciliation with its merged
version. The existing solo-maintainer-security, test-acceleration and
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

1. Repair extended mutation/profiling execution and retain useful artifacts on
   both success and failure; verify with focused executable regressions.
2. Run final-head hosted gates, record PR/run/merge identifiers and prune merged
   branches.
3. Reconcile all acceptance items and historical applicability statements, then
   archive after PR #431's registry changes are merged.
