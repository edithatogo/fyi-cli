# Specification: Remote MCP observability and security

## Overview

Create the fail-closed policy and operator-control foundation required before the MCP server can expose remote Alaveteli operations. The implementation must reuse the existing resource-aware agent middleware and make remote capability state observable without exposing credentials or sensitive response content.

## Functional requirements

1. Define validated per-instance remote MCP policy with separate read and write capabilities.
2. Keep all remote capabilities disabled by default and reject ambiguous configuration.
3. Add an immediate operator kill switch, degraded mode, circuit-breaker state, and deterministic recovery.
4. Emit structured, correlated MCP/SyncClient audit events and bounded-cardinality metrics.
5. Expose safe capability and remote-health status without secrets.
6. Apply the existing identity, SSRF, pacing, cache, concurrency, request, byte, duration, and redaction controls to every remote MCP operation.
7. Enforce per-instance request/byte/time budgets and expose safe budget utilization to operators.

## Non-functional requirements

- No credentials, tokens, PII, response bodies, or attachment contents in logs, traces, metrics, or errors.
- Offline deterministic CI; live smoke remains explicit, bounded, and opt-in.
- Backward compatible with the current local-only MCP server.
- No proprietary observability dependency is mandatory.

## Acceptance criteria

- Startup validation fails closed for invalid or over-broad policy.
- Read enablement cannot imply write enablement.
- Kill switch and circuit breaker prevent outbound requests deterministically.
- Status, audit, trace, and metrics schemas have unit/property/security coverage.
- Threat model, privacy review, retention rules, incident runbook, and operator documentation are current.
- GitHub issue #172 and parent epic #169 are linked in code and track evidence.

## Out of scope

- Remote MCP endpoint implementations.
- User-facing GUI changes.
- Mandatory external telemetry services.
