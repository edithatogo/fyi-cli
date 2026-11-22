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

PR #488 added request-page and sync-pull evidence under #310. PR #489 changed
the Wayback builder and replay engine. Both increments are present on
`master` and linked in the track plan.

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

Final PR head `e1e0a6ade6f23e04115dbd890e4a936f903964aa` passed hosted CI and
receipt portability on Linux, macOS and Windows. PR #489 merged into `master`
as `428ad00ee9e094fb09a59c9295eadf0bd33f5e2d`.
[Final CI](https://github.com/edithatogo/fyi-cli/actions/runs/44389636962) · [Portability run](https://github.com/edithatogo/fyi-cli/actions/runs/44389636938) · [PR #489](https://github.com/edithatogo/fyi-cli/pull/489)

## Offline Wayback archive handoff

On 22 November 2026, one synthetic gzip fixture ran with network access
denied. The in-process test transport supplied a single response, yielding a
73-byte encoded HTTP entity and a 50-byte decoded payload. The unmodified
Rust CLI verified the saved receipt, result and responses and resumed the
completed checkpoint with zero requests. The native `fyi-archive` builder
and verifier accepted one snapshot with one synthetic case and six replay
artifacts in its checksummed provenance.
[Pinned fixture](https://github.com/edithatogo/fyi-cli/tree/c94f3ed9159773b9dc77ee9207ed11fd2767b4f1/tests/fixtures/internet_archive_replay)

A fixture driver passed the saved body to the Rust WARC/WACZ API. The writer
reconstructed HTTP framing from the fixture metadata. The second snapshot's
checksummed provenance carried the WACZ and original receipt bytes and
linked the WACZ digest, WARC record ID and compressed range to receipt
response index 0. The CDXJ entry selected the WARC response at compressed
offset 0 with length 544 bytes. Reading its encoded entity returned the same
73 bytes byte for byte. The pinned CLI verified the receipt against the
extracted body and rejected a same-length byte change. The linkage check
rejected an incorrect WARC record ID. The package manifest, schema and
store indexes also verified.
[Rust WARC/WACZ writer and verifier](https://github.com/edithatogo/fyi-cli/blob/c94f3ed9159773b9dc77ee9207ed11fd2767b4f1/crates/fyi-core/src/archive_format.rs) · [Archive package builder and verifier](https://github.com/edithatogo/fyi-archive/blob/0691a9aabddc5b83b2f656321cb9ae2a879fc1cf/src/fyi_archive/archive_package.py)

### Evidence identifiers

- Rust `fyi-cli` 0.1.2 was built at `c94f3ed9159773b9dc77ee9207ed11fd2767b4f1`
  under Rust 1.100.0 on macOS arm64. Executable SHA-256
  `8c9eafb9968c8347c0dd4c1514e654e827ba15cfd230ba1b60f1546f485d24da`.
- `fyi-archive` 0.20.2 ran at `0691a9aabddc5b83b2f656321cb9ae2a879fc1cf`
  under CPython 3.15.2. Warcio 1.8.1 provided the independent WARC readback.
- Both snapshots bind replay receipt identity
  `d80db593d865f94bfd6351c603152437c508ad8bd42c67fccd5fa3f727dd9480`
  to the encoded body SHA-256
  `ddf6ca0e623a8d84c8e6828c9eb3f0a9239c9dd9d25676c5a3e271817f064f20`.
  The receipt file SHA-256 is
  `44c825439039f7e0e95a42f3786cdb2c6191f300646f9e6c2be3a3ff52ca4282`.
- The WACZ SHA-256 is
  `21d607c843099c8b96eeac68494417466d30fd838a0246e8bfe06396b3e0043a`.
  It contains WARC record `<urn:uuid:7ced2661-5068-4998-ac76-ef80d6b5aef8>`.
- The replay snapshot package ID is
  `907084dea815d339b6e566cc9473b7953833d81ea962173338f345565ab3a30c`.
  Manifest SHA-256
  `467c74a74100c98b48de01742071cf7b33ac8ed8697b236865df74ed2eac9200`.
  Retained local verification JSON SHA-256
  `53c98d5b51f813f43357a15f5c10849fe281f88862a60f3d3730b3cea6be406d`.
- The linked WARC/WACZ snapshot package ID is
  `8017b28c4eff9c4d1e50a071e6d84d4d461ad4d19c7acaa8d4306242b7bb7e1c`.
  Manifest SHA-256
  `13b19741d79081c8280267b4610857c982fb1d4f5047fd8fc8efedfff6a685c4`.
  Retained local verification JSON SHA-256
  `b3193af6a0b3a766a927776a137354478dfbf1172f592dfdaee8018112a77629`.

## Remaining programme gates and rollback

At the observed `fyi-archive` head, seed capture still invoked
`python -m fyi_system.cli capture`. #309, #310, #312 and the downstream NZ
shadow-cutover issue remain open. See the plan for production receipt and
WARC/WACZ integration, consumer release pinning, parity, authorized smoke and
rollback gates. [Pinned seed entry point](https://github.com/edithatogo/fyi-archive/blob/0691a9aabddc5b83b2f656321cb9ae2a879fc1cf/src/fyi_archive/seed.py) · [Cutover gates](plan.md#integration)

`archive_cutover_allowed=false`. Reverting this focused change requires no data
migration. Existing approvals, payloads, checkpoints and receipts are not rewritten.
