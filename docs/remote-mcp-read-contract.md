# Guarded remote MCP reads

Issue [#170](https://github.com/edithatogo/fyi-cli/issues/170) adds a complete read
contract around the existing `SyncClient`. Reads are disabled by default. Set an
explicit `FYI_MCP_REMOTE_INSTANCES` allowlist and enable its opaque instance IDs
through `FYI_MCP_REMOTE_READ`. Read enablement does not enable remote writes.
Credentials and arbitrary source URLs are never tool arguments.

Start with the local `fyi://remote/read-contract` resource for the current schemas
and recommended call order, then inspect `fyi://remote/status` for capabilities,
budgets, the circuit breaker and process-local dispatch counts. Reading these
resources does not contact a source or consume a remote allowance. The read
catalogue appears only when at least one configured instance enables reads;
every call also checks that its specific instance enables the capability.

## Tools and side effects

| Tool | Arguments after `instance_id` | Source effect | Local data effect |
| --- | --- | --- | --- |
| `remote_health` | None | One bounded GET to the request API; failures retain their typed status | None |
| `remote_version` | None | One version GET | None |
| `remote_search_requests` | `query`, optional `limit` (1–50; default 10) | One bounded search GET | None |
| `remote_get_request` | Positive `request_id` | One request GET; returned ID must match | None |
| `remote_list_authorities` | Optional `filter`, `limit` (1–200; default 50) | One authority-list GET; filtering and limit apply to the parsed result | None |
| `remote_discover_authority_feeds` | `authority_slug` | No source request; constructs the canonical `/feed/body/<slug>` route | None |
| `remote_plan_authority_feed` | `authority_slug`, optional `limit` (1–200; default 50) | One bounded RSS/Atom feed GET | None |

An authority slug contains 1–128 ASCII letters, digits, hyphens or underscores.
URL, path, query and fragment inputs are rejected before dispatch. First discover
an authority with `remote_list_authorities`, then inspect its canonical feed
location and request a bounded plan if needed. Discovery returns
`availability: "not_checked"`: it does not assert that the route exists or is
supported by that deployment.

A feed plan contains `feed_url`, `references`, `request_ids`, and `truncated`.
Each reference has a canonical `url`, a `kind` (`request` or `request_event`), and
a nullable `request_id`. Only the explicit API route
`/api/v2/request/<positive-id>.json` supplies a request ID. A `/request/<slug>`
route remains unresolved even if its slug consists entirely of digits, as does
a request-event route. For example:

```json
{
  "feed_url": "https://foi-repository.example/feed/body/synthetic-authority",
  "references": [
    {"kind": "request", "url": "https://foi-repository.example/request/budget_report", "request_id": null},
    {"kind": "request_event", "url": "https://foi-repository.example/request_event/123", "request_id": null}
  ],
  "request_ids": [],
  "truncated": false
}
```

Alaveteli's [authority feed route][upstream-routes] uses `/feed/body/<slug>`. Its
[Atom template][upstream-template] links events through `/request_event/<id>`;
the [event controller][upstream-events] redirects those links to their request
or message. The planner preserves those references without following the
redirect or mistaking an event ID for a request ID. These upstream shapes are
covered by synthetic fixtures; they are not captures of live FOI records.

The single item limit applies across unique canonical reference URLs. References
sort by URL; `request_ids` is the sorted, deduplicated projection of known request
IDs from the retained API references. Fragment anchors are removed. Relative links
resolve against the feed document URL, then must match the configured origin
and base path. Queries, credentials, foreign origins, encoded path segments and
extra route segments are not admitted. Atom alternate links and RSS item links
are considered; unrelated metadata does not authorize a source request.

Planning does not fetch request records, resolve event links, follow pagination,
call `pull_feed`, or open/write the application database. A bounded XML parser
rejects malformed structure, DTDs and XML base overrides; it does not claim full
Atom/RSS schema validation. An empty valid feed is an empty plan. `truncated`
means the document contained more eligible unique references than the limit;
it does not establish whether an upstream feed omits older records. Only known
request IDs can be passed directly to `remote_get_request`.

[upstream-routes]: https://github.com/mysociety/alaveteli/blob/00735fc4f26fe576bc2f2fe6faf096e47127ed3f/config/routes.rb
[upstream-template]: https://github.com/mysociety/alaveteli/blob/00735fc4f26fe576bc2f2fe6faf096e47127ed3f/app/views/track/atom_feed.atom.erb
[upstream-events]: https://github.com/mysociety/alaveteli/blob/00735fc4f26fe576bc2f2fe6faf096e47127ed3f/app/controllers/request_controller.rb

## Successful results and cache provenance

Existing result fields are retained. Every read now also returns `provenance`:

```json
{
  "instance_id": "synthetic",
  "retrieved_at": "2026-11-17T00:00:00.000Z",
  "cache_status": "miss",
  "correlation_id": "mcp-17",
  "source_status": 200,
  "response_bytes": 128
}
```

`retrieved_at` is the original representation retrieval time. A conditional GET
that receives HTTP 304 keeps that time, reports `cache_status: "revalidated"`
and `source_status: 304`, and counts only newly downloaded entity bytes
(normally zero). It is not represented as a new acquisition of the cached body.
A 304 without a matching cached representation/validator fails closed.

Feed discovery has no downloaded representation: its timestamp is the time the
local route plan was generated, `cache_status` is `not_applicable`, status is
`null`, and downloaded bytes are zero. Correlation IDs are generated by the
server and match the dispatch event/span; they are process-local, not caller IDs
or globally unique identifiers. Successful `structuredContent` and JSON text
content contain the same payload.

## Safe structured failures

Invalid JSON-RPC requests or arguments outside the advertised input schema keep
the existing protocol error behavior. A valid remote read that is denied by
policy, fails source validation or cannot complete returns `isError: true`, the
existing private text message, and this versioned structured content:

```json
{
  "error": {
    "schema_version": 1,
    "kind": "rate_limited",
    "operation": "remote_version",
    "retryable": true,
    "status": 429,
    "safe_message": "remote endpoint rate limited request",
    "correlation_id": "mcp-18"
  }
}
```

Kinds are `auth`, `forbidden`, `not_found`, `rate_limited`,
`upstream_unavailable`, `upstream_server`, `timeout`, `upstream_unknown`,
`validation`, `malformed_payload`, `guardrail`, `transport`, `cache`, `disabled`,
and `budget_exceeded`. Status is the actual observed HTTP status when available,
including nonstandard codes accepted by the HTTP library, otherwise `null`.
Source exception strings, response bodies, credentials, headers, URLs and caller
values are not copied into this envelope. Status and retryability come from
source error types, never from parsing error-message text.

A failure before dispatch has a null correlation ID and consumes no source
request. A transient read failure may be marked retryable, but the caller must
still respect the instance's remaining allowances, pacing and circuit breaker.
The server does not automatically retry these reads. Remote-write confirmations,
error text and durable replay behavior remain governed by their existing contract.

## Bounds and shared state

Each read-enabled instance has one source client shared across calls and cloned
policy handles. Its existing identity, SSRF validation, rate/back-pressure,
validator cache, concurrency and cumulative request/byte guardrails are reused.
The explicitly bounded read transport disables automatic redirects and retries,
so a single admitted source read sends at most one HTTP request.

Default MCP allowances are 20 admitted operations and 4 MiB of serialized
successful result payloads per instance, with one operation active at a time and
a 120-second deadline per operation. The shared read client separately receives
the same request maximum and a 4 MiB cumulative downloaded-byte allowance. Failed
source attempts still spend the source allowance. The effective per-read response
cap also respects the remaining MCP result allowance. Headers and streaming
chunks are checked before accumulation; a cached representation must fit the
current read cap before reuse. Result JSON, including provenance, is counted
again before publication; raw and serialized byte counts are distinct.

The deadline covers pacing, concurrency waits, transport and parsing. Late
synchronous parsing or serialization is rejected before result publication;
the cooperative timer does not forcibly interrupt that CPU work. The source
client retains its additional one-hour lifetime ceiling beginning at server
configuration. After that ceiling, or exhaustion of the cumulative allowances,
restart with reviewed operator configuration to start a new session. A restart
resets read cache, budgets and transient circuit state; it does not clear durable
write reservations. These byte bounds constrain the response/cache data retained
by this client; they are not a measurement or guarantee of whole-process peak RSS.

## Compatibility and verification

MCP protocol profiles remain `2025-06-18` and `2025-03-26`. The current catalogue
fixture is `tools-v2.json`, structured exchanges are `v3.json`, upstream cases are
`upstream-v2.json`, and status/metrics use schema version 2 with ten fixed tool
rows. Historical v1/v2 exchange, tool and status fixtures remain regression
fixtures. Clients that validate successful read results against a pinned old
schema should migrate to the current catalogue before requiring exact fields.
Local tool contracts and remote write payload schemas are unchanged.

The Rust source tests use WireMock/local fixtures; the Python SDK harness runs
the actual production binary against a local TLS fixture and records exact GET
counts, typed faults, cache provenance, feed parsing, limits and non-mutation.
No live deployment is certified by these tests. Platform and full CI execution
must be evidenced separately in the existing Conductor track.

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-features --locked
python scripts/verify_remote_mcp_contract.py
python -m pytest -q tests/test_remote_mcp_contract.py tests/test_mcp_client_interop.py tests/test_mcp_upstream_contract.py tests/test_mcp_observability.py
python scripts/verify_frontier_conductor.py
```
