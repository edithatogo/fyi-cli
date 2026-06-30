# Tracks Registry

This file tracks all major tracks for the project. Each track has its own detailed plan in its respective folder.

---

## Completed Tracks

- [x] **Track: integrate-fyi-cli-history** (COMPLETED 2026-03-08)
  *Link: [./tracks/integrate-fyi-request-system-history/](./tracks/integrate-fyi-request-system-history/)*
  *Description: Migrate and integrate accumulated FYI request system work from versioned zip archives (v1-v14) into this repository, preserving provenance and using v14 as canonical source.*
  *Test Results: 30/30 passed*
  *Commit: 578ee4e*

- [x] **Track: improve-test-coverage** (COMPLETED 2026-03-08)
  *Link: [./archive/improve-test-coverage/](./archive/improve-test-coverage/)*
  *Description: Improve test coverage from 62% to 80%, fix all linting and type checking errors.*
  *Results: 80% coverage achieved, 135 linting errors fixed, 3 type errors fixed, 131 tests passing*
  *Status: TARGET ACHIEVED ✅*

- [x] **Track: testing-infrastructure** (COMPLETED 2026-03-09)
  *Link: [./archive/testing-infrastructure_20260309/](./archive/testing-infrastructure_20260309/)*
  *Description: Comprehensive testing improvements: E2E CLI tests, API contract tests, mutation testing, and integration test fixes.*
  *Target: 280+ tests, >80% mutation score, 90+ maturity score*
  *Results: 280 tests achieved, 36 API contract tests added, 22 E2E tests added, mutation testing infrastructure ready*
  *Status: TARGET ACHIEVED*

- [x] **Track: security-hardening** (COMPLETED 2026-03-09)
  *Link: [./archive/security-hardening/](./archive/security-hardening/)*
  *Description: Comprehensive security hardening: encryption, credentials, sessions, audit logging, data retention, input validation, security headers.*
  *Target: 8 security phases, 200+ tests, security documentation*
  *Results: 8/8 phases complete, 243 tests passing, 6/6 security verifications passing*
  *Status: TARGET ACHIEVED*

- [x] **Track: webapp-coverage-95** (COMPLETED 2026-06-15)
  *Link: [./tracks/webapp-coverage-95/](./tracks/webapp-coverage-95/)*
  *Description: Achieve test coverage of >=95% on the webapp.py module.*
  *Results: 96% coverage achieved, all phases complete, documentation updated*
  *Status: TARGET ACHIEVED*

- [x] **Track: rust-core-migration** (COMPLETED 2026-06-15)
  *Link: [./tracks/rust-core-migration/](./tracks/rust-core-migration/)*
  *Description: Rust Rewrite Phase 1 - Core API contracts, secure database, encryption, and Tor networking layer.*
  *Results: Handshake API contracts, SQLite async connection pool and migrations, AES-256-GCM plus keyring and zeroize, SOCKS5 Tor proxy.*
  *Status: TARGET ACHIEVED*

- [x] **Track: rust-mcp-tui-implementation** (COMPLETED 2026-06-15)
  *Link: [./tracks/rust-mcp-tui-implementation/](./tracks/rust-mcp-tui-implementation/)*
  *Description: Rust Rewrite Phase 2 - Command-line interface, native MCP server implementation, and Ratatui terminal UI dashboard.*
  *Results: clap parser, JSON-RPC 2.0 stdin/stdout MCP server daemon, Ratatui multi-tab dashboard.*
  *Status: TARGET ACHIEVED*

- [x] **Track: rust-quality-hardening** (COMPLETED 2026-06-15)
  *Link: [./tracks/rust-quality-hardening/](./tracks/rust-quality-hardening/)*
  *Description: Rust Rewrite Phase 3 - Quality verification using proptest, cargo-mutants, profiling, and coverage auditing.*
  *Results: proptest generative tests, wiremock integration CLI tests, cargo-mutants configuration, dhat heap profiling integration.*
  *Status: TARGET ACHIEVED*

- [x] **Track: rust-cicd-publishing** (COMPLETED 2026-06-15)
  *Link: [./tracks/rust-cicd-publishing/](./tracks/rust-cicd-publishing/)*
  *Description: Rust Rewrite Phase 4 - CI/CD pipeline setup, cross-compilation packaging (cargo-dist), and Crates.io/registry publishing.*
  *Results: clippy/fmt/audit/coverage GitHub Actions, cross-platform release workflow, Homebrew and Chocolatey manifest templates.*
  *Status: TARGET ACHIEVED*

- [x] **Track: research-grade-quality** (COMPLETED 2026-06-15)
  *Link: [./archive/research-grade-quality/](./archive/research-grade-quality/)*
  *Description: Achieve research-grade quality standards: >95% test coverage, mutation testing, load testing, hypothesis testing, and TypeScript migration preparation.*
  *Results: >95% test coverage (96%), >90% mutation score (91%), load-testing scenarios fully implemented, performance baseline established.*
  *Status: TARGET ACHIEVED*

- [x] **Track: nextjs-web-dashboard** (COMPLETED 2026-06-30)
  *Link: [./archive/nextjs-web-dashboard/](./archive/nextjs-web-dashboard/)*
  *Description: Built a modern Next.js web interface with interactive charting, TailwindCSS, MCP-backed request management, search/filtering, bulk actions, responsive layouts, and accessibility improvements.*
  *Status: TARGET ACHIEVED*

