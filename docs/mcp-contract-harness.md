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

## Remote admission and confirmation contract

Each configured instance admits at most one active MCP remote operation. The
request allowance counts admitted operations, including failures and timeouts;
it does not count each internal HTTP retry or request inside an operation.
The duration budget covers the complete admitted future. Returned payload bytes
are counted before publication, including rejected oversized results. Failure,
timeout and cancellation release concurrency without refunding the attempt.
These are MCP boundary limits; raw download/retry/allocator limits require
separate source-adapter evidence.

Confirmation tokens are opaque OS-generated random capabilities. Unused tokens
expire after 300 seconds using a monotonic clock. They bind an instance,
operation, idempotency key and immutable preparation. Commit arguments contain
only `instance_id`, `phase`, `confirmation_token` and `idempotency_key`; changing
the payload requires a new preparation. Invalid, expired and scope-mismatched
tokens consume no operation allowance. Concurrency/budget rejection leaves a
valid pending confirmation intact.

The process-local replay ledger retains at most 128 entries. Each preparation
and cached result is limited to 64 KiB of serialized JSON. Unused expired
confirmations can be reclaimed; attempted writes are retained even when their
remote outcome is uncertain. Completed results remain retrievable without a
fresh dispatch allowance, subject to current capability and operator controls.
The ledger and raw payloads are excluded from diagnostic formatting.

Restarting loses the ledger. This is not durable, cross-process idempotency, and
an uncertain remote outcome requires reconciliation before another key is used.
Serialized-state limits are not a measured heap/RSS ceiling. Attachment paths
reject traversal and observed symlinks before canonicalization; source APIs
still reopen paths, so this does not prove content immutability from preparation
through submission or eliminate filesystem races. Those remain explicit
closure dependencies.

## Normative references

- [MCP 2025-06-18 lifecycle](https://modelcontextprotocol.io/specification/2025-06-18/basic/lifecycle)
- [MCP 2025-06-18 transports](https://modelcontextprotocol.io/specification/2025-06-18/basic/transports)
- [MCP 2025-03-26 transports](https://modelcontextprotocol.io/specification/2025-03-26/basic/transports)
- [MCP 2025-06-18 tools](https://modelcontextprotocol.io/specification/2025-06-18/server/tools)
