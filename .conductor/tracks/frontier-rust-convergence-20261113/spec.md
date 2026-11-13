# Frontier architecture and Rust-only migration — specification
Date: 2026-11-13
Status: proposed; implementation gates not yet satisfied
Owner: single maintainer
Canonical related track: `.conductor/tracks/source-adapter-boundary-20260812/`

## Objective and non-goals
Make fyi-cli a verifiable, multi-jurisdiction public-information acquisition and legal-workflow engine. Retain `fyi-core` as the source-network and receipt authority; other consumers (fyi-archive, dashboard, MCP and research) use typed outputs and pinned releases. Do not create a second source HTTP client in downstream projects. No unreviewed remote writes, identity-sensitive submission, paywall circumvention, or automated legal advice.

## Existing state and critical evidence
- Rust crates `fyi-core`, `fyi-cli`, `fyi-mcp` exist and are published separately from Python `fyi_system`.
- `pyproject.toml` currently publishes **three Python commands** `fyi`, `fyi-cli`, `fyi-system`, with `requires-python >=3.11`; Rust CLI and Python CLI have overlapping names but are not demonstrated equivalent in all workflows.
- `uv.lock` is locked for `>=3.11`. CI uses Python 3.11/3.12; Atheris is pinned to 3.1.0; the Presidio privacy action uses isolated Python 3.12.
- No root `rust-toolchain.toml` was found; Rust CI resolves `stable`; Cargo package manifests do not declare `rust-version`.
- CI contains Rust fmt/clippy/tests, Python test/property/fuzzing, mutation, security, coverage, privacy, profiling, packaging and release automation. Existing CI includes RUSTSEC ignore entries, excluded Rust coverage paths, `actions/checkout@v4` and Python lint/typing opt-outs.
- `docs/bleeding-edge/README.md` explicitly marks multiple features foundation/experimental/design-only. Existing source-adapter boundary track retains unmet cutover gates.
- Do not re-label the system frontier until independently verifiable, current-commit evidence proves it.

## Workstream A: Evidence-first source boundary (P0)
Extend existing source-adapter-boundary track rather than duplicating #310–#314:
1. Version an acquisition receipt contract: request identity, redirect chain, UTC retrieval times, HTTP status, source URL (privacy classification), byte hashes, content type, tool/build version, budget, robots and rate-limit policy outcome, error class, redaction metadata; canonical serialization.
2. Atomic append-only writer with idempotent IDs, crash recovery and immutable raw WARC/WACZ linkage. Signed export manifests; byte integrity independent verifier.
3. Pin a release and require offline contract parity, bounded upstream smoke, NZ shadow parity, rollback and downstream archive cutover before retiring legacy transports.
4. Add negative tests for source poisoning, malicious redirects, error-body leakage and malformed archives.

## Workstream B: Jurisdiction and workflow intelligence (P0)
Version rule packs with legal source, effective range, locale, holidays, authority class, exclusion/extension/transfer/review/appeal semantics. Deterministic `CaseEvent` state machine and auditable deadline calculations. Require boundary-day, daylight-saving, transfers, extensions, conflicting rules, retroactive changes, uncertainty and manual-override tests. Explicitly distinguish guidance from legal advice.

## Workstream C: Provider conformance and federated documents (P1)
Evolve read-only provider contract with capability negotiation and backward-compatible schema versions; isolated credentials, network budgets, schema-drift quarantine and opt-in bounded smoke. Require contract fixtures for Alaveteli, MuckRock and FragDenStaat before default enablement. Never silently fall back to scraping or bypass an upstream restriction.

## Workstream D: Grounded agents (P1)
Evaluate authority discovery, request drafting, duplicate detection, evidence gaps, response evaluation and review preparation. Treat retrieved documents as untrusted. Require citation-level grounding, abstention when lacking evidence, per-tool policy permissions, least-privilege filesystem access, prompt-injection red-team fixtures, privacy budget, no unsanctioned submission, and deterministic replay of tool traces (not model outputs).

## Workstream E: Versioned reproducible transformations (P1)
Record input hashes, schema, pipeline Git SHA, dependency lock, parameters, outputs, tool versions and operator overrides in a provenance DAG. Support branch/fork/diff/replay and compare results with declared nondeterministic tolerances. Integrate with fyi-archive, not duplicate ingestion.

## Workstream F: Evaluated research extensions (P2/P3)
Cross-request entity resolution, temporal knowledge graph and agency response metrics; evaluate selection bias, linkage error and disclosure risk. Experimental counterfactual request optimisation and independent multi-agent evidentiary challenge require pre-registered, held-out evaluation and calibrated uncertainty.

## Version policy: Python 3.15 and Rust 1.99
- Requested minimums: Python >=3.15 **for retained Python production surfaces**, Rust MSRV 1.99 **for all Rust crates**; toolchain should use tested Rust 1.99.1 or equivalent compatible patch.
- Python 3.15 stable and Rust 1.99 stable as of 2026-11-13. They are **targets, not claimed to be enabled** in this planning PR.
- Before updating Python metadata, prove dependency resolution and wheels/install or tested sdist for all supported targets, including Atheris, keyring, cryptography, FastMCP and Presidio. A Python 3.12 **isolated security scanner** is a tooling exception, not evidence of Python 3.15 application support; document and eventually remove the exception. Do not disable fuzz or privacy controls merely to meet the target.
- Before updating Rust manifests and CI, run `cargo +1.99.1 test --workspace --all-features --locked`, fmt, clippy, cargo deny/audit and release build on required targets. Check dependency MSRV and lockfile compatibility; add `rust-version = "1.99"` per crate, a consistent `rust-toolchain.toml`, and pin action/tool versions only once green. Use the `1.99.1` patch when pinning execution; `1.99` is the declared MSRV.
- Updating minimums is intentionally a compatibility-breaking release; preserve migration guide, executable rename/entrypoint compatibility and deprecation schedule. Never change `pyproject.toml` without regenerating `uv.lock` and running `uv lock --check`.

## Rust migration decision register
Inventory every Python module, entry point and script, owner, callers, network reachability, credentials, public interface, external consumers, parity fixture and intended fate:
1. **Port to Rust** application functionality: user CLI, acquisition, receipts, archive operations, provider discovery, state, deadlines, security, API/MCP and any runtime service.
2. **Retain temporarily** Python parity and migration verification: read-only comparators and fixture generators, clearly non-production with sunset date.
3. **Retain as dev tooling only** useful independent Python scripts with low maintenance cost, or replace with Rust when they are operational dependencies; do not falsely label these Python-free.
4. **Retire** the legacy PyPI package and three Python entrypoints only after consumer inventory, canonical Rust command mapping, release notes, deprecation period, parity and migration tests. Never silently point the old Python package to unverified Rust behaviour.
5. **Preserve** public command and serialized data contracts through recorded deprecations. Require a no-regression contract test and downstream fyi-archive compatibility before deletion.

## Required acceptance evidence
- Current-commit CI pass with signed provenance, lockfiles, audited exceptions and tested release artifacts.
- Receipt verification independently reconstructs hash chain and catches corrupted or reordered events.
- Deterministic case-machine and deadline invariants hold on synthetic cross-jurisdiction cases.
- Every provider contract fixture passes offline; live smoke bounded, opt-in and non-destructive.
- Red-team tests prove no model-driven write without explicit capability and confirmation.
- Per-surface Python deprecation ledger complete; Rust/Python parity and consumer migration documented.
- Zero unsanctioned exfiltration, zero secret material in logs, redacted evidence artifacts.
- The conductor registry, metadata, issue/PR links and acceptance ledger reconcile automatically.
