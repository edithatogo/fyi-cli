# Remote MCP dispatch observability, version 1

This is the MCP-owned portion of [#172](https://github.com/edithatogo/fyi-cli/issues/172)
and [#169](https://github.com/edithatogo/fyi-cli/issues/169). It uses the existing
`tracing` subscriber and policy object. It adds no exporter, backend dependency,
budget engine, remote endpoint, request persistence or control-changing tool.
Durable write-journal behavior belongs to the separately reviewed
[#457](https://github.com/edithatogo/fyi-cli/pull/457); references to that journal
below describe the combined deployment after the coordinator integrates it.

## Local status resource

`resources/list` advertises `fyi://remote/status`. Reading it returns one
`application/json` content item with `schema_version: 1`, `policy` and `metrics`.
The offline contract is
[`status-v1.schema.json`](../tests/fixtures/remote_mcp/status-v1.schema.json).
The resource is available in local-only, degraded, kill-switch, circuit-open and
exhausted-budget states. Reading does not contact an upstream, consume an
allowance, reset counters or grant capabilities. HTTP transport retains its
existing authentication boundary; stdio access follows access to that process.

`policy` is the existing version-1 policy snapshot: configured instance IDs,
read/write enablement, control flags, budgets and recorded usage. It excludes
base URLs and protected credentials. Use opaque operator-chosen instance IDs;
the status resource is not intended to store personal information in identifiers.

`metrics` is a consistent snapshot of a fixed eight-row in-memory array, one row
for each current remote tool. Its scope is `process` and its measurement is
`mcp_admitted_operations`. Policy and metrics are sampled under separate locks;
their two snapshots are not an atomic cross-component transaction during
concurrent work. A policy clone shares the same counters. Process restart
resets them; neither the request database nor the separate durable-write journal
is queried or changed by this resource.

### Measurement contract

| Field | Meaning |
| --- | --- |
| `operation` | One of the eight fixed remote tool names; never caller-created. |
| `started` | Operations entering the existing admitted dispatch wrapper after policy/confirmation admission. |
| `in_flight` | Such operations that have not reached a terminal outcome. |
| `outcomes.success` | An operation returned a payload that passed the existing publication checks. It does not assert that a health payload says the upstream is healthy. |
| `outcomes.failed` | The source future returned an error. Source status, retryability and failure kind are not guessed from its private string. |
| `outcomes.timed_out` | The MCP operation timeout expired. A source client's own timeout currently returns a coarse `failed` outcome. |
| `outcomes.result_rejected` | Serialization or an existing post-operation publication/budget/control check rejected the result. |
| `outcomes.cancelled` | The observed future was dropped before finishing, including task cancellation or unwinding. This is not proof that an upstream write did not occur. |
| `serialized_result_bytes` | UTF-8 JSON payload bytes measured before publication, including a payload rejected by the existing result budget. Failed/timed-out/cancelled futures with no payload contribute zero. |
| `duration_sum_ms` | Sum of completed dispatch durations in whole milliseconds, using a monotonic clock. |
| `duration_bucket_counts` | Seven cumulative duration buckets: `<=10`, `<=100`, `<=1000`, `<=10000`, `<=60000`, `<=120000` ms, then all durations (`+infinity`). The six finite bounds are also returned explicitly. |

Counters saturate at `u64::MAX`; they never wrap. Milliseconds truncate
sub-millisecond fractions. No percentiles are inferred from these coarse
buckets. A telemetry-state failure rejects dispatch or status export with a
fixed private error; it never silently replaces missing measurements with zero.

Preparing a write, rejecting arguments or confirmation, and returning an
already-completed cached result do not enter the dispatch wrapper. They therefore
do not create dispatch metrics or events. A policy reservation can precede a
gate/journal failure before the wrapper is entered: the existing policy allowance
is not refunded, and the dispatch counters need not equal all policy reservations.
The separate write journal owns durable prepare/commit/reject/replay events.

These values **do not measure** raw HTTP attempts, retries, downloaded bytes,
peak memory, cache hits, pacing/throttling or source guardrail trips. One admitted
MCP operation can perform several HTTP exchanges. The existing source transport
continues to enforce its separate limits. Successful MCP dispatch also precedes
any later write-result journal completion; journal failure still prevents a
successful tool response and leaves the durable safety boundary intact.

## Events and async spans

Each admitted dispatch emits `started` and one `completed` event to target
`fyi_mcp::dispatch`. Existing diagnostics remain on stderr, leaving MCP stdout
for JSON-RPC. Starts and successes use INFO; other terminal outcomes use WARN,
preserving the existing read-failure warning level. The default INFO filter
includes the correlation spans; filtering those spans out also removes inherited
child context. Completion events still carry explicit correlation fields.
The `mcp_remote_dispatch` span surrounds polling of the complete
source future, including async yields; a child event on that future inherits its
server correlation. Independent tasks spawned outside the instrumented future
must explicitly propagate context if a future source implementation needs them.

| Event field | Version-1 values |
| --- | --- |
| `schema_version` | `1` |
| `event` | `started` or `completed` |
| `correlation_id` | Server-generated `mcp-<integer>`, process-scoped; never the JSON-RPC caller ID. |
| `operation` | One of the eight fixed tool names. |
| `outcome` | Completion only: `success`, `failed`, `timed_out`, `result_rejected`, `cancelled`. |
| `duration_ms` | Completion only: monotonic elapsed milliseconds. |
| `serialized_result_bytes` | Completion only: measured MCP payload bytes. |

No instance ID, URL, request ID, query, body, attachment path/content, token,
idempotency key, raw exception or response text is passed to these events or
spans. Metric dimensions are limited to the eight operations and five outcome
counters. Correlation IDs are event fields, **never metric labels**. The legacy
policy `audit_event` token-normalizer is not used to sanitize dispatch events;
excluding private data and using enums provides this boundary.

The default subscriber produces readable structured fields. An operator can
collect those fields through the existing logging setup; this change does not
add a network exporter or an automatically retained event file. A hard process
kill cannot run Rust destructors and may leave only the start event. Metrics are
process-local diagnostics, not a durable or tamper-proof audit record, upstream
idempotency proof, or exactly-once guarantee.

## Privacy, retention and incident handling

Keep stderr collection local to the approved operator environment. If logs are
retained, configure access controls, rotation and a deletion period in the
collector before enabling persistent collection. No universally appropriate
legal retention period is asserted here. Include the process/run identity at
the collector so the process-scoped correlation sequence is not mistaken for a
globally unique identifier; do not use personal paths or credentials for that
identity.

For a suspected incident, read the local status resource to inspect current
capabilities, circuit state and usage. Follow the existing
[disable/degraded runbook](remote-mcp-security.md#operator-controls) to stop new
remote writes or the process. A status read does not change operator controls.
Preserve only approved redacted events and compare a dispatch's start/completion
under its process correlation. Investigate uncertain writes through the separate
write journal and upstream procedures; cancellation or timeout is not permission
to replay a write. Restarts reset these diagnostic counters and transient policy
state, so retain any required safe snapshot before restarting.

Rollback removes the resource and process-local metrics; it neither removes nor
resets the durable journal. Clients should treat an absent resource as unsupported,
not as a report of zero operations. No application request data is migrated.

## Verification and remaining original acceptance

Rust contracts cover discoverability, disabled/control-state reads, no allowance
changes, UTF-8 byte counts, fixed terminal outcomes, cancellation, async span
propagation, privacy, histogram boundaries, saturation, clone/concurrency behavior
and poisoned-telemetry failure. The actual SDK tests use the existing non-forwarding
loopback TLS fixture and unchanged production `SyncClient`, covering all eight
tools, private source failure, prepare/reject/cache no-dispatch behavior,
restart reset, schema strictness and twelve deterministic generated caller values.
The local TLS trust fixture remains explicitly Linux-qualified.

This increment does not complete #172's original correlated source observability.
Source-owned typed outcomes and cache provenance, raw allowance sharing, retry /
throttle / cache / guardrail events, bounded source metrics, and runtime/platform
qualification remain required. Keep the existing Conductor track and #169/#172
open. The sole integration coordinator must reconcile the separate journal and
offline-harness candidates and verify all required checks on the eventual final
integrated head; a local candidate or temporary combined tree is not that approval.
