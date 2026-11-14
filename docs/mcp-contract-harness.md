# MCP product and contract harness

This document records executable evidence for the existing
`remote-mcp-contract-harness-20260711` track. All examples and tests use
synthetic local state. No CI probe sends a request to a live authority.

## Supported wire profiles

| Profile | Behaviour | Automated sensor |
| --- | --- | --- |
| stdio, MCP 2025-06-18 | Newline-delimited UTF-8 JSON-RPC; individual messages; initialization, ping, discovery and calls | `crates/fyi-mcp/tests/wire_contract.rs`; official SDK integration |
| stdio, MCP 2025-03-26 | Negotiated legacy revision; batches limited to 64 messages, with no notification replies | Same executable test suite |
| HTTP, MCP 2025-06-18 | Stateless POST with JSON responses; notification/response 202; authenticated GET 405 | `crates/fyi-mcp/src/transport_tests.rs`; official SDK integration |
| HTTP, MCP 2025-03-26 | Explicit header or missing-header fallback; bounded legacy batches | Same HTTP tests |
| Other initialization version | Offers supported 2025-06-18; client decides whether to continue | Executable negotiation test |
| Other HTTP version header | HTTP 400; no dispatch | HTTP negative tests |

These profiles do not establish support for later MCP revisions. Optional
SSE streaming, resumable sessions, and server-initiated requests are absent.
The HTTP bearer profile is a single-operator deployment, not OAuth discovery
or tenant isolation. Browser `Origin` headers are denied; bearer authentication
precedes payload parsing. TLS remains the deployment ingress's responsibility.

## Wire and privacy contracts

- JSON-RPC requests use `jsonrpc: "2.0"` and a string or integer ID.
- Notifications produce no response and cannot execute `tools/call` writes.
- Malformed JSON returns `-32700`; an invalid envelope returns `-32600`;
  malformed method parameters return `-32602`; unknown methods return `-32601`.
- Parse and envelope errors use `id: null`; valid request IDs are echoed.
- Known tool execution, capability and confirmation failures return a tool
  result with `isError: true`. Unknown tools and malformed arguments return
  `-32602`; these are distinct from failures inside a valid tool call.
- Errors do not echo rejected input, database paths, secrets, SQL errors or
  parser diagnostics. Synthetic database triggers/views and caller markers
  exercise the tool, resource, prompt and date-error boundaries.
- Stdio input and HTTP request bodies are capped at 1 MiB. Oversized stdio
  closes after one safe error, preventing an unbounded drain of attacker input.
- Stdout contains protocol messages only; diagnostics go to stderr.

## Verification

```bash
cargo test -p fyi-mcp --locked
cargo clippy -p fyi-mcp --all-targets -- -D warnings
cargo fmt --all -- --check
python scripts/verify_remote_mcp_contract.py
python -m pytest -q tests/test_remote_mcp_contract.py tests/test_mcp_client_interop.py
```

`tests/test_mcp_client_interop.py` uses the official Python SDK's stdio and
Streamable HTTP transports. It exercises initialization, ping, discovery,
successful local operations, output validation, private failures and governed
write preparation/rejection. It checks the default SDK negotiation and explicit
March/June revisions. Default negotiation uses SDK defaults; explicit revision
cases select the SDK's advertised version constant while retaining its session
and transport implementation. No successful remote commit or live authority
call occurs.

When Cargo is available the SDK fixture builds current source once. An explicit
`FYI_MCP_TEST_BINARY` selects the intended executable; a failed build or session
cannot silently skip. Outside GitHub Actions the test can skip only when both
Cargo and a binary are absent; GitHub Actions fails on that missing prerequisite.
The existing unfiltered Python Tests job invokes the SDK suite without a new
workflow. The Layered Harness Contract job validates fixtures separately and
does not itself establish client interoperability.

### Degraded stdio startup and recovery

`FYI_MCP_REMOTE_DEGRADED=true` or `1` activates the existing degraded policy at
subprocess startup. `false`, `0`, or an absent setting selects normal mode;
other values stop startup with a private configuration error. Recovery requires
a new subprocess and preserves the separate configured capability restrictions.

