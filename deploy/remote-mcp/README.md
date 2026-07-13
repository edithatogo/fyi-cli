# Hosted MCP deployment contract

The repository’s `fyi-mcp` binary speaks MCP JSON-RPC over stdio by default and
now has an opt-in stateless HTTP JSON-RPC mode. Set
`FYI_MCP_TRANSPORT=http`, `FYI_MCP_HTTP_BEARER_TOKEN`, and optionally
`FYI_MCP_HTTP_ADDR` to enable it. The root `Dockerfile` remains safe for local
or container deployment, but TLS must be provided by the hosting platform or a
trusted reverse proxy.

Before submitting to a hosted Connector directory, a deployment must provide:

- a standards-compliant HTTPS MCP transport, including authentication;
- `/healthz` and version metadata suitable for bounded probes;
- read-only capabilities enabled by default;
- explicit instance allowlisting and rate limiting;
- no credentials or write capabilities in the public deployment;
- privacy, retention, incident-response, and operator-contact documentation;
- OCI packaging so the same image can run on Cloud Run, Fly.io, Azure
  Container Apps, Kubernetes, or another OCI-compatible platform.

The HTTP listener is intentionally stateless and bearer-token protected. It
implements the JSON response form of Streamable HTTP, validates the required
dual `Accept` media types, accepts the current `2025-06-18` and fallback
`2025-03-26` protocol versions, returns `202 Accepted` for notifications, and
returns `405 Method Not Allowed` for `GET /mcp` because it does not offer an SSE
stream. Session management, resumability, and OAuth protected-resource
metadata are not implemented; a deployment targeting clients that require
those features must add and verify them before claiming full Connector
compatibility.

Do not advertise the current stdio image as a hosted remote connector until
these conditions are verified. The portable local distribution remains the
MCPB bundle and platform-specific release binaries.
