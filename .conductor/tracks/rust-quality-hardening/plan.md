# Plan: rust-quality-hardening (Phase 3)

## Phase 3.1: Property-Based Verification & Integration Tests
- [x] Task: Integrate `proptest` and write generative tests for DB state transitions and encryption
- [x] Task: Implement full E2E CLI and mock API client tests
- [x] Task: Conductor - User Manual Verification 'Phase 3.1: Property & E2E Testing' (Protocol in workflow.md)

## Phase 3.2: Mutation & Code Coverage Auditing
- [x] Task: Set up `cargo-llvm-cov` coverage targets (>90% threshold check)
- [x] Task: Run `cargo-mutants` and write missing unit tests to eliminate surviving mutants
- [x] Task: Conductor - User Manual Verification 'Phase 3.2: Mutation & Coverage' (Protocol in workflow.md)

## Phase 3.3: Heap Profiling & Performance Flamegraphs
- [x] Task: Set up `dhat` memory profiler and compile flamegraph targets
- [x] Task: Optimize startup latency and network execution flows
- [x] Task: Conductor - User Manual Verification 'Phase 3.3: Heap & Flamegraph Profiling' (Protocol in workflow.md)
