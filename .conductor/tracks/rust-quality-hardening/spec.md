# Specification: rust-quality-hardening (Phase 3)

## Overview
This track hardens and verifies the quality of the new Rust codebase. It integrates bleeding-edge testing, profiling, and static analysis infrastructure, targeting >90% code coverage, property-based verification, and zero mutant survivals.

## Functional Requirements
1. **Property-Based Testing (`proptest`):**
   - Implement generative property-based tests for critical database state transformations, encryption key derivations, and URL building logic.
2. **Mutation Testing (`cargo-mutants`):**
   - Inject faults to analyze test suite robustness, fixing code paths where mutations go undetected.
3. **High-Precision Coverage (`cargo-llvm-cov`):**
   - Audit and enforce code coverage targeting `>90%` of executable paths.
4. **Performance & Memory Profiling (`cargo-flamegraph` / `dhat`):**
   - Profile CLI startup and Tor routing layers.
   - Run memory profiling using `dhat` to verify heap allocation counts and detect leaks.

## Non-Functional Requirements
- **Coverage Gate:** >90% coverage on all workspaces.
- **Mutation Survival Rate:** <15% undetected mutants.
- **Leak Prevention:** Zero memory leaks.

## Acceptance Criteria
- Coverage gates report >=90% test coverage.
- Mutation runs completed with high test efficacy.
- Memory leak analysis and performance profiles successfully executed.
