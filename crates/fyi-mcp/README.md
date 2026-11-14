# FYI MCP

Model Context Protocol server for FYI request management.

[![smithery badge](https://smithery.ai/badge/edithatogo/fyi-mcp)](https://smithery.ai/servers/edithatogo/fyi-mcp)

## Registry

- mcp-name: `io.github.edithatogo/fyi-mcp`

## Build

```bash
cargo build --release --package fyi-mcp
```

## Run

```bash
cargo run --package fyi-mcp
```

Remote capabilities are disabled by default. See
[`docs/remote-mcp-security.md`](../../docs/remote-mcp-security.md) before
enabling an explicit instance allowlist. Read and write enablement are
independent. A write-enabled instance exposes a prepare/commit confirmation
flow with process-local replay protection; enabling read does not enable writes.

## Optional HTTP transport

Desktop clients continue to use stdio by default. For a stateless HTTP JSON-RPC
deployment, set `FYI_MCP_TRANSPORT=http` and provide
`FYI_MCP_HTTP_BEARER_TOKEN`; the listener defaults to `0.0.0.0:8080` and can
be changed with `FYI_MCP_HTTP_ADDR`. The service exposes `/healthz` and `/mcp`.
The HTTP endpoint requires an `Accept` header listing both
`application/json` and `text/event-stream`, accepts protocol versions
`2025-06-18` and `2025-03-26`, and returns `405 Method Not Allowed` for
`GET /mcp` because this implementation does not provide an SSE stream.
Notifications return `202 Accepted` without a body.
Initialization negotiates the requested supported revision, or offers
`2025-06-18` when the client requests another revision. The `2025-03-26`
profile accepts batches of up to 64 messages; `2025-06-18` accepts single
messages. Stdio frames and HTTP bodies are limited to 1 MiB. An oversized
stdio frame produces an error and closes the stream without draining it.

This HTTP authentication profile is for native or server clients with a
preconfigured bearer token. Requests bearing an `Origin` header are rejected
with HTTP 403. Missing or invalid authentication is rejected before the body
is parsed. This profile does not implement OAuth discovery or browser access.
Terminate TLS at a trusted reverse proxy or platform ingress; do not expose
the bearer-token listener directly to the public internet. HTTP authentication
does not grant remote capability: the instance policy and confirmation gate
still apply to each write.

## Tool contracts

Every registered tool publishes an input schema, output schema, and annotations.
Arguments are validated before dispatch, including unknown fields, types,
required fields and bounded values. Remote writes advertise their mutating
behaviour and separate preparation from confirmation-only commit arguments.

Malformed tool parameters return JSON-RPC `-32602`. A valid call that fails
execution or capability/confirmation checks returns an MCP tool result with
`isError: true`. Successful results contain matching `structuredContent` and
JSON text content. Raw database/parser diagnostics and rejected caller values
are excluded from error messages.

The versioned catalogue is captured from the executable and compared in Rust
and official Python MCP SDK tests. Run the existing suites with:

```bash
cargo test -p fyi-mcp --locked
python -m pytest -q tests/test_remote_mcp_contract.py tests/test_mcp_client_interop.py
python scripts/verify_remote_mcp_contract.py
```

The SDK suite builds the current executable once when Cargo is available.
`FYI_MCP_TEST_BINARY` selects an explicitly built executable. Only a missing
toolchain and missing binary can skip interoperability tests; a build or client
failure fails the suite. All scenarios use synthetic data and local transports.

See [MCP contract evidence](../../docs/mcp-contract-harness.md) for supported
client profiles, deterministic tests, and limits on release-readiness claims.

## Glama

The repository root contains the Dockerfile used for Glama release builds. It
builds the `fyi-mcp` binary and starts it over stdio with an ephemeral SQLite
database so registry inspection can list and call tools without external state.

See [../../GLAMA.md](../../GLAMA.md) for the release checklist.
