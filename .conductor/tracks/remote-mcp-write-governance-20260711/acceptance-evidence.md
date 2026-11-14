# Remote MCP write governance: remaining acceptance evidence

## Scope and status — 2026-11-14

Related: #171 and parent #169. This track stays **in progress**. Historical
closure/checkmarks did not prove a durable production journal, immutable
attachment content or server-side atomic state changes. Existing prepare/commit,
operation budgets and source delegation are preserved.

This increment owns only MCP gate/startup, its new private journal, tests,
operator documentation and this existing track. It uses the already-present
SQLite dependency plus `hmac 0.12.1` and `sha2 0.10.9`, which were already in the
workspace lock; only the MCP dependency edges are added. `db.rs`, `sync.rs`,
source transports, application CLI, i18n, shared registry and workflows are
unchanged. Master #452 was reconciled before the implementation commit, keeping
its shared credential derivation intact.

Published focused PR: [#457](https://github.com/edithatogo/fyi-cli/pull/457).
Test-only checkpoint: [`f5b130b`](https://github.com/edithatogo/fyi-cli/commit/f5b130b503809395aaf556d9bfa7a9009d3af34a).
Implementation: [`dbee12d`](https://github.com/edithatogo/fyi-cli/commit/dbee12d5ad42db30f8c15150c355cc85d5a39972),
exact locally tested tree `1772fd8aff4191a707e4ac9ec62e10b8c198e470`.
[Coordinator handoff](https://github.com/edithatogo/fyi-cli/pull/457#issuecomment-6877876150).

## Test-first evidence

At baseline `70fef29`, the real-binary
`remote_writes_require_persistent_state_before_startup` assertion failed:
"remote writes started with only process-local replay protection".

```sh
cargo test --locked -p fyi-mcp --test write_journal_contract
```

The original local RED commit was `d49c16a`. The same test-only checkpoint was
rebased onto merged #452 (`e953438`) as `899f65c`; the original failing execution
was against the earlier baseline. No claim that a compile failure established RED.

## Executed assertions

| Boundary | Evidence and limits |
| --- | --- |
| Startup | Missing or unusable protected journal configuration fails before application database creation. Read-only startup remains compatible. |
| Restart | A real stdio subprocess prepares and attempts a write. The source constructor safely rejects its loopback URL before I/O. After exit, a second subprocess using the same journal denies the same key and the old token. This proves restart persistence, not a successful upstream write. |
| Admission | Two SQLite connections race the same scope; exactly one reserves. An injected audit failure rolls back reservation and blocks admission. |
| Outcome | Success is persisted before publication; a matching in-process cached result works after budget exhaustion. Failed, cancelled and interrupted attempts remain reserved across reconstruction. Restart never replays a private result body. |
| Privacy | Database bytes and subprocess diagnostics exclude synthetic private body/result/token/key/path values. Only HMAC scope tags and fixed labels are stored. Rejected arguments never enter audit fields. |
| Storage | Missing marker/database, changed key, corruption, unknown format, closed connections, bounded attempt/event capacity and Unix mode/symlink errors fail closed. No automatic eviction or format reset. |
| Identity | Twenty-seven generated instance/operation/key combinations produce distinct tags; identity is exact and case-sensitive, and tags stay stable on reopen. Invalid scopes fail before persistence. |
| Events | The real two-process scenario observes ordered prepare/commit/failure/replay/reject/reject events; success and cancellation are independently tested at the gate/journal boundary. |

Three individually applied temporary mutants were caught by **executed test
assertions**, not by compilation:

1. Remove durable reservation from `reserve_durable`: closed-storage dispatch
   test rejects the bypass.
2. Allow preparation when a persisted scope exists: restart test rejects it.
3. Mark an uncertain result successful: the cached-success eligibility test
   rejects it.

Production files were restored byte for byte after each mutation. These three
targeted outcomes are not a broad source mutation score.

## Verification commands

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --locked -p fyi-mcp
cargo test --workspace --all-features --locked
PYTHONPATH=src FYI_MCP_TEST_BINARY=/absolute/path/to/fyi-mcp \
  python -m pytest -q tests/test_mcp_client_interop.py tests/test_remote_mcp_contract.py
python scripts/verify_frontier_conductor.py
python scripts/verify_remote_mcp_contract.py
```

Local environment: Linux x86_64, Rust 1.99.1, Python 3.12.14, installed MCP SDK
1.29.0. This is not evidence for the coordinator's separate Python 3.15.1 or
locked SDK 1.30.0 runtime work. Local Rust compilation disables debug info and
incremental output only to bound scratch storage; assertions, features, tests
and CI gates are unchanged. The reconciled workspace passed **696 tests**, zero failed, one ignored,
including **153 MCP tests** (144 unit, six existing wire, three journal wire).
The focused Python SDK/contract run passed **151 tests**; all 22 SDK cases ran.
Strict workspace Clippy and both Conductor/fixture verifiers passed.
Exact candidate hosted checks and coverage must be
recorded on the focused PR before integration.

## Original acceptance still open

- Successful actual MCP-to-source create/correspondence/state and attachment
  variants; HTTP error/fault/retry matrix and safe structured source errors.
- Source budget propagation, raw-transfer/cancellation/peak memory evidence.
- Immutable attachment bytes across preparation/submission; the source API
  still reopens paths. Existing path checks do not prove race resistance.
- Atomic upstream compare-and-set: existing GET-then-PUT is only preflight.
- Windows/macOS runtime and protected-state ACL qualification, and complete
  declared client/compiler compatibility evidence.
- Broader generated/mutation targets and the original complete release/rollback
  review. No sandbox live-write workflow has been run or added here.

SQLite full synchronization plus transaction/process-restart tests do not prove
power-loss behavior on every filesystem, audit tamper resistance, remote
exactly-once delivery or protection after an operator resets the state namespace.
See `docs/remote-mcp-security.md` for setup, retention, incident recovery and
rollback. The sole integration coordinator owns final head/base reconciliation,
required-check verification, merge and branch pruning. No optional queued AI job
is counted as passed; no gate or original acceptance item is waived.
