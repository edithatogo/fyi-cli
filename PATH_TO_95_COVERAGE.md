# Path to 95% Coverage

This document outlines the testing strategy, tooling, and scripts used to maintain high code coverage targets (>90% baseline, striving for 95%+) in the FYI Request System Rust rewrite.

## Tooling: `cargo-llvm-cov`

We use `cargo-llvm-cov` to generate source-based code coverage reports for the Rust workspace. It uses LLVM instrumentation to produce highly accurate line, region, and branch coverage metrics.

### Installation

To install `cargo-llvm-cov` and the required LLVM tools:

```bash
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov
```

### Running Coverage Audits

To audit the codebase against our coverage requirements:

1. **Local Coverage Runs**:
   Generate an HTML report and open it in your browser:
   ```bash
   cargo llvm-cov --all-features --workspace --html --open
   ```

2. **Automated Enforcement (CI & Scripts)**:
   Audit coverage targets using the provided scripts. These commands enforce a strict **90% coverage threshold** and will exit with a non-zero code if the threshold is not met:
   - On Windows: `powershell ./run-coverage.ps1`
   - On Unix: `./run-coverage.sh`

   Equivalent command:
   ```bash
   cargo llvm-cov --all-features --workspace --fail-under-lines 90
   ```

## Targets & Priority Areas

1. **`fyi-core` (Data Structures & DB Abstractions)**:
   - Priority: **Critical** (>95% coverage)
   - Ensure all database state transitions (`db.rs`) and cryptographic operations (`security.rs`) are fully covered by unit and integration tests.

2. **`fyi-cli` (CLI Interface & Main Loop)**:
   - Priority: **High** (>90% coverage)
   - Test argument parsing, command dispatching, and conditional workflows.
