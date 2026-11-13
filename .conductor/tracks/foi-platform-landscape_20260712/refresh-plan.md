# Platform landscape refresh implementation plan

Goal: refresh the six-candidate research snapshot for #195 without enabling
providers or adding source-network transports. Existing spec.md is authoritative.

Design: a reviewed JSON contract inventory plus a small standard-library-only
validator and offline observation comparator. A documentation-only edit would
leave dates/permissions unchecked; a live crawler would add avoidable network,
authentication and privacy risks. This implementation uses supplied observations
and leaves live collection to explicitly authorised acquisition workflows.

- [x] Review primary sources and distinguish directly retrieved from indexed-only evidence.
- [x] Write offline validation/drift tests, execute the expected failing baseline.
- [x] Implement schema, dates, source allowlists, fail-closed decision gates and semantic fingerprints.
- [x] Exercise matching/changed/incomplete/deprecated observations and CLI exit codes.
- [x] Update the dated six-platform comparison and existing Conductor evidence.
- [x] Run focused tests and compile checks locally: 40 passed.
- [ ] Run complete hosted CI on the final PR head.
- [ ] Merge only after required checks succeed and verify remote branch removal.

Review focus: duplicate JSON keys, omitted platforms, future/overdue review dates,
credential-bearing evidence URLs, and observations incorrectly treated as approval.
Fingerprint meaning: SHA-256 of the explicitly reviewed contract projection,
not a claim that remote HTML or OpenAPI bytes were downloaded or authenticated.

Ruling: the issue reopening comment retains raw schema fingerprints and actual
documentation drift evidence as phase-3 acceptance. This PR adds the reviewed
snapshot and offline comparator but leaves #195 and the track open. The validator
is repository tooling, not a new Python application adapter.
