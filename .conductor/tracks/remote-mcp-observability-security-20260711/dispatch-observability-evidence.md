# MCP dispatch observability evidence

## Scope and original acceptance

This is a bounded #172 / #169 increment, prepared in the existing ordinary
ChatGPT workstream. It does not close either issue or the original Conductor
phase. The user-designated **FYI-CLI — Issue fixes and PR merges** coordinator
owns final reconciliation, exact-head checks/review, merge and pruning.

The existing per-instance policy, controls and result accounting are retained.
New behavior is confined to MCP dispatch telemetry and the read-only local
`fyi://remote/status` resource. Metrics have eight fixed operation rows and five
terminal outcome counters, fixed cumulative duration buckets, saturation and
process lifetime. Events/spans use server correlation and exclude caller data.
No additional backend, budget, journal or mandatory dependency is introduced.

The implementation base is #459's fixed
`94e52ab64dde827c891bc8619c18c3c1efd76357`, including #458's offline fixture repair.
During work, the coordinator verified and merged #458 as
`393241da3dd3124c969e2d3f4304b27a2cf8fd22` and refreshed #457 to
`c8150242d5306bfb1c410eaae9114a0de6fbe13b`. Neither published branch was changed
by this workstream. Source/runtime/CI/i18n owners' files remain outside the delta;
there is no Cargo manifest/lock, shared registry, source, db.rs or sync.rs change.

## Test-first implementation

The published RED checkpoint `3e6051bfcf04eb84fb161dde743ac29f64af3d81`
(local equivalent `c8925b0a79a1158051ea1f19efb574ea984d5c5d`)
compiled and executed **six failing assertions**, not a missing-API compilation
failure. The existing handler neither advertised nor served the status resource,
so success/failure/timeout/cancellation accounting was unavailable to callers.

The smallest dispatch guard was then added to the existing execution wrapper.
It observes the existing admission and publication decisions without authorizing
any I/O. The existing read-failure warning level also received an executed RED:
the actual SDK assertion rejected an INFO completion before the emitter was
corrected to retain WARN for all non-success terminal outcomes.

The full Rust run exposed parallel tracing callsite/subscriber interference in
the new thread-local capture test. The repaired sensor runs in a 30-second-bounded
subprocess with a process-wide subscriber, matching production startup. It retains
all six start/child/completion assertions, correlation equality/distinction,
async-yield propagation and privacy checks. No test, assertion, gate or threshold
was removed. An early verification attempt using a stale binary after a compile
error was stopped and excluded; final SDK/full-suite evidence uses the rebuilt,
fixed executable identified below.

Implementation `985a1dd151328d7e7c6e7c3d2454b2d7b9a16a4d` has full tree
`963bbdf89899d2c8af29c540ce50d573faafe1c7`, exactly matching the locally verified
implementation (`09c5756e6f2638fea25baba551d21e69de777a37`).

## Local verification, 2026-11-14 UTC

Environment: Linux x86_64, Rust **1.99.1**, Python **3.12.14**, locked MCP SDK
**1.30.0** and unchanged `uv.lock`.

| Sensor | Verified result |
| --- | --- |
| `cargo fmt --all -- --check` | Passed. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed; lint findings repaired without allowances. |
| `cargo test --workspace --all-features --locked` | **688 passed, zero failed, one existing ignored test.** |
| MCP Rust subset | **145 passed:** 139 unit/contracts plus six executable wire tests. Includes 12 new Rust tests. |
| Actual SDK observability module | **11 passed**, all eight current tools, one private source failure, a 12-example deterministic generated caller-value property, and restart/schema tests. |
| Guarded full Python suite | **1,563 passed, zero failed, two existing opt-in smoke skips**, 192.62 seconds. |
| Full-suite offline sensor | **Zero external Python DNS/socket attempts**, including swallowed-exception detection. |
| New Python Ruff/format, Conductor, remote-contract and layered-harness verifiers, diff whitespace | Passed. |

The fixed SDK executable SHA-256 is
`24f1f5b5f15682f3c0aa99ee1599b4ee5b1f8442c84ed6dfb5b7666379550f62`.
It is built from this increment, without the separately pending #457 journal.
The full suite includes #459's production header/body timeouts and connection
closure checks. The local guard is a Python DNS/socket sensor, **not an OS or
native-binary sandbox claim**. The TLS server is loopback-only, refuses other
CONNECT destinations, contains no forwarding code, and uses synthetic data.

All three targeted compiled mutants failed their corresponding behavioral
assertions (including the final isolated trace sensor): remove the async span, invert cancellation accounting,
and replace recorded result bytes with zero. Sources are restored byte-for-byte
after every mutant. This is focused sensor evidence, not a broad source mutation
score or a substitute for the source owner's guardrail mutations.

## Operator contract and remaining dependencies

See [the operator contract](../../../docs/mcp-dispatch-observability.md) for
the schema, cumulative histogram interpretation, warning levels, cancellation,
privacy, collector retention, reset, rollout and rollback boundaries. The
versioned schema is `tests/fixtures/remote_mcp/status-v1.schema.json`.

Prepare/reject/cache replay do not enter dispatch metrics. The existing write
journal remains responsible for durable governance events after #457 integration.
A successful dispatch is not a guarantee that later journal completion succeeds,
an upstream request is legally valid, or a write can safely be replayed.

Original acceptance still requires source-owned typed private outcomes, cache
provenance, shared raw request/byte/time allowances, pacing/cache/retry/throttle /
guardrail events and metrics, supported platform/runtime qualification and the
remaining track reviews. Metrics do not infer HTTP counts, bytes or status from
MCP payloads or private error strings. No full phase/track closure is claimed.

## Final-head handoff

Focused draft: [PR #460](https://github.com/edithatogo/fyi-cli/pull/460).

Hosted candidate checks and the sole coordinator's final integrated-head review
remain pending. Optional queued offline AI is not passed or required-review
evidence. The intended dependency order remains #458, coordinator #457/#459
integration, then this independent observability delta. The coordinator must
preserve `finish_durable` while adding the typed operation argument at the
`remote_commit_write` dispatch call; this workstream will supply any exact local
combination evidence separately without moving a published integration branch.
