# Remote MCP security and operations

Remote Alaveteli capabilities are local-only and disabled by default. The MCP
server accepts an explicit, credential-free HTTPS allowlist through
`FYI_MCP_REMOTE_INSTANCES`:

```text
FYI_MCP_REMOTE_INSTANCES=nz=https://foi-repository.example
FYI_MCP_REMOTE_READ=nz
# Leave unset until the governed-write track is complete.
FYI_MCP_REMOTE_WRITE=
```

The configuration is fail-closed. Malformed instance IDs, wildcard hosts,
credentials, non-HTTPS URLs, duplicate IDs, and zero budgets stop startup.
Read and write capabilities are independent; enabling read never enables
write. Remote budgets are bounded per instance for request count, response
bytes, and wall-clock duration.

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
constrain blast radius. Remote writes remain out of scope until the separate
confirmation, idempotency, and replay-protection track is complete.

Issue: https://github.com/edithatogo/fyi-cli/issues/172
