# Plan: rust-core-migration (Phase 1)

## Phase 1.1: Workspace Scaffolding & API Contract Verification
- [x] Task: Scaffold Cargo Workspace with crates `fyi-core`, `fyi-cli`, and `fyi-mcp`
- [x] Task: Write failing contract-first API schema serialization tests
- [x] Task: Implement Alaveteli JSON schemas using `serde` and validate with `wiremock`
- [x] Task: Conductor - User Manual Verification 'Phase 1.1: Workspace & Contracts' (Protocol in workflow.md)

## Phase 1.2: Database Storage & Async Schema Migrations
- [x] Task: Write failing database schema integration tests using `sqlx` in offline mode
- [x] Task: Implement async SQLite repository connection pool and embedded SQL migrations
- [x] Task: Conductor - User Manual Verification 'Phase 1.2: Database Storage' (Protocol in workflow.md)

## Phase 1.3: Secure Keyring, Cryptography & zeroize Protection
- [x] Task: Write failing unit tests for AES-256-GCM encryption and zeroize scrubbing
- [x] Task: Implement `aes-gcm` credential encryption and system keyring-rs wrapper
- [x] Task: Conductor - User Manual Verification 'Phase 1.3: Cryptography' (Protocol in workflow.md)

## Phase 1.4: In-Process Tor Routing Interface (`arti` + `reqwest`)
- [x] Task: Write failing connection tests for Tor circuit states
- [x] Task: Implement in-process Tor proxy client via `arti` and route `reqwest` client traffic
- [x] Task: Conductor - User Manual Verification 'Phase 1.4: Tor Integration' (Protocol in workflow.md)
