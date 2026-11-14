# Actual MCP-to-source offline contracts

Issue #169 / #173 originally required successful remote-tool calls and source
faults through the actual executable. Schema snapshots and synthetic MCP results
alone do not establish that integration. `tests/test_mcp_upstream_contract.py`
drives the ordinary Rust binary with the official Python MCP SDK and unchanged
production `SyncClient`.

## Fixture boundary

`tests/fixtures/remote_mcp/upstream-v1.json` is a versioned synthetic wire corpus.
It identifies exact methods, paths, query encoding, JSON/multipart request fields,
response shapes and expected structured MCP results. Source and MCP tests can
consume this common corpus without duplicating transport/parsing code. It is
**not** a recording, legal approval or certification of a live platform. Current
tools and schemas are unchanged.

The fixture accepts only `CONNECT mcp-fixture.invalid:443` on a loopback listener,
terminates TLS itself and has no forwarding code. An independent test verifies
rejection of another target with no HTTP request observed. The actual executable
receives an explicit local proxy and an ephemeral CA through its own environment.
No production URL validation or TLS verification is disabled; system trust is
unchanged. Private keys are generated per temporary fixture, stored with private
permissions and never committed. The child's environment excludes inherited
credentials and supplies only an explicitly synthetic protected-header value.

The `SSL_CERT_FILE`/empty `SSL_CERT_DIR` trust path is qualified on Linux. Runtime
cases explicitly skip elsewhere; fixture-inventory checks still run. Those skips
are unexecuted Windows/macOS acceptance, not passes. The source HTTP
implementation, platform TLS roots and operating-system trust remain unchanged.

## Executed matrix

| Surface | Assertions |
| --- | --- |
| Health | Real request-list GET and separate network/API reachability fields |
| Version | String/object representations, same-origin redirect, chunked JSON split within UTF-8 sequences |
| Request | Complete/minimal representations and exact request-ID endpoint |
| Search | Array, `results` and `requests` wrappers; encoded Unicode/punctuation query; cap when the server over-returns |
| Authorities | Array and `authorities` wrapper; Unicode case-insensitive filtering and bounded result |
| Create | Prepare and invalid confirmation send nothing; exact JSON POST; structured result; cached/consumed replay sends nothing |
| Correspondence | Exact JSON POST and synthetic multipart upload; fields, attachment name, MIME and bytes checked |
| State | Matching state produces GET then PUT; stale state produces GET only. This is client preflight, not atomic server compare-and-set. |
| Failures | 401/403/404/429/500/503; malformed JSON, missing version, uncached 304; disconnect, truncated body, malformed headers/chunks; foreign/loopback redirect rejection |
| Stalls | Headers and body each stall until the unchanged production 60-second timeout; private failure, exactly one request, peer connection closure and a responsive subsequent ping |

The 16 successful scenarios and 15 failure variants execute over real loopback
TLS. Each failure requires exactly one observed request, a fixed private MCP
error and a responsive subsequent ping. These operations do not retry ordinary
HTTP failures; this is not evidence about other sync retry APIs. A legitimate
same-origin redirect intentionally has two observed requests.

Two additional stall cases measure production cancellation, with an elapsed
bound of 50–70 seconds and independent observation that the source connection
closed. Their fixture and SDK deadlines are deliberately longer than the
production timeout. A fast fixture/SDK timeout cannot satisfy those assertions.
Together with inventory, proxy rejection, stale/read-only denial and generated
query coverage, the module contains **37 tests**.

Generated testing adds 12 deterministic query/limit examples through the actual
SDK and source encoder, checking whitespace normalization, Unicode, URI
punctuation, observed query strings and returned caps. Bounded synthetic wire
captures stay in memory. Diagnostics must omit synthetic private error content,
credentials and write/attachment body markers.

## Run and review

```sh
cargo build --locked -p fyi-mcp
python -m pytest -q tests/test_mcp_upstream_contract.py
python -m ruff check tests/mcp_upstream_fixture.py tests/test_mcp_upstream_contract.py
```

The existing MCP fixture builds the executable, or accepts an explicit
`FYI_MCP_TEST_BINARY` path. Explicit binary failures are never converted to skips.
Keep its source head with the evidence. SDK result-schema validation stays
enabled. Every expected exchange must execute; extra/mismatched requests fail.
Incoming capture is bounded at 2 MiB. Ordinary fixture connection I/O and SDK
calls are bounded at five seconds; SDK sessions are bounded at 20 seconds. Only
the two production-stall cases use 75-second fixture I/O, 70-second SDK calls and
90-second sessions. Listeners/child processes are cleaned up.

An executed inventory assertion on baseline `76de6a2` first failed because the
versioned actual upstream corpus was missing. This does not claim the existing
endpoint implementations failed. Exact local/hosted and targeted mutation
evidence is recorded in the existing Conductor track and focused PR.

## Remaining source and platform requirements

The original tracks remain active:

- **Read-only feeds:** RSS/Atom and authority-feed tools are absent from the
  current MCP catalogue. Source `pull_authority_feed`/`pull_feed` also write to
  the application database. The source owner needs to provide bounded typed
  read-only discovery, including malformed-feed behavior, before MCP exposes it.
- **Raw budgets:** source `GuardrailConfig` defaults to 500 MiB per response and
  one hour per client; MCP bounds serialized results at 4 MiB and each operation
  at 120 seconds. MCP allowances are not passed into a source limits constructor.
  A source-owned interface must accept remaining request/byte/time allowances and
  establish early multi-chunk cancellation and peak-memory behavior.
- **Cache and errors:** MCP reconstructs a source client per call. Uncached 304
  denial is tested; reused-cache integrity/corruption and provenance need source
  evidence. Private MCP errors currently erase source status/kind/retryability.
  A typed safe source-error interface is needed; string parsing is insufficient.
- **Raw streams:** header/body stall cancellation is now measured on Linux.
  Early cancellation of an oversized multi-chunk stream, raw-byte/request budget
  propagation, peak memory and reused-cache corruption still need source-owned
  evidence; timeout closure does not establish those separate bounds.
- **Writes:** multipart success does not prove immutable bytes across preparation
  and source path reopen. Immutable attachment submission and atomic upstream
  state changes remain explicit #171 dependencies. Durable replay/audit is the
  separate #457 increment.
- **Compatibility:** Windows/macOS runtime/ACL, complete compiler/client coverage,
  broader source mutations and genuine profile-withdrawal/migration evidence
  remain open. No protocol profile is invented or declared deprecated here.

The sole integration coordinator owns final combined-head verification, merge
and pruning. This increment depends on #458's offline scheduler fixture repair;
the prior unguarded full run was rejected and is excluded from passing evidence.
No `sync.rs`, source adapter, `db.rs`, CLI dispatch, workflow,
packaging, runtime lock or shared Conductor registry changes are included.