The existing SDK harness now exercises normal → degraded → recovered launches
for both boolean spellings. It verifies all three write preparations are denied
in degraded mode despite direct calls to hidden tools, remote read definitions
remain available, local health/list/ping still work, and new preparations recover
with fresh tokens. Invalid commits remain denied throughout. The read-only
scenarios also verify that clearing degraded mode cannot enable writes.
Five invalid-setting subprocess cases fail before creating a database, without
protocol output, rejected values or private paths in diagnostics. Time bounds
remain five seconds per SDK request, thirty seconds per SDK session, and five
seconds per invalid-startup subprocess. These scenarios perform no source-network
reads or submissions; valid preparations stay local and direct invalid commits
receive the specific policy or confirmation denial before dispatch.

```bash
python -m pytest -q tests/test_mcp_client_interop.py -k 'degraded or read_only'
```

At base `39591dcf91bb50ed847b0d36bdc3bd670a7e60a7`, the seven new regression
cases failed against the unchanged executable. With the startup control wired,
the full MCP SDK/fixture suite passes 151 cases: 22 SDK scenarios and 129 verifier
cases. The Rust MCP suite passes 124 unit/boundary tests and five executable
wire tests. Linux SDK 1.30.0 evidence does not establish other platform runs or
hot switching within a running subprocess.

## Versioned schema and exchange fixtures

| Artifact | Contract and evidence |
| --- | --- |
| `tests/fixtures/remote_mcp/v1.json` | Unchanged historical inventory and old custom error envelopes; not the current wire contract |
| `tests/fixtures/remote_mcp/tools-v1.json` | Full 23-tool executable catalogue, including descriptions, input/output schemas and annotations |
| `tests/fixtures/remote_mcp/tools-v1.schema.json` | Snapshot structure and version contract |
| `tests/fixtures/remote_mcp/v2.json` | 23 current success/error exchanges; each labels executed-handler or synthetic-output evidence |
| `tests/fixtures/remote_mcp/v2.schema.json` | Current exchange structure and version contract |

The existing verifier checks strict JSON, duplicate keys/names, versions,
complete case coverage, input/output schemas, error classification and matching
text/structured content. It rejects unsupported versions, external schema
references and invalid fixtures. The eight handler examples are replayed
through actual JSON-RPC dispatch; all eight remote tools have synthetic success
examples checked against output schemas. Synthetic read/commit outputs do not
claim source-network success.

Rust catalogue equality and actual SDK `tools/list` comparison detect drift.
Tool order and object-key order do not matter; schema arrays, values, additions,
removals, descriptions and annotations do. A captured JSON-RPC `tools/list`
response can also be checked with:

```bash
python scripts/verify_remote_mcp_contract.py --tools-result captured-tools-list.json
```

Capture all tools using an ephemeral process with the synthetic instance
`synthetic=https://mcp-fixture.invalid` enabled for read and write, and call only
`tools/list`. The verifier never rewrites snapshots. Review intentional drift;
new incompatible contracts require a new fixture version with explicit migration
evidence while retaining historical versions. Unsupported HTTP versions fail
before dispatch; initialization offers a supported revision for the client to
accept or reject.

Neither supported MCP profile is currently designated deprecated. Historical
fixture v1 remains regression-only evidence, not an advertised wire profile.
No profile-withdrawal or contract-migration scenario has been executed; that
part of the original deprecation acceptance criterion remains open. Unknown
version rejection and negotiation fallback do not establish migration behavior.

Input validation implements only the embedded schema vocabulary and fails
closed on unknown assertions, including unused branches. Its bounded recursion,
node count and shared branch budget run before side effects. Integer handling
preserves exact parsed integer bounds, accepts lossless integral JSON numbers,
and never coerces strings or booleans. `format` remains an annotation; date
parsing is a separate handler check. Remote read semantic checks occur before
request admission and do not count as upstream circuit failures. Deadline day
counts and capability scope
lists have explicit computation bounds. A 20-example Hypothesis test sends
generated invalid arguments through the real SDK and verifies unchanged local
state and subsequent ping responsiveness.

The 1 MiB and 64-message input limits are deterministic input bounds. They do
not independently establish a process memory limit. Cross-platform evidence,
source fault injection and shared source fixture versioning remain explicit
track dependencies.

## Remote admission and confirmation contract

Each configured instance admits at most one active MCP remote operation. The
request allowance counts admitted operations, including failures and timeouts;
it does not count each internal HTTP retry or request inside an operation.
The duration budget covers the complete admitted future. Returned payload bytes
are counted before publication, including rejected oversized results. Failure,
timeout and cancellation release concurrency without refunding the attempt.
These are MCP boundary limits; raw download/retry/allocator limits require
separate source-adapter evidence.

