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

- #488 / #310 at `a26298e55f18c3056f5d063f9b71867ee9df4c12` adds a dedicated
  request-evidence transport. When integrating it with this change, retain its
  `evidence_http` initialization in the new shared-guardrail constructor and its
  separate evidence constructor. This change does not implement a competing
  receipt engine or alter its source-adapter track.
- #489 / #312 at `7805d42b0ccf986a55a21b97cfdeae0a5103a8a4` qualifies Wayback
  transport/replay and the existing receipt portability workflow. Its Wayback
  builder and h2 test dependency are distinct from the bounded read builder and
  quick-xml parser added here. The new read workflow leaves its workflow intact.

Reconcile fresh heads before integration; do not overwrite either source owner.

## Closure

Implementation is under review. Issue #170 and its existing Conductor track
remain open until current-head gates and final evidence are recorded. Broader
#169/#171/#173 write atomicity, platform TLS, filesystem cache and whole-process
memory questions are not closed by this read increment.
