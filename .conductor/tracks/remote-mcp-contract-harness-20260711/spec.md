# Specification: Remote MCP contract harness

## Overview

Build a versioned, offline, aggressive verification harness for remote MCP tools across supported Alaveteli response variants, operating systems, Rust versions, and representative MCP clients.

## Functional requirements

1. Maintain shared JSON/RSS/Atom/error fixtures with explicit contract versions.
2. Verify every remote tool schema, annotation, structured result, and error envelope.
3. Add MCP stdio end-to-end tests for disabled, read-enabled, degraded, and write-gated modes.
4. Add property and mutation tests for bounds, SSRF, capability checks, confirmation/replay protection, state expectations, and redaction.
5. Add performance, memory, latency, request, byte, and concurrency budgets.
6. Publish a compatibility and release-readiness matrix.
7. Add deterministic fault injection for timeout, disconnect, malformed chunk, cache corruption, clock skew, and circuit-breaker recovery.
8. Version MCP schemas and test deprecation/unknown-version behavior.

## Non-functional requirements

- Deterministic offline default suite.
- Fast layered gates plus explicit expensive mutation/performance execution points.
- Reusable fixtures shared with paired Alaveteli contract tests where practical.
- Fail closed on unknown contract versions or unsupported schemas.

## Acceptance criteria

- Unit, integration, end-to-end, smoke/system, property, mutation, edge, performance, security, compatibility, usability, regression, and sanity categories have executable sensors or explicit gates.
- Supported Windows/Linux/macOS and Rust/MSRV combinations are recorded and exercised in CI where practical.
- Mutation targets protect security and capability boundaries.
- Issue #173 and epic #169 evidence are complete.

## Out of scope

- Live load against volunteer Alaveteli instances.
- Replacing existing repository-wide quality gates.