Confirmation tokens are opaque OS-generated random capabilities. Unused tokens
expire after 300 seconds using a monotonic clock. They bind an instance,
operation, idempotency key and immutable preparation. Commit arguments contain
only `instance_id`, `phase`, `confirmation_token` and `idempotency_key`; changing
the payload requires a new preparation. Invalid, expired and scope-mismatched
tokens consume no operation allowance. Concurrency/budget rejection leaves a
valid pending confirmation intact.

The process-local replay ledger retains at most 128 entries. Each preparation
and cached result is limited to 64 KiB of serialized JSON. Unused expired
confirmations can be reclaimed; attempted writes are retained even when their
remote outcome is uncertain. Completed results remain retrievable without a
fresh dispatch allowance, subject to current capability and operator controls.
The ledger and raw payloads are excluded from diagnostic formatting.

Restarting loses the ledger. This is not durable, cross-process idempotency, and
an uncertain remote outcome requires reconciliation before another key is used.
Serialized-state limits are not a measured heap/RSS ceiling. Attachment paths
reject traversal and observed symlinks before canonicalization; source APIs
still reopen paths, so this does not prove content immutability from preparation
through submission or eliminate filesystem races. Those remain explicit
closure dependencies.

## Measured local latency and memory

The existing SDK test fixture measures ten warmup calls followed by 100
sequential `check_status` calls over one persistent HTTP session. Timing includes
client serialization, loopback transport, database work and SDK output validation.
Linux memory is the server process's `/proc/PID/status` RSS and lifetime high
water mark; the Python client is excluded. Each run emits its raw samples,
executable hash and runtime metadata into pytest's temporary directory, subject
to the existing retention policy. A machine-readable summary is also printed.

```bash
python -m pytest -q -s tests/test_mcp_client_interop.py -m performance
```

Observed on 2026-11-14, Linux x86_64, Rust 1.99.1 debug build, Python 3.12.14,
official MCP SDK 1.30.0, empty ephemeral SQLite:

| Metric | Observed | Enforced fixture budget |
| --- | ---: | ---: |
| Median latency | 3.612 ms | Recorded |
| p95 latency, nearest rank | 4.154 ms | 250 ms |
| Maximum latency | 6.007 ms | Recorded |
| RSS after warmup | 20,254,720 bytes | Recorded |
| RSS / lifetime peak after measurements | 20,291,584 bytes | 128 MiB peak |
| RSS growth after warmup | 36,864 bytes | 16 MiB |

Measured executable SHA-256:
`73fcc8bb5bc72b88b1cba74241ee716cce01e9a37fac4090dcee4b370468d30a`.
These conservative regression budgets are for this local fixture. They do not
establish remote latency, populated-database performance, allocator ceilings,
release-build results or production-load capacity. Non-Linux runs still assess
latency and explicitly report unavailable `/proc` memory evidence.

## Targeted mutation execution

`cargo-mutants 27.1.0` tested the PR #441 policy/confirmation implementation at
`2b4edb2eb060c31a24f9ebe65cc30bcde18ddc8e` with Rust 1.99.1. The unmutated
baseline passed 51 selected tests. Of 40 selected mutations, 39 were caught,
none survived or timed out, and one did not compile (`Duration` subtraction
changed to division). This measures the selected security predicates only.

| Predicate family | Caught | Unviable |
| --- | ---: | ---: |
| Controls, admission and operation budgets | 15 | 0 |
| Confirmation token matching and scope | 18 | 0 |
| Confirmation expiry and remaining lifetime | 6 | 1 |

The run used an isolated source snapshot and build cache, two build jobs, two
test threads, locked/offline dependencies, a 15-second test timeout and a
180-second build timeout. It completed in 85 seconds and source restoration
was verified. Reproduce from that implementation revision with the existing
tool, supplying a disposable checkout and output directory:

```bash
cargo mutants --dir /path/to/disposable-checkout --in-place -p fyi-mcp \
  -f crates/fyi-mcp/src/write_gate.rs -f crates/fyi-mcp/src/policy.rs \
  --re '^crates/fyi-mcp/src/(write_gate\.rs:(298|302|303|304|312|414|415|429|430|433|434):|policy\.rs:(166|172|178|344|493|498|512|525|527):)' \
  --no-shuffle --cap-lints=false --timeout=15 --build-timeout=180 \
  --output /path/to/mutation-results \
  --cargo-arg=--locked --cargo-arg=--offline --cargo-arg=--bin=fyi-mcp \
  --cargo-test-arg=-- --cargo-test-arg=--test-threads=2 \
  --cargo-test-arg=policy::tests:: --cargo-test-arg=write_gate::tests::
```