- [x] **Track: mfa-authentication-layer** (COMPLETED 2026-06-30)
  *Link: [./archive/mfa-authentication-layer/](./archive/mfa-authentication-layer/)*
  *Description: Implemented Multi-Factor Authentication (MFA) via TOTP tokens inside the security engine to guard credential access, CLI management commands, TUI visibility, and MCP tool exposure.*
  *Status: TARGET ACHIEVED*

- [x] **Track: bulk-site-enumeration** (COMPLETED — fyi-archive capability)
  *Link: [./tracks/bulk-site-enumeration/](./tracks/bulk-site-enumeration/)*
  *Description: Full-corpus discovery of fyi.org.nz via advanced-search Atom/JSON feeds (date-windowed, paginated) + optional sequential-ID gap backfill. Enables the fyi-archive historical seed.*
  *Consumer: fyi-archive `historical_seed_orchestration_20260627`.*

- [x] **Track: faithful-archive-capture** (COMPLETED — fyi-archive capability)
  *Link: [./tracks/faithful-archive-capture/](./tracks/faithful-archive-capture/)*
  *Description: Faithful per-request capture (JSON + rendered HTML + attachment binaries) written as WARC 1.1 records packaged into WACZ, with content-addressed attachment dedup. The archival source of truth.*
  *Consumer: fyi-archive `historical_seed_orchestration_20260627`.*

- [x] **Track: archival-content-diff** (COMPLETED — fyi-archive capability)
  *Link: [./tracks/archival-content-diff/](./tracks/archival-content-diff/)*
  *Description: Content-addressed change detection (added/updated/removed by SHA-256 of the captured JSON) → latest_changes.json. Powers the fyi-archive prospective daily sync. Read-only; distinct from the tracker-focused offline-sync-engine.*
  *Consumer: fyi-archive `prospective_sync_orchestration_20260627`.*

- [x] **Track: archive-health-doctor** (COMPLETED — fyi-archive capability)
  *Link: [./tracks/archive-health-doctor/](./tracks/archive-health-doctor/)*
  *Description: Archive health reporting (freshness, coverage gaps, raw record counts) consumed by fyi-archive's doctor/parity CI.*
  *Consumer: fyi-archive `observability_quality_20260627`.*

- [x] **Track: offline-sync-engine** (COMPLETED 2026-06-30)
  *Link: [./archive/offline-sync-engine/](./archive/offline-sync-engine/)*
  *Description: Design a SQLite sync service to handle periodic database caching, OIA request tracking, and conflict reconciliation with the upstream FYI API.*
  *Status: TARGET ACHIEVED*

- [x] **Track: release-readiness-audit-20260630** (COMPLETED 2026-06-30)
  *Link: [./archive/release-readiness-audit-20260630/](./archive/release-readiness-audit-20260630/)*
  *Description: Reconcile release readiness, packaging, documentation, and CI state for the current Rust-first FYI CLI.*
  *Status: TARGET ACHIEVED*

- [x] **Track: api-contract-hardening-20260630** (COMPLETED 2026-06-30)
  *Link: [./archive/api-contract-hardening-20260630/](./archive/api-contract-hardening-20260630/)*
  *Description: Hardened FYI/Alaveteli API contracts, error handling, and live-safe integration boundaries.*
  *Status: TARGET ACHIEVED*

- [x] **Track: load-testing-baseline** (COMPLETED)
  *Link: [./tracks/load-testing-baseline/](./tracks/load-testing-baseline/)*
  *Description: Establish performance baselines and load testing infrastructure for the FYI request system.*
  *Status: TARGET ACHIEVED*

- [x] **Track: mutation-testing-execution** (COMPLETED)
  *Link: [./tracks/mutation-testing-execution/](./tracks/mutation-testing-execution/)*
  *Description: Execute mutation testing on the codebase to verify test effectiveness and achieve greater than 90% mutation score.*
  *Status: TARGET ACHIEVED*

---

## Active Tracks

- None

---

## Imported Tracks (from v14)

The following tracks were imported from fyi-cli-v14 and represent completed phases of the original Python implementation:

- [x] **Track: fyi-phase-1** - Initial project scaffolding
- [x] **Track: fyi-phase-2** - Core data models
- [x] **Track: fyi-phase-3** - FYI API integration
- [x] **Track: fyi-phase-4** - Request drafting
- [x] **Track: fyi-phase-5** - Response tracking
- [x] **Track: fyi-phase-6** - Timeline management
- [x] **Track: fyi-phase-7** - Basic reporting
- [x] **Track: fyi-phase-8** - Scheduler integration
- [x] **Track: fyi-phase-9** - Dashboard UI
- [x] **Track: fyi-phase-10** - Follow-up generation
- [x] **Track: fyi-phase-11** - Bundle export
- [x] **Track: fyi-phase-12** - Correspondence pack
- [x] **Track: fyi-phase-13** - Privacy hardening
- [x] **Track: fyi-phase-14** - Next action operator

*Note: These phases represent the completed Python implementation. Future tracks may focus on Rust reimplementation or new features.*

---

## Future Tracks (Proposed)

Potential tracks for future development:

- [x] **Track: rust-cli-scaffold** (COMPLETED) - Create Rust CLI skeleton with Clap
- [x] **Track: fyi-client-crate** (COMPLETED) - Extract/create FYI.org.nz API client library
- [x] **Track: mcp-server** (COMPLETED) - Implement Model Context Protocol server
- [x] **Track: tor-integration** (COMPLETED) - Add TOR/proxy support with arti or Stem
- [x] **Track: python-rust-migration** (COMPLETED) - Systematic migration from Python to Rust

---

## Track Status Legend

- `[ ]` - Not started
- `[~]` - In progress
- `[x]` - Completed
