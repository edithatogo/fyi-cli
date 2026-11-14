# MCP product and contract harness

This document records executable evidence for the existing
`remote-mcp-contract-harness-20260711` track. All examples and tests use
synthetic local state. No CI probe sends a request to a live authority.

## Supported wire profiles

| Profile | Behaviour | Automated sensor |
| --- | --- | --- |
| stdio, MCP 2025-06-18 | Newline-delimited UTF-8 JSON-RPC; individual messages; initialization, ping, discovery and calls | `crates/fyi-mcp/tests/wire_contract.rs` |
| stdio, MCP 2025-03-26 | Negotiated legacy revision; batches limited to 64 messages, with no notification replies | Same executable test suite |
| HTTP, MCP 2025-06-18 | Stateless POST with JSON responses; notification/response 202; authenticated GET 405 | `crates/fyi-mcp/src/transport_tests.rs` |
| HTTP, MCP 2025-03-26 | Explicit header or missing-header fallback; bounded legacy batches | Same HTTP tests |
| Other initialization version | Offers supported 2025-06-18; client decides whether to continue | Executable negotiation test |
| Other HTTP version header | HTTP 400; no dispatch | HTTP negative tests |

These profiles do not establish support for later MCP revisions. Optional
SSE streaming, resumable sessions, and server-initiated requests are absent.
The HTTP bearer profile is a single-operator deployment, not OAuth discovery
or tenant isolation. Browser `Origin` headers are denied; bearer authentication
precedes payload parsing. TLS remains the deployment ingress's responsibility.

## Wire and privacy contracts

- JSON-RPC requests use `jsonrpc: "2.0"` and a string or integer ID.
- Notifications produce no response and cannot execute `tools/call` writes.
- Malformed JSON returns `-32700`; an invalid envelope returns `-32600`;
  malformed method parameters return `-32602`; unknown methods return `-32601`.
- Parse and envelope errors use `id: null`; valid request IDs are echoed.
- Generic parser/dispatch errors do not echo supplied input, database paths,
  secrets, or internal parser diagnostics. Application error and remote audit
  contracts require the separate schema/privacy slice before track closure.
- Stdio input and HTTP request bodies are capped at 1 MiB. Oversized stdio
  closes after one safe error, preventing an unbounded drain of attacker input.
- Stdout contains protocol messages only; diagnostics go to stderr.

## Verification

```bash
cargo test -p fyi-mcp --locked
cargo clippy -p fyi-mcp --all-targets -- -D warnings
cargo fmt --all -- --check
python scripts/verify_remote_mcp_contract.py
```

The existing static `v1.json` inventory is historical evidence, not a claim
that a real client can use the server. Actual catalogue snapshots and the
official Python SDK interoperability suite are the next owned slice.

The 1 MiB and 64-message input limits are deterministic resource bounds.
They are not measured process RSS or latency baselines. Mutation execution,
cross-platform/MSRV evidence, remote fault injection and upstream byte limits
must remain visible in track evidence until verified.

## Normative references

- [MCP 2025-06-18 lifecycle](https://modelcontextprotocol.io/specification/2025-06-18/basic/lifecycle)
- [MCP 2025-06-18 transports](https://modelcontextprotocol.io/specification/2025-06-18/basic/transports)
- [MCP 2025-03-26 transports](https://modelcontextprotocol.io/specification/2025-03-26/basic/transports)
- [MCP 2025-06-18 tools](https://modelcontextprotocol.io/specification/2025-06-18/server/tools)