Set `CARGO_BUILD_JOBS=2` and an isolated `CARGO_TARGET_DIR`. The line selector
is revision-specific and must be reviewed when those files change. The
repository's older root mutation configuration was not modified or treated as
proof of execution. Broader SSRF/source guardrail/error-classification mutation
coverage remains a closure gate.

## Reused source-adapter evidence

Twenty-two existing `fyi-core` unit tests passed with local mock servers on
2026-11-14 (1.88 seconds of test execution). The exact command is retained in
the track evidence. No second source harness or source changes were introduced.

| Existing sensor family | What passed | What is still not established |
| --- | --- | --- |
| JSON parsing and persistence | Shared response fixtures, optional fields, malformed/missing-field rejection without local mutation | Explicit source fixture version manifest |
| RSS/Atom links and feed retrieval | Link extraction, deduplication and watched-request fetch | Shared versioned feeds, malformed XML, full parser conformance |
| SSRF and redirects | Private/credential target rejection, rejected cross-port redirect, accepted same-origin relative redirect | Rebinding, redirect-chain exhaustion and other untested redirect variants |
| Source request/byte guardrails | Oversized small response and blocked next request; exact one-request allowance | Multi-chunk cancellation/peak memory and MCP-to-source budget propagation |
| HTTP cache | Reuse on 304, error responses do not replace a valid entry | Corrupt cache metadata/body or filesystem cache injection |
| Timeout and retry | Pacing wait rejected beyond runtime; exhausted retry queue/dirty-state outcomes | Stalled socket/header/body timeout and exact retry attempt counts |
| Remote state update | Invalid/stale state produces no PUT; matching preflight permits PUT | Atomic server-side comparison during a concurrent state change |
| Error privacy | Status errors exclude response bodies | Privacy for the absent transfer/cache fault cases |

No deterministic mid-body disconnect or malformed HTTP chunk test was found.
These gaps are missing evidence, not demonstrated source defects. MCP timeout,
clock, circuit, confirmation and returned-payload tests cover their own boundary
and do not substitute for the missing source-transfer assertions.

## Compatibility and release-readiness matrix

| Combination or requirement | Evidence | Status |
| --- | --- | --- |
| Linux x86_64, Rust 1.99.1, MCP SDK 1.29.0 and 1.30.0, Python 3.12.14 | Executable framing plus SDK stdio/HTTP, default/2025-06-18/2025-03-26 profiles | Passed locally |
| Remote-disabled, read-enabled and write-gated stdio registration | Real SDK sessions; all three write preparations/rejections use v2 fixtures | Passed offline |
| Degraded stdio startup and recovery | Real SDK launches with strict startup control, write denial, restart recovery and private errors | Passed on Linux; recovery requires a new subprocess and fresh confirmations |
| Kill switch, cooldown and skew | Deterministic injected policy/gate/future tests | Passed internally; no external live-control channel claimed |
| Workspace stable/MSRV on Linux | Existing CI and Frontier Conductor/MSRV jobs | Record checks for each PR's final head |
| Dependency-lock MCP SDK version | Existing Python CI resolves `uv.lock` | SDK 1.30.0 passed locally; full locked-environment final-head CI is recorded separately |
| Windows and macOS runtime/SDK pairs | Tests are portable; no new platform workflow introduced | Not executed in this workstream |
| Supported source JSON/RSS/Atom variants | Existing source tests reused | Shared versioned feed fixtures and full MCP-to-source success matrix incomplete |
| Source transfer/cache faults | Existing source unit tests plus MCP future timeout/circuit tests | Disconnect, malformed transfer and corrupt cache integration not established |
| Full layered release gate | Current verifier, Rust tests and SDK suites reuse existing jobs | Track remains open pending the remaining rows |

The acceptance checklist and exact closure dependencies are maintained in the
existing track's `acceptance-evidence.md`. No archived or release-ready status
is inferred from a green local unit suite.

## Normative references

- [MCP 2025-06-18 lifecycle](https://modelcontextprotocol.io/specification/2025-06-18/basic/lifecycle)
- [MCP 2025-06-18 transports](https://modelcontextprotocol.io/specification/2025-06-18/basic/transports)
- [MCP 2025-03-26 transports](https://modelcontextprotocol.io/specification/2025-03-26/basic/transports)
- [MCP 2025-06-18 tools](https://modelcontextprotocol.io/specification/2025-06-18/server/tools)
