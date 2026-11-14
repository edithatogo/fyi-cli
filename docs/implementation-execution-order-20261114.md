# Dependency-ordered execution roadmap (2026-11-14)

Status: proposed sequence, pending PR merge and Conductor reconciliation. This document does **not** assert functional completion.

## Principles
- Rust is the retained application target. Freeze Python application feature development; preserve necessary safety and compatibility patches until independently tested cutover.
- Reuse existing `.conductor` tracks and GitHub issues, particularly `source-adapter-boundary-20260812` (#309–#314), `frontier-rust-convergence-20261113`, and `quality-frontier-testing-cicd-20261113`. Do not duplicate completed work.
- Each track needs a machine-readable capability ledger with command/tool, Rust handler, Python equivalent (where present), consumer, contract/fixture, status (`unimplemented|partial|parity_verified|retired`), test evidence SHA, release pin and rollback.
- No archive cutover before real-byte evidence, release-pinned downstream shadow parity, and fail-closed safety verification; keep `archive_cutover_allowed=false`.
- Test/security/provenance/Conductor checks are continuous gates, not deferred to late phases. No manual approvals for safe routine PRs; preserve explicit user confirmation for external writes, credentials and sensitive or breaking operations.
- A complete track requires a reproducible command, observable positive and negative outcomes, matching CI SHA, consumer compatibility, release/rollback evidence and reconciled Conductor status.

## Execution order and dependencies

| Seq | Workstream | Existing owner | Exit gate |
| --- | --- | --- | --- |
| 0 | Current-branch reconciliation | PR #430; #310 | Confirm all required latest-head checks, merge PR #430, prune branch and refresh baseline; do not claim green from queued jobs |
| 1 | Source acquisition and evidence integrity | `source-adapter-boundary-20260812`, #309/#310/#312; #311/#313/#314 historical work | Versioned strict Python/Rust receipt parity, bounded CDX/replay/probe/capture, raw/attachment/redirect and WARC/WACZ byte linkage, immutable publication and crash/Windows fault verification, safe network policy, independent fixtures and verified CI |
| 2 | Rust core request/authority functions and legacy data migration | `frontier-rust-convergence-20261113`; command parity inventory | Import/list authorities; request CRUD/status/timeline; deterministic state, transactional schema migrations and safe legacy SQLite import, negative tests |
| 3 | Archive downstream cutover | #309 and `edithatogo/fyi-archive` consumer contract | Pin Rust release, offline output parity + bounded hosted smoke + NZ shadow capture/event/attachment/checkpoint parity, rollback rehearsal; only then enable cutover |
| 4 | Retained CLI/MCP behavioural parity | `frontier-rust-convergence-20261113` | Every retained command/tool has contract and verified behaviour; retire unneeded entries deliberately; migrate three Python entrypoints only when downstream consumers qualify |
| 5 | Reporting and automation | `frontier-rust-convergence-20261113` | Explicit product scope selection; exports, attachment reports, follow-ups, scheduler, server and dashboard either implemented with integration tests or explicitly retired |
| 6 | Legacy Python application retirement | `frontier-rust-convergence-20261113` | All retained consumers and data migrated, release/rollback evidence, published deprecation notes, then remove obsolete Python application paths without prematurely removing independent tooling |
| 7 | Frontier release and CI hardening | existing quality/security/testing tracks | Release-level inclusive coverage, changed-risk and full scheduled mutation/fuzz/metamorphic, cross-platform security, SBOM/signing/provenance, reproducible manifests, dependency-rationalisation measurements and Conductor drift repair |
| 8 | Jurisdictions/providers | #195/#234 and existing jurisdiction tracks | Source-backed provider go/no-go, allowed terms/auth, bounded signed-off captures, fixtures per jurisdiction and no escape from source network boundary |
| 9 | Packaging/registry rollout | #193/#194/#197/#198/#32 | One signed release source, install smoke/hashes/SBOM and accurate external publication ledger; external acceptance reported as blocked until independently observed |
| 10 | Research extensions | `frontier-rust-convergence-20261113` | Evaluated deterministic rules/case engine, grounded agents and versioned provenance DAG; no speculative auto-enable |

## Immediate track: Source acquisition and evidence integrity

Preflight: check PR #430 status; merge only after latest-head required checks succeed; do not pile unrelated code into that PR. Rebase a new feature branch from current `master`.

1. Inventory all existing network entrypoints in Python, Rust CLI/core/MCP and downstream `fyi-archive`, including HTTP clients that escape the intended boundary. Record consumer contracts and pinned versions.
2. Produce a verified cross-runtime conformance matrix: JSON Schema 2020-12 constraints, canonicalization (Unicode/numeric/UTC/escaping), receipt self-digest, raw/result hashes, request budget, 304/429/503, redirects, SSRF/DNS rebinding, symlinks, conflicts, races, partial publication and crash recovery.
3. Complete Rust bounded CDX + Wayback replay and preserve Python-compatible checkpoint and original response bytes; add manifests linking entity bytes, metadata, headers, attachment streams and preservation records, without conflating WARC records with HTTP entity bytes.
4. Integrate the atomic writer into **every retained network command**, enforce fail-closed receipt/no-clobber semantics and shared Retry-After pacing; audit absence of new alternate HTTP clients.
5. Run deterministic offline fixtures, independent-process adversarial tests on Linux/macOS/Windows, Rust 1.99.1 locked tests/Clippy/fmt, required Python compatibility/security checks, source contracts, mutation/property tests targeted to changed risk, and signed manifest verification where available.
6. Publish PRs in bounded vertical slices, inspect latest-head CI and merge/prune one by one. Cross-repo `fyi-archive` cutover remains sequence 3, not an implicit part of #310 closure; maintain `archive_cutover_allowed=false` until verified.

## Evidence and reconciliation
- Update existing `source-adapter-boundary-20260812` plan, conformance manifests, docs and issues in place. Record CI run links and exact commit SHAs, not only checkboxes.
- Do not mark a capability `parity_verified` on a command name match, a stub, passing mocks alone, or a test workflow that was merely triggered.
- Update `.conductor/tracks.md`, indexes/metadata and issue states only to reflect independently demonstrated status.
- Leave distribution or optional new providers outside the current track unless necessary for its acceptance.
