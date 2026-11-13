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

## Phase 3: Maintenance and monitoring

- [x] Store source URLs, review dates, confidence, per-platform cadence and deterministic
  reviewed-contract fingerprints in `conformance/providers/foi-platform-landscape-v1.json`.
- [x] Implement a network-free completeness/freshness check and supplied-observation
  drift sensor; malformed, incomplete, unavailable and deprecated observations fail closed.
- [x] Add offline tests and the Conductor CI gate. Local baseline: checker absent,
  collection failed as expected. Implementation: 40 tests passed on Python 3.13.5/Linux.
- [ ] Verify final PR-head hosted checks before merge. No local full-repository suite
  is claimed: this environment could not resolve GitHub for a full clone.
- [ ] Acquire raw remote schemas and continuous monitoring through the authorised
  source-adapter boundary when appropriate. A contract projection hash is not a raw
  schema fingerprint. This recurring operation is outside the completed #195 snapshot.

## Completion boundary

The #195/spec.md research acceptance criteria are met by the dated six-platform
snapshot, explicit decisions, confidence levels, sources and refresh procedure.
This is not provider rollout or archive cutover. Track status stays in-progress
for recurring evidence refresh and separate remote-capture work; no human approval
or successful live smoke is fabricated. The next reviews are 2026-12-14 (first
four platforms) and 2027-01-13 (Canada/India).
