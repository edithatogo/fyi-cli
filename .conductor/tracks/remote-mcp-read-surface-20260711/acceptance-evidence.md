# Issue #170 implementation evidence

## Scope and receiving baseline

Started from master `1f0ae6d0b00663d52ccb65904152ca4e933b3486` on
`codex/issue-170-remote-read-contract`. The issue's five original reads were
already delivered. This increment supplies typed read errors, per-result
provenance, retained source state, explicit source bounds, and read-only feed
route discovery/pull planning. It does not call a live authority or mutate the
application database through a remote read.

## Local verification at the implementation checkpoint

- `cargo fmt --all -- --check`: passed.
- `git diff --check`: passed.
- `python scripts/verify_frontier_conductor.py`: passed.
- `python scripts/verify_remote_mcp_contract.py`: passed for current v3 and historical v1/v2.
- Python fixture/verifier tests: 144 passed, Python 3.12.14. Two fixture transport
  checks also passed in the harness owner's run, giving 146 focused checks.
- After the source-based feed corrections, 150 fixture/verifier tests and two
  upstream-fixture checks pass (152 total); historical fixtures remain unchanged.
- An isolated harness executed ten exact, mechanically extracted parser tests
  successfully against the repository's locked dependency versions. This is
  parser-only evidence for `read_contract.rs` SHA-256
  `f1a0e0e84d540073f1127120583a377612b23cbfea470a406c9832e56d83368c`,
  not a substitute for the production core/MCP suites. The updated source module
  contains 30 tests for hosted execution.
- Ruff lint/format of changed Python harness files: passed.
- Independent source/MCP review identified redirect/request-count, capability
  discovery and nonstandard HTTP-status schema defects; implementation and
  regression tests address them.

The local Rust compiler was terminated with SIGKILL under memory pressure,
including one-job/debug-disabled checks and a smaller harness importing actual
source modules. This is an infrastructure failure, not a passing build or an
observed behavioral test failure. No local Rust runtime pass is claimed at this
checkpoint. The full hosted gates remain authoritative.

The Python contract tests first failed on missing new feed tools/current output
schemas (142 passing, two failing). The corrected source-derived catalogue and
fixture set passed the 144-test verifier suite. The production catalogue still
has an exact executable comparison in Rust and the official SDK; source-derived
fixture generation is not recorded as binary execution evidence.

## Hosted checkpoint and source-contract review

Draft implementation PR: [#494](https://github.com/edithatogo/fyi-cli/pull/494).
At `dc0910a388a51aa6a32fd069bc715d40ee3032b9`, the macOS job in
[run 44055105225](https://github.com/edithatogo/fyi-cli/actions/runs/44055105225)
built the actual production MCP executable and passed all 21 source-read tests.
Its subsequent MCP test compilation exposed a new test's invalid
`RemoteMcpPolicy::default()` call. The next revision uses the existing validated
empty-policy constructor. The same head's full Python 3.15.2 suite reported
1,727 passes, three failures and two skips; all failures were redirect-case
expectations. The source correctly exposes an unfollowed HTTP 302 as
`upstream_unknown`; the current harness now asserts that status and still
requires exactly one GET. The failed job is not recorded as a passing platform
gate; the new revision still requires its own complete hosted run.

Review against pinned upstream Alaveteli source also corrected the new feed
contract: authority feeds use `/feed/body/<slug>`; normal Atom links refer to
request events; request-page paths contain slugs, including potentially numeric
slugs. Current fixtures therefore preserve bounded typed references and derive
request IDs only from the client's explicit numeric API route. They cover
event links, slug fragments, document-relative links, shared truncation, empty
feeds and foreign-only feeds without following any reference. The operator
documentation links the exact upstream route, template and controller revision.

## Runtime and platform gates

The supplemental `Remote read contracts` workflow runs actual source tests and
MCP crate tests on Linux, macOS and Windows, plus portable official SDK read
discovery/policy checks. Linux additionally runs actual production-binary local
TLS cases. Every selected SDK case must execute: skips and missing cases fail
the platform receipt. It retains source/tree/binary identities, compiler/client
versions, command logs and JUnit artifacts.

Native macOS/Windows TLS trust-store behavior is separately unqualified; no
system trust-store change or TLS verification bypass is introduced. Historical
receipts for #169/#173 are not evidence for this new code. Full CI, coverage,
security and runtime results are pending at this checkpoint.

## Concurrent source work

No open PR existed at intake. During implementation, two independent source
PRs appeared:

- #488 / #310 at `f3aa10ceb5a0a2fd58093ac2e78a220f6c81279c` adds a dedicated
  request-evidence transport. When integrating it with this change, retain its
  `evidence_http` initialization in the new shared-guardrail constructor and its
  separate evidence constructor. This change does not implement a competing
  receipt engine or alter its source-adapter track.
- #489 / #312 at `dd56b7de0de1142b2bd1b58b738c04fcca8da003` qualifies Wayback
  transport/replay and the existing receipt portability workflow. Its Wayback
  builder and h2 test dependency are distinct from the bounded read builder and
  quick-xml parser added here. The new read workflow leaves its workflow intact.

Reconcile fresh heads before integration; do not overwrite either source owner.

## Closure

Implementation is under review. Issue #170 and its existing Conductor track
remain open until current-head gates and final evidence are recorded. Broader
#169/#171/#173 write atomicity, platform TLS, filesystem cache and whole-process
memory questions are not closed by this read increment.
