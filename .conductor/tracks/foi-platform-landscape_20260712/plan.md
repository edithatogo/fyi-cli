# Implementation Plan

## Phase 1: Evidence inventory

- [x] Review the six named candidates against primary documentation; record jurisdiction,
  lineage, authentication, writes, privacy, rate-limit evidence and bounded experiments.
  Evidence: `docs/foi-platform-landscape.md`, research snapshot 2026-11-14.
- [x] Distinguish directly retrieved documentation from indexed-only sources.
  FragDenStaat and USCIS remain provisional; no live API or schema availability claimed.
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
- [ ] Capture raw primary schema/documentation snapshots with retrieval evidence and
  byte fingerprints. Indexed-only material is not a substitute for a retrieved snapshot.
- [x] Implement a network-free completeness/freshness check and supplied-observation
  drift sensor; malformed, incomplete, unavailable and deprecated observations fail closed.
- [x] Add offline tests and the Conductor CI gate. Local baseline: checker absent,
  collection failed as expected. Implementation: 40 tests passed on Python 3.13.5/Linux.
- [ ] Verify final PR-head hosted checks before merge. No local full-repository suite
  is claimed: this environment could not resolve GitHub for a full clone.
- [ ] Connect authorised documentation/schema observations through the source-adapter
  boundary to the offline drift comparator. A contract projection hash is not a raw
  schema fingerprint. This remains an outstanding #195 phase-3 requirement.

## Completion boundary

The refreshed snapshot and offline validation are an increment toward #195, not
issue completion. The reopening comment (issuecomment-6843663307) explicitly
requires raw schema fingerprints and documentation drift evidence. Both the
issue and track remain open until those phase-3 requirements have evidence.
This is not provider rollout or archive cutover; no human approval or successful
live smoke is fabricated. The next reviews are 2026-12-14 (first four platforms)
and 2027-01-13 (Canada/India).
