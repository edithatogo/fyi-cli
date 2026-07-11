# Rust API migration guide

The Rust `fyi-core::SyncClient` now covers the primary Alaveteli request,
authority, feed, health, and write surfaces. Existing JSON callers can keep
using the original methods; the additions are opt-in methods on the same
client.

## Capability mapping

| Capability | Rust method | Notes |
| --- | --- | --- |
| Search requests | `search_requests_with_options` | Supports query, pagination, sorting, and filtering. |
| Search feed IDs | `search_request_ids_from_feed` | Returns deduplicated request IDs; fetch full records separately. |
| Add correspondence | `add_correspondence` | JSON payload for requests without files. |
| Add correspondence with files | `add_correspondence_with_attachments` | Multipart parts named `attachment_<index>`; 50 MiB per-file limit. |
| Update request state | `update_request_state` | Rejects unsupported state values before network I/O. |
| Compare-and-update state | `update_request_state_if_current` | Fetches the current state and refuses stale updates before PUT. |
| Authority discovery | `list_authorities` / `list_authorities_matching` | Matching is deterministic and case-insensitive across identifying fields. |
| Prefilled requests | `build_prefilled_url` | Encodes title, body, and optional tags safely. |
| Authority feeds | `pull_authority_feed` | Accepts an authority path and appends `/feed`. |
| Health and version | `health_check` / `get_api_version` | Read-only probes with guarded network execution. |

## State and compatibility notes

Supported write states are `waiting_response`, `rejected`, `successful`, and
`partially_successful`. The client validates this vocabulary but does not
invent a graph-level transition policy because the current update payload does
not include the remote request's current state.

All endpoint tests use WireMock fixtures. Live-instance smoke tests remain
opt-in; CI is the authoritative Rust execution environment on workstations
without the Windows SDK linker libraries.

The CLI preserves human-readable sync errors in text mode and emits
`{"error":{"kind":"sync","message":"..."}}` in JSON mode. The MCP server
currently exposes local SQLite tools rather than remote SyncClient operations,
so it has no remote API error presentation boundary.

See [ALAVETELI_CLIENT.md](./ALAVETELI_CLIENT.md) for the existing Python
client examples and [api-contract-inventory.md](./api-contract-inventory.md)
for contract coverage and residual risk.
