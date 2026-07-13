# Hosted MCP deployment contract

The repository’s `fyi-mcp` binary currently speaks MCP JSON-RPC over stdio.
The root `Dockerfile` is therefore suitable for local/container inspection,
but it is not yet an HTTPS MCP Connector endpoint.

Before submitting to a hosted Connector directory, a deployment must provide:

- a standards-compliant HTTPS MCP transport, including authentication;
- `/healthz` and version metadata suitable for bounded probes;
- read-only capabilities enabled by default;
- explicit instance allowlisting and rate limiting;
- no credentials or write capabilities in the public deployment;
- privacy, retention, incident-response, and operator-contact documentation;
- OCI packaging so the same image can run on Cloud Run, Fly.io, Azure
  Container Apps, Kubernetes, or another OCI-compatible platform.

Do not advertise the current stdio image as a hosted remote connector until
these conditions are verified. The portable local distribution remains the
MCPB bundle and platform-specific release binaries.
