# Specification: rust-core-migration (Phase 1)

## Overview
This track initiates the Rust rewrite of `fyi-cli`. It builds the core library package (`fyi-core`) containing the contract-first Alaveteli API types, the secure database backend (SQLite via `sqlx`), local database/credential encryption (AES-256-GCM with zeroize), and the native Tor network client layer (`arti` + `reqwest`).

## Functional Requirements
1. **Contract-First Schemas:**
   - Define strict Rust models (`serde` serialized) representing Alaveteli REST/RSS endpoints.
   - Use `wiremock` to validate the Rust serialization/deserialization against mock HTTP server responses.
2. **Database Integration (`sqlx` + SQLite):**
   - Implement asynchronous SQLite database client using `sqlx`.
   - Embed startup database migration files.
3. **Cryptographic Compatibility:**
   - Implement AES-256-GCM symmetric encryption using `aes-gcm`.
   - Ensure secure memory scrubbing for keys/passwords with `zeroize`.
   - Implement platform-independent keyring support using `keyring-rs`.
4. **Tor-Native Connection Layer:**
   - Boot and run a native, in-process Tor SOCKS5 proxy using `arti`.
   - Configure `reqwest` to route all FYI.org.nz queries through this in-process Tor proxy.

## Non-Functional Requirements
- **Performance:** Startup latency `< 10ms` for core library initialization.
- **Security:** Strict memory clearing for raw secret variables.
- **Coverage:** Minimum `90%` code coverage target for core modules.

## Acceptance Criteria
- Cargo workspace successfully compiles.
- `fyi-core` tests verify schema validation, DB operations, and AES-256-GCM encryption.
- Tor connections initialize internally and fetch the FYI.org.nz API successfully.
