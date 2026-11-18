# Acquisition evidence integrity — 17 November 2026

Related issue: [#310](https://github.com/edithatogo/fyi-cli/issues/310).
Implementation PR: [#488](https://github.com/edithatogo/fyi-cli/pull/488).
Receiving master: `1f0ae6d0b00663d52ccb65904152ca4e933b3486`.
First implementation checkpoint: `a26298e55f18c3056f5d063f9b71867ee9df4c12`.
Verified implementation checkpoint: `90129b7779a7ae11b6d7cab1bcb034acfbd6ce77`.

## Implemented behavior

- Public request acquisition uses one dedicated guarded GET with a 16 MiB cap,
  no redirects, conditional cache, content decoding or automatic retries,
  including reqwest's default protocol retries. Complete HTTP, parse, encoding
  and identifier failures preserve raw bytes, a fixed failure projection and a
  failed v1 receipt; the command still returns an error. Incomplete exchanges
  are refused because receipt v1 cannot honestly represent their partial bytes.
- Evidence preflight rejects normalized aliases, nested outputs, existing
  destinations, symlink ancestors, and database/sidecar overlaps before source
  or database I/O. It runs again after the awaited exchange. Literal database
  filename options keep the validated and opened paths identical, including
  percent signs and Unix backslashes. Receipt mode rejects SQLite URL syntax.
- Sync pull projections now carry redacted before/after stored-boundary
  snapshots. The receipt hashes each canonical snapshot; the result hash binds
  them both. Final state is observed after partial application failures too.
  A missing or invalid final boundary leaves raw evidence without publishing a
  completed result or receipt. Numeric rate-limit signals are parsed before
  filesystem/database processing and exclude free-text headers.
- Built-in `SyncClient` constructors retain a paired evidence transport. An
  opaque caller-supplied client is refused by the legacy evidence method rather
  than silently replacing its proxy/TLS policy. Ordinary custom-client APIs are
  unchanged.
- Dedicated replay transport also explicitly disables reqwest protocol retries
  so its existing attempt accounting remains truthful.

The v1 schema, canonicalization and immutable writer contract are retained. No
dependency was added and no live source request, submission or publication was
made during this implementation.

## Regression and review evidence

The tests-first source was `5368b6a78d9bf4404f8c630fceda234a5cf6880d`.
GitHub tested synthetic merge `fb9ec0e1960f3561fa5872a1a7f4c30af5d75ba3`
in [CI run 44051717545](https://github.com/edithatogo/fyi-cli/actions/runs/44051717545).
Coverage job `133930901034` executed the original six fetch-preflight tests:
five failed for missing preflight behavior and the valid-path control passed.
This is actual behavioral red evidence. Cargo stopped before the core lineage
unit tests, so a red result for those tests is not claimed. Initial Rustfmt
failures were also corrected; they are not behavioral red evidence.

New socket-free acquisition and lineage cases cover complete failures, body
interruption and bounds, source identity, shared pacing, boundary advance and
stability, partial database application, numeric header redaction, competing
output files, and parent replacement during the source await. Filename tests
verify the actual SQLite file created. The existing receipt portability matrix
now executes these focused suites on Linux, macOS and Windows while retaining
the prior persistence, process, canonicalization and CLI ingestion suites.

Independent read-only review identified and prompted fixes for post-await path
substitution, reqwest protocol retries, delayed HTTP-date interpretation, and
SQLite URL/path identity. It also checked opaque custom transport policy,
publication order, partial database state and existing API test compatibility.
Static review is not a substitute for execution.

Local `cargo fmt --all -- --check`, `git diff --check` and
`python scripts/verify_frontier_conductor.py` passed. Seven pure portability
workflow contract tests passed when invoked directly under CPython 3.15.2;
pytest was not installed in that interpreter. Local Cargo builds were terminated
with SIGKILL while compiling dependencies, including with one build job; they
provide no passing or failing Rust test result. Hosted checks on the final PR
head remain the acceptance evidence. Their results are recorded in the PR.

### Hosted green checkpoint

All twelve required checks passed for source
`90129b7779a7ae11b6d7cab1bcb034acfbd6ce77`, tested by GitHub on synthetic merge
`319e5196b59274bc6d50c9000f51cb6782454fbc` into the receiving master above.

| Gate | Evidence |
| --- | --- |
| Full CI: Rust workspace, strict Clippy, Python, property tests, package smoke, security audit, packaging, layered harness and coverage | [Run 44054377597](https://github.com/edithatogo/fyi-cli/actions/runs/44054377597), all ten jobs passed. |
| Rust 1.99.1 full workspace and strict Clippy; Conductor contracts | [Run 44054377789](https://github.com/edithatogo/fyi-cli/actions/runs/44054377789), both jobs passed. |
| Linux, macOS and Windows receipt portability | [Run 44054380472](https://github.com/edithatogo/fyi-cli/actions/runs/44054380472), all three jobs passed. |
| CodeQL and both language analyses | [Run 44054377710](https://github.com/edithatogo/fyi-cli/actions/runs/44054377710), passed. |
| Python security, Hypothesis and four Atheris suites | [Run 44054377581](https://github.com/edithatogo/fyi-cli/actions/runs/44054377581), passed. |

The MSRV log contains 56 test-result summaries totaling 843 passed, zero failed
and one pre-existing ignored helper, across unit, integration and documentation
test binaries. All 32 new tests passed on Linux and macOS: thirteen request
acquisition, eight pull lineage, four literal database filename and seven CLI
preflight tests. The corresponding Windows suites passed; Unix-only cases are
conditionally excluded there.

The coverage job measured 93.60% line coverage; Codecov's patch check passed at
97.10% against its unchanged 90.00% target. Dependency review, privacy/secrets,
workflow linting, profiling and test-profile isolation also passed. At this
checkpoint the non-blocking Python migration sensor was still running and the
advisory self-hosted offline-AI security job was queued. Neither is counted as
passed. No protection, test or coverage threshold was weakened.

## Current network-command coverage and remaining gates

| Native command/path | Receipt support and current boundary |
| --- | --- |
| `fetch-request-page` | Complete bounded single-response success/failure evidence; all three explicit output paths required. |
| `sync pull` incremental API list | Complete bounded single-response evidence, raw/result hashes, numeric rate signals and before/after boundary lineage. |
| `sync pull --feed-url` | Receipt mode fails before source/database I/O; multiresponse acquisition and partial-failure evidence remain open. |
| `sync push` | Receipt mode fails before source/database I/O; write attempts, retries and mixed outcomes need a separate evidence design under existing write authorization. |
| `internet-archive-replay` | Existing bounded replay, ordered response bodies, v1 receipt and checkpoint integration retained; hidden protocol retries disabled. |

Unsupported native source-command stubs do not acquire data. Retained Python
ingest-feed/run-cycle/scheduler/source-probe paths still require receipt-producing Rust
migration or retirement; the legacy implementation was not extended. Full
network-entrypoint coverage, WARC/WACZ and attachment linkage, and downstream
`fyi-archive` shadow-parity evidence remain open. Live smoke and archive cutover
are separate parent-track gates and were not authorized or performed here.

Filesystem checks assume an operator-owned directory tree; they do not claim
handle-relative protection against every concurrent hostile replacement. The
three output files are not one transaction. Sync applies records individually,
and boundary snapshots describe observed state without claiming transactionality
or exclusive causation in the presence of other database writers. Rate signals
do not claim measured pacing waits. Power-loss/fault consistency and Windows ACL
privacy are not established by the added tests.

Issue #310 and the source-adapter track remain active;
`archive_cutover_allowed` remains `false`.
