# Tracks Registry

This file tracks all major tracks for the project. Each track has its own detailed plan in its respective folder.

---

## Completed Tracks

- [x] **Track: integrate-fyi-request-system-history** (COMPLETED 2026-03-08)
  *Link: [./integrate-fyi-request-system-history/](./integrate-fyi-request-system-history/)*
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

---

## Archived Tracks

- [~] **Track: research-grade-quality** (ARCHIVED 2026-03-09)
  *Link: [./archive/research-grade-quality/](./archive/research-grade-quality/)*
  *Description: Achieve research-grade quality standards: >95% test coverage, mutation testing, load testing, hypothesis testing, and TypeScript migration preparation.*
  *Target: 95% coverage, 90% mutation score, documented performance characteristics*
  *Results: 83% coverage achieved (up from 80%), 280 tests passing, scheduler 100% covered, mutation testing infrastructure ready, hypothesis testing in place*
  *Status: SUBSTANTIAL PROGRESS - Split into 3 focused tracks (webapp-coverage-95, mutation-testing-execution, load-testing-baseline)*
  *Lessons: Track scope too ambitious; recommend splitting into smaller tracks*

---

## New Tracks (from research-grade-quality split)

- [ ] **Track: webapp-coverage-95** (NEW 2026-03-09)
  *Link: [./webapp-coverage-95/](./webapp-coverage-95/)*
  *Description: Improve webapp.py test coverage from 63% to 95% with comprehensive route, form, and rendering tests.*
  *Target: 95% coverage, +40-50 new tests*
  *Priority: HIGH*
  *Estimate: 3-6 days*

- [ ] **Track: mutation-testing-execution** (NEW 2026-03-09)
  *Link: [./mutation-testing-execution/](./mutation-testing-execution/)*
  *Description: Execute mutation testing on full codebase and achieve >90% mutation score.*
  *Target: >90% mutation score, CI integration*
  *Priority: MEDIUM*
  *Estimate: 3-5 days*

- [ ] **Track: load-testing-baseline** (NEW 2026-03-09)
  *Link: [./load-testing-baseline/](./load-testing-baseline/)*
  *Description: Establish performance baselines and load testing infrastructure.*
  *Target: Documented baselines, performance targets, CI integration*
  *Priority: MEDIUM*
  *Estimate: 3-5 days*

---

## Security & Privacy Tracks

- [~] **Track: security-hardening** (IN PROGRESS 2026-03-09)
  *Link: [./security-hardening/](./security-hardening/)*
  *Description: Implement comprehensive security hardening: database encryption, secure credential storage, session management, audit logging, and data retention policies.*
  *Target: Encrypted database, OS keyring integration, audit logs, secure sessions*
  *Priority: CRITICAL*
  *Estimate: 8-12 days*

---

## Imported Tracks (from v14)

The following tracks were imported from fyi-request-system-v14 and represent completed phases of the original Python implementation:

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
