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

Implementation PR: [#494](https://github.com/edithatogo/fyi-cli/pull/494).
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
gate. The corrected implementation revision is qualified below.

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
receipts for #169/#173 are not evidence for this new code. The following
qualification uses this implementation revision directly.

## Verified implementation revision

Source commit: `0f80180abbc4a3d83f1a183f04611e7308ce2e52` (tree
`88ce7a594761fcf70ff9b4cf0336a899ee45d361`). Hosted PR checkout:
`909fec0dfca2af689cf98bf6a49c2265773b6630`.

All 12 stable required checks completed successfully: Rustfmt, Clippy Lints,
Workspace Tests, Package Smoke, Security Audit, Code Coverage, Layered Harness
Contract, Python Tests, CodeQL, Analyze (python), Analyze
(javascript-typescript), and Packaging Assets. The
[main CI run](https://github.com/edithatogo/fyi-cli/actions/runs/44058738293)
and [read-contract matrix](https://github.com/edithatogo/fyi-cli/actions/runs/44058738450)
completed successfully using Rust 1.100.0 and Python 3.15.2.

| Platform | Source read tests | MCP unit tests | MCP integration tests | Actual SDK cases |
| --- | ---: | ---: | ---: | ---: |
| Ubuntu 24.04 | 30 passed | 175 passed | 9 passed | 52 passed |
| macOS 15 | 30 passed | 175 passed | 9 passed | 4 passed |
| Windows 2025 | 30 passed | 173 passed | 9 passed | 4 passed |

Every listed Rust suite reported zero failures and zero ignored tests. Windows
excludes two Unix-only unit tests at compilation. All selected SDK cases passed
the explicit no-skips JUnit gate. Linux covers actual local TLS/source exchanges;
macOS and Windows cover portable SDK discovery/policy behavior alongside their
source and MCP Rust suites. Native TLS on those two platforms is not certified.

The full Python suite passed 1,741 tests with two skips. Coverage independently
ran all 237 core unit tests and the MCP unit/integration suites and reported
93.55% line coverage and 89.99% region coverage. The optional self-hosted Offline
AI job remained queued; it is not one of the repository's 12 required checks and
is not claimed as passed. The PR body records the latest documentation-head CI
status; this section preserves the exact implementation execution checkpoint.

## Manual review and phase checkpoint

The implementation, source/SDK qualification and independent scoped reviews are
complete. `.conductor/workflow.md` separately requires explicit user feedback
before a final manual-verification checkpoint is created. No such confirmation
has been received or recorded. The manual review can remain entirely offline:

1. Build the production server with
   `cargo build --locked --package fyi-mcp --bin fyi-mcp`.
2. Configure a stdio MCP client to launch that binary with
   `FYI_MCP_EPHEMERAL=1`, `FYI_MCP_TRANSPORT=stdio`,
   `FYI_MCP_REMOTE_INSTANCES=synthetic=https://mcp-fixture.invalid`,
   `FYI_MCP_REMOTE_READ=synthetic`, and an empty `FYI_MCP_REMOTE_WRITE`.
3. Read `fyi://remote/read-contract`, then call
   `remote_discover_authority_feeds` with
   `{"instance_id":"synthetic","authority_slug":"synthetic-authority"}`.
4. Confirm the returned feed URL is
   `https://mcp-fixture.invalid/feed/body/synthetic-authority`, availability is
   `not_checked`, provenance has `cache_status: not_applicable`, null source
   status and zero downloaded bytes, and the result has a correlation ID.
   Confirm that the catalogue exposes seven remote reads and no remote writes.
5. Review the realistic feed fixtures and CI evidence: event IDs and numeric
   request slugs stay unresolved; only explicit API routes produce request IDs.
   Confirm the behavior before creating the final manual phase checkpoint.

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

Implementation and automated acceptance are verified at the revision above.
PR #494 remains subject to its final-head checks and review. The track stays
in progress until the manual checkpoint and merge lifecycle are complete. Broader
#169/#171/#173 write atomicity, platform TLS, filesystem cache and whole-process
memory questions are not closed by this read increment.
