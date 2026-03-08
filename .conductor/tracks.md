# Tracks Registry

This file tracks all major tracks for the project. Each track has its own detailed plan in its respective folder.

---

## Completed Tracks

- [x] **Track: integrate-fyi-request-system-history** (COMPLETED 2026-03-08)
  *Link: [./integrate-fyi-request-system-history/](./integrate-fyi-request-system-history/)*
  *Description: Migrate and integrate accumulated FYI request system work from versioned zip archives (v1-v14) into this repository, preserving provenance and using v14 as canonical source.*
  *Test Results: 30/30 passed*
  *Commit: 578ee4e*

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
