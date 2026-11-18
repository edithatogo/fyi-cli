# Replay qualification — issue #312

Implementation PR: [#489](https://github.com/edithatogo/fyi-cli/pull/489).
Receiving baseline: `1f0ae6d0b00663d52ccb65904152ca4e933b3486`, including the
Rust replay engine and guarded transport from #481/#483.

## Changes and scope

Implementation checkpoint:
[`d7b577efe25491ae1e7ccac0255ebdcd081384fe`](https://github.com/edithatogo/fyi-cli/commit/d7b577efe25491ae1e7ccac0255ebdcd081384fe).

- Serialize and validate the next checkpoint against the reader's inclusive
  16 MiB limit before publishing its payload or replacing previous progress.
  Overflow preserves both an absent checkpoint and an existing resumable prefix.
  This bounds checkpoint publication; serialization still uses an in-memory buffer.
- Explicitly disable reqwest's implicit protocol-NACK retries in the
  Wayback-specific production builder. Each observed exchange remains one
  physical attempt, consistent with the existing zero-retry receipt contract.
- Rerun the pinned legacy Python implementation against the frozen replay
  corpus already exercised by Rust. Keep the two documented native improvements
  separate from the 30 shared replay cases.
- Preserve replay corpus/payload bytes under Git newline conversion and add
  replay, transport, evidence and CLI cases to the existing portability gate.
- Correct the legacy guide and document the current Rust invocation and recovery.

The test uses the existing locked `h2 0.4.20` dependency directly; no dependency
versions or production receipt schemas changed. Tests use synthetic inputs,
in-process responses and an HTTP/2 duplex memory stream. No live source request
or downstream operation is part of this increment.

The concurrent #310 work in #488 owns request-page and sync-pull evidence. This
slice changes the Wayback builder and replay engine. Receiving integration must
retain both additive Conductor plan sections and their distinct source changes.

## Reproduced failures before correction

Tests-first head:
[`7805d42b0ccf986a55a21b97cfdeae0a5103a8a4`](https://github.com/edithatogo/fyi-cli/commit/7805d42b0ccf986a55a21b97cfdeae0a5103a8a4).

| Regression | Exact hosted evidence | Observed failure |
| --- | --- | --- |
| Oversized checkpoint publication | [Linux](https://github.com/edithatogo/fyi-cli/actions/runs/44052174918/job/133932438997), [macOS](https://github.com/edithatogo/fyi-cli/actions/runs/44052174918/job/133932438868), [Windows](https://github.com/edithatogo/fyi-cli/actions/runs/44052174918/job/133932438532) | The original code published the oversized checkpoint; the new rejection assertion failed on every platform. |
| Hidden protocol-NACK retries | [Workspace tests](https://github.com/edithatogo/fyi-cli/actions/runs/44052174963/job/133932439053), [instrumented tests](https://github.com/edithatogo/fyi-cli/actions/runs/44052174963/job/133932439139) | The default-policy control retried; the production Wayback client made three attempts where one was required. |

The checkpoint regression exercises both initial and partial progress and then
resumes with ordinary metadata. The retry fixture obtains a genuinely remote
HTTP/2 reset over an in-memory connection, then injects it through a connector
that never calls DNS or opens a socket. Its default-client positive control
ensures that the regression reaches reqwest's real retry classifier.

Local checkout regressions also failed for six replay-fixture cases before the
attribute changes, then all nine acquisition/replay checkout cases passed.
The portability-contract additions first produced two expected failures and
then passed after the corresponding workflow changes.

## Verification and review

Local CPython 3.15.2 qualification:

```text
.venv/bin/python -m pytest -q tests/test_internet_archive_replay_conformance.py \
  tests/test_receipt_fixture_checkout.py tests/test_receipt_portability_workflow.py
82 passed: 67 conformance, 9 Git checkout, 6 portability contracts
```

Ruff check/format and BasedPyright passed on all three touched Python test files.
Rustfmt, `git diff --check`, `verify_frontier_conductor.py` and
`verify_cli_parity.py` passed. The checkout helper's narrow S603 annotation
documents a verified false positive: the resolved Git executable receives only
internally constructed test arguments, without a shell.

Local Rust compilation was repeatedly terminated with SIGKILL under memory
pressure before tests executed. Full local Python environment installation also
hit a missing libffi development header; focused tests used dependencies
constrained by the unchanged lockfile. Neither infrastructure failure is a
test pass. Hosted Python Tests at tests-first head passed **1,766 tests**, with
two skips, on the complete environment.

Independent source review found no blocking issue in both production fixes,
their regressions, the corpus harness or the portability changes. Reviewed core
blobs are `8649308660a83563ab282c7758fa9b83fa4ed379` (replay) and
`f2c9ad6658acac35d2b3250be779d96e9a0eb208` (Wayback transport).

Final executable acceptance is the exact receiving head recorded in
[PR #489's checks](https://github.com/edithatogo/fyi-cli/pull/489/checks).
The tests-first failures and static review are development evidence; receiving
integration requires all protected checks and the three-platform replay gate.

## Remaining programme gates and rollback

Issue #312 and the parent track remain open for separately authorized bounded
live smoke, released downstream integration and NZ shadow parity. WARC linkage
and all-command receipt coverage retain their existing owners. The finite corpus
does not establish arbitrary cross-runtime equivalence.

`archive_cutover_allowed=false`. Reverting this focused change requires no data
migration. Existing approvals, payloads, checkpoints and receipts are not rewritten.
