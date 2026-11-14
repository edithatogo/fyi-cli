# Remote MCP security and operations

Remote Alaveteli capabilities are local-only and disabled by default. The MCP
server accepts an explicit, credential-free HTTPS allowlist through
`FYI_MCP_REMOTE_INSTANCES`:

```text
FYI_MCP_REMOTE_INSTANCES=nz=https://foi-repository.example
FYI_MCP_REMOTE_READ=nz
# Leave unset unless governed writes are explicitly approved.
FYI_MCP_REMOTE_WRITE=
```

The configuration is fail-closed. Malformed instance IDs, wildcard hosts,
credentials, non-HTTPS URLs, duplicate IDs, and zero budgets stop startup.
Read and write capabilities are independent; enabling read never enables
write. Remote budgets are bounded per instance for request count, response
bytes, and wall-clock duration.

The MCP request counter measures admitted tool operations, including failures;
raw HTTP retries within an operation belong to the source transport's separate
limits. One operation may be active per instance. The response budget measures
serialized MCP result bytes before publication, and does not establish a raw
download or heap allocation bound.

## Operator controls

The policy boundary exposes a kill switch, degraded read-only mode, and a
three-failure circuit breaker with deterministic recovery. Operators should
disable the process or clear `FYI_MCP_REMOTE_INSTANCES` before rotating
credentials or investigating an upstream incident. Restarting the local MCP
process resets transient circuit and budget state; durable request data stays
in the local SQLite database.

## Privacy and retention

Remote audit events contain schema version, bounded correlation/operation
tokens, instance ID, outcome, and a coarse error class only. They must never
contain credentials, authorization headers, URLs with query strings, request
bodies, attachments, PII, or upstream response content. Retain events only as
long as required by the operator's incident-response policy and delete them
when that period expires.

## Threat model

The boundary assumes an untrusted MCP caller, hostile or malformed upstream
responses, accidental operator misconfiguration, and a compromised local
workspace. The allowlist and existing `SyncClient` SSRF checks constrain
destinations; resource-aware pacing and budgets constrain load; redacted
structured events constrain disclosure; kill switch and circuit breaker
constrain blast radius. Remote writes require the separate confirmation,
idempotency, and replay-protection flow. They also require a protected
per-instance bot token (`FYI_MCP_REMOTE_BOT_TOKEN_<INSTANCE_ID>`) when the
upstream instance needs one; token values are never included in responses or
telemetry.

Attachment writes additionally require `FYI_MCP_ATTACHMENT_ROOT`. Paths are
relative to that canonical directory; symlinks, traversal, unsupported MIME
types, more than eight files, files over 50 MiB, and aggregate payloads over
100 MiB are rejected before any remote request is sent.

The confirmation ledger is bounded and process-local: unused tokens expire in
300 seconds; accepted tokens bind the exact preparation; invalid tokens do not
consume dispatch allowance. Completed/uncertain writes are retained until
process exit, with 128 total entries and 64 KiB serialized preparation/result
limits. Completed replay does not spend another request allowance. Restarting
clears this protection; it is not a durable remote idempotency guarantee.

Attachment path checks reject observed symlinks before canonicalization.
Because the source API subsequently reopens paths, content changes between
preparation and submission and filesystem races remain unverified. The MCP
contract track records this boundary as a closure dependency.

Issue: https://github.com/edithatogo/fyi-cli/issues/172
