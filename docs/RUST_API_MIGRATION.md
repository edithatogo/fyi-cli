# Rust API migration guide

The Rust `fyi-core::SyncClient` implements request, authority, feed, health and
write methods. Synthetic tests verify their current client shapes; they do not
establish stock Alaveteli or deployed-instance compatibility. The
[coverage contract](api-contract-inventory.md) records the pinned upstream
differences and open account-journey work.

## Capability mapping

| Capability | Rust method | Notes |
| --- | --- | --- |
| Pinned-upstream authority request read | `fetch_upstream_authority_request_read` / `RequestEvidenceClient::fetch_upstream_authority_request` | Explicit authority key and bounded no-redirect transport; CLI receipt integration and socket-free tests; MCP and deployed-source qualification remain open. |
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

Older endpoint tests use WireMock fixtures; the pinned authority read uses an
injected socket-free transport. Live-instance smoke tests remain
opt-in; CI is the authoritative Rust execution environment on workstations
without the Windows SDK linker libraries.

The existing `fetch-request-page` command selects the pinned authority profile
with `--authority-api` and requires `--receipt`, `--raw-file` and
`--result-file` together. Supply `FYI_ALAVETELI_AUTHORITY_KEY` through the
operator's protected environment, not a command-line argument. The receipt
redacts the query and verifies hashes of the exact response and normalized
result. The raw authority response can contain personal information; its
retention and access require the operator's private evidence policy. Unix
evidence files are created with mode `0600`; Windows ACLs are not qualified by
the local socket-free tests. The current tests cover the source/receipt path
and CLI preflight; they do not establish a successful CLI-to-deployment run.

The CLI preserves human-readable sync errors in text mode and emits
`{"error":{"kind":"sync","message":"..."}}` in JSON mode. MCP also exposes
explicitly enabled remote read tools and separately governed write tools;
their synthetic source contracts do not qualify a live instance.

See [ALAVETELI_CLIENT.md](./ALAVETELI_CLIENT.md) for the existing Python
client examples and [api-contract-inventory.md](./api-contract-inventory.md)
for contract coverage and residual risk.
