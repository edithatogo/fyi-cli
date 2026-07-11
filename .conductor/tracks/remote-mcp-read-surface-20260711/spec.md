# Specification: Remote MCP read surface

## Overview

Expose a small, opt-in, read-only remote Alaveteli tool surface through the Rust MCP server. Every tool delegates network behavior to `SyncClient` and inherits the remote policy, identity, SSRF, pacing, cache, concurrency, response-size, duration, and redaction controls.

## Functional requirements

1. Add remote health, version, search, request retrieval, authority discovery, and authority-feed tools.
2. Prefix or otherwise clearly distinguish remote tools from local SQLite tools.
3. Return stable structured content and a versioned safe error envelope.
4. Disable tools unless validated remote-read policy enables the target instance.
5. Keep all operations idempotent and read-only.
6. Add MCP resources or resource templates where browseable remote context is safer than tool calls.
7. Include source instance, retrieval time, cache status, and correlation ID provenance in structured results.

## Non-functional requirements

- No live network in default CI.
- Bounded pagination, requests, bytes, concurrency, and duration.
- Backward-compatible local MCP schemas and behavior.
- Tool descriptions must state remote side effects, bounds, and recommended call order.
- Prefer resource-first progressive disclosure so remote tools do not inflate every MCP session context.

## Acceptance criteria

- All listed tools appear only when remote-read capability is enabled.
- Every tool has input/output schemas, annotations, success/error fixtures, and WireMock coverage.
- 401/403/404/429/5xx, malformed payload, timeout, guardrail, cache, and redaction paths are tested.
- MCP stdio end-to-end tests exercise discovery and representative calls offline.
- Documentation and issue #170/epic #169 evidence are complete.

## Out of scope

- Any remote mutation.
- Credential entry through MCP arguments.
- Unbounded bulk export or recursive crawling.
