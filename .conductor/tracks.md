# Tracks Registry

This file tracks all major tracks for the project. Each track has its own detailed plan in its respective folder.

---

## Completed Tracks

- [x] **Track: integrate-fyi-cli-history** (COMPLETED 2026-03-08)
  *Link: [./integrate-fyi-cli-history/](./integrate-fyi-cli-history/)*
  *Description: Migrate and integrate accumulated FYI request system work from versioned zip archives (v1-v14) into this repository, preserving provenance and using v14 as canonical source.*
  *Test Results: 30/30 passed*
  *Commit: 578ee4e*

- [x] **Track: improve-test-coverage** (COMPLETED 2026-03-08)
  *Link: [./improve-test-coverage/](./improve-test-coverage/)*
  *Description: Improve test coverage from 62% to 80%, fix all linting and type checking errors.*
  *Results: 80% coverage achieved, 135 linting errors fixed, 3 type errors fixed, 131 tests passing*
  *Status: TARGET ACHIEVED ✅*

- [x] **Track: testing-infrastructure** (COMPLETED 2026-03-09)
  *Link: [./testing-infrastructure_20260309/](./testing-infrastructure_20260309/)*
  *Description: Comprehensive testing improvements: E2E CLI tests, API contract tests, mutation testing, and integration test fixes.*
  *Target: 280+ tests, >80% mutation score, 90+ maturity score*
  *Results: 280 tests achieved ✅, 36 API contract tests added ✅, 22 E2E tests added ✅, mutation testing infrastructure ready ✅*
  *Status: TARGET ACHIEVED ✅*

- [x] **Track: security-hardening** (COMPLETED 2026-03-09)
  *Link: [./security-hardening/](./security-hardening/)*
  *Description: Comprehensive security hardening: encryption, credentials, sessions, audit logging, data retention, input validation, security headers.*
  *Target: 8 security phases, 200+ tests, security documentation*
  *Results: 8/8 phases complete, 243 tests passing, 6/6 security verifications passing*
  *Status: TARGET ACHIEVED ✅*

---

## Active Tracks

- [ ] **Track: research-grade-quality** (NEW)
  *Link: [./research-grade-quality/](./research-grade-quality/)*
  *Description: Achieve research-grade quality standards: >95% test coverage, mutation testing, load testing, hypothesis testing, and TypeScript migration preparation.*
  *Target: 95% coverage, 90% mutation score, documented performance characteristics*

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

- [ ] **Track: rust-cli-scaffold** - Create Rust CLI skeleton with Clap
- [ ] **Track: fyi-client-crate** - Extract/create FYI.org.nz API client library
- [ ] **Track: mcp-server** - Implement Model Context Protocol server
- [ ] **Track: tor-integration** - Add TOR/proxy support with arti or Stem
- [ ] **Track: python-rust-migration** - Systematic migration from Python to Rust

---

## Track Status Legend

- `[ ]` - Not started
- `[~]` - In progress
- `[x]` - Completed
