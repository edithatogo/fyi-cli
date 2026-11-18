# Implementation Plan

## Phase 1: Evidence inventory

- [x] Review the six named candidates against primary documentation; record jurisdiction,
  lineage, authentication, writes, privacy, rate-limit evidence and bounded experiments.
  Evidence: `docs/foi-platform-landscape.md`, research snapshot 2026-11-17.
- [x] Distinguish directly retrieved documentation from indexed-only sources.
  All six selected documents were retrieved. FragDenStaat remains provisional
  because its documentation and source disagree; no deployed API is qualified.
- [x] Preserve explicit Alaveteli exclusions and mark unlocated transaction APIs as
  `not_identified`, not as proof that an API cannot exist.

## Phase 2: Comparative feasibility

- [x] Rank MuckRock, FragDenStaat, FOIA.gov, USCIS, Canada ATIP and India RTI.
- [x] Give each candidate a documented go/no-go decision and bounded next experiment.
- [x] Separate public discovery from account, identity, case, payment and submission
  workflows; research metadata cannot enable provider/network/write capabilities.

## Phase 3: Maintenance and monitoring (partially implemented)

- [x] Store source URLs, review dates, confidence, per-platform cadence and deterministic
  reviewed-contract projection fingerprints in `conformance/providers/foi-platform-landscape-v1.json`.
- [x] Capture raw primary schema/documentation snapshots with retrieval evidence and
  byte fingerprints. Six credential-free research GETs used the managed HTTPS
  proxy and normal TLS verification. The baseline records that acquisition method;
  retained raw bodies were independently checked against all six digests.
- [x] Implement a network-free completeness/freshness check and supplied-observation
  drift sensor; malformed, incomplete, unavailable and deprecated observations fail closed.
- [x] Add offline tests and the Conductor CI gate. The semantic and byte-level
  checkers have socket-free tests. The byte checker distinguishes metadata,
  retained research bodies and complete guarded-adapter observations.
- [~] Verify final PR-head hosted checks before merge. PR #492 is the acceptance
  surface; the initial source tree passed MSRV, Clippy and Rust workspace tests.
- [~] Connect authorised documentation/schema observations through the source-adapter
  boundary to the offline drift comparator. A contract projection hash is not a raw
  schema fingerprint. The adapter and receipt-bound comparator are implemented
  and tested offline. Successful live qualification remains open: this execution
  environment cannot resolve direct DNS, and ambient proxies are refused.

### 2026-11-17 implementation scope

Use a fixed, six-source Rust documentation adapter through the existing guarded
source transport, with explicit live consent, one GET, no redirects/credentials,
bounded response/time, an acquisition receipt and a final observation marker.
Keep original source bytes in operator-controlled capture bundles: source rights,
page tokens and identity-shaped documentation examples prevent blanket public
redistribution. Commit only reviewed metadata and digests. An offline verifier
must distinguish metadata validation from verified local bytes and fail closed on
incomplete/tampered/stale observations. A byte change requires human review; it is
not proof of a semantic API change or permission to promote a provider.

Implementation is published in [PR #492](https://github.com/edithatogo/fyi-cli/pull/492).
The initial head `85db1e2a5b2927428c3c22a1cac3c06808e63ba0` passed the hosted
MSRV job on Rust 1.99.1, including all-target/all-feature Clippy, workspace tests,
all 12 documentation-adapter tests and the example argument-parser test
([run 44053568367](https://github.com/edithatogo/fyi-cli/actions/runs/44053568367)).
Local CPython 3.15.2 focused checks and Rustfmt pass. Local Rust builds were
terminated with SIGKILL in existing dependencies; no local Rust test execution
is claimed. Independent review's timestamp precision finding was fixed by
rejecting submicrosecond timestamp strings; the provenance separation was also
independently reviewed.

The workflow-built observer artifact was checked against its source tree,
revision and SHA-256 before use. All six explicitly authorised live attempts
failed closed in this environment because direct DNS is unavailable. No attempt
produced a successful observation, and the public-DNS/proxy guards were preserved.
The real research captures are labelled `managed_proxy_https_review`; they are
not relabelled as Rust captures and have no fabricated acquisition receipts.
`--baseline-bodies` verifies their retained raw bytes. `--observations` continues
to require complete guarded-adapter evidence. An operator with permitted direct
public DNS can use the documented observer to finish live qualification.

## Completion boundary

The reopening comment (issuecomment-6843663307) explicitly requires raw schema
fingerprints and documentation drift evidence. Raw research fingerprints and
the guarded observation implementation are now present. The issue and track
remain open until a successful live observer run and final PR-head checks have
evidence. Provider rollout and archive cutover remain outside this track. The
next reviews are 2026-12-17 (first four platforms) and 2027-01-16 (Canada/India).
