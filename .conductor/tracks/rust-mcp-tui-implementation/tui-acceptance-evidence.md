# TUI presentation acceptance evidence

Status: bounded presentation implemented and locally verified, 2026-11-14. Final-head Actions remain the merge gate.
PR: [#435](https://github.com/edithatogo/fyi-cli/pull/435).
Scope: the standalone read-only presentation portion of Phase 2.3 only.
Contract: [TUI presentation contract](../../../docs/tui-presentation-contract.md).

## Audited baseline

The baseline is commit
[`a816dcd425a7b6277761a18faeae1685202eefde`](https://github.com/edithatogo/fyi-cli/commit/a816dcd425a7b6277761a18faeae1685202eefde).
The initial regression-test checkpoint is
[`40261e57be7b33d7bd75275dc6ebb1a415c90a23`](https://github.com/edithatogo/fyi-cli/commit/40261e57be7b33d7bd75275dc6ebb1a415c90a23).
The expected red result was observed locally and in [Workspace Tests](https://github.com/edithatogo/fyi-cli/actions/runs/43552675034/job/132318429651): the three new defect regressions failed; the resize regression passed. This is a baseline failure, not completion evidence.

| Baseline observation | Consequence for acceptance |
| --- | --- |
| `main.rs` dispatches `tui` to `unsupported_command`. | CLI launch is unimplemented; rendered buffers cannot establish that the command boots. |
| `AppState::new()` includes sample records, invented totals and activity messages. | Ordinary startup must use honest empty/loading state; examples belong in synthetic fixtures. |
| The presentation state contains DB save/delete and keyring mutation helpers. | The delivered read-only path must contain no backend service handles or write actions. |
| Conflict mapping uses requester `user_name` as authority. | Authority must come from a real relationship or remain unavailable. |
| Credential identities and backend error strings enter displayed state. | Default privacy and static generic presentation errors are required. |
| Keyboard handling processes release events, and modal views intercept exits. | Explicit event-kind behavior and a global Ctrl+C exit are required. |
| Tables/logs do not maintain a visible viewport around selection. | Interaction and resize tests must assert visible selection. |
| No supplied timeline contract or loading/error/privacy model exists. | Unknown, unavailable, empty, loading and failure states need distinct behavior. |
| The old plan and metadata mark all TUI work complete. | Historical completion/manual-verification claims require correction, not repetition. |

These findings concern the baseline. The bounded presentation corrections are verified below; runtime/CLI integration remains a separate open acceptance item.

## Bounded acceptance matrix

| Requirement | Required evidence | Current evidence |
| --- | --- | --- |
| Preserve Ratatui and the existing `AppState`/`draw_ui` approach. | Scoped source diff; no new application framework or dependency. | Passed: contract/interaction tests and final source review. |
| Owned `PresentationSnapshot` only; no backend/service handles. | Code review plus side-effect-negative interaction coverage. | Passed: contract/interaction tests and final source review. |
| At most 500 requests and 1,000 total timeline items. | Oversize and boundary fixtures; partial-scope assertions. | Passed: contract/interaction tests and final source review. |
| `safe_text` replaces unsafe controls and limits text to 512 Unicode scalars, including an ellipsis. | Unicode/control/truncation cases through rendered output and filter paths. | Passed: contract/interaction tests and final source review. |
| Typed unknown status/attention and supplied authority labels. | Unknown-value fixtures; supplied-versus-inferred metric assertions; actual authority adapter remains an integration requirement. | Passed: contract/interaction tests and final source review. |
| Complete/Partial scope and optional backend total remain distinct. | Summary snapshots for complete, partial and unknown-total data. | Passed: contract/interaction tests and final source review. |
| Summary, Requests, Timeline and Drafts display supplied data read-only. | Deterministic complete-buffer fixtures for each tab. | Passed: contract/interaction tests and final source review. |
| Empty/loading/failed views and separate timeline unavailable/empty states. | Exact messages and absence of stale prior content. | Passed: contract/interaction tests and final source review. |
| Static generic errors; no raw diagnostics or snapshot/error logging. | Source review and negative privacy fixtures. | Passed: contract/interaction tests and final source review. |
| All content redacted by default; filtering disabled while redacted. | Identical static hidden views for different snapshots; negative tab/overlay/filter cases. | Passed: contract/interaction tests and final source review. |
| `p` reveals/hides only in the current terminal session. | No persistence/service calls; valid refresh preserves choice, loading/failure clear data and rehide. | Passed: contract/interaction tests and final source review. |
| List navigation clamps and tab navigation wraps. | Press/release/repeat tests, stable-ID refresh and boundary cases. | Passed: contract/interaction tests and final source review. |
| Filtering and help implement documented modal behavior. | 64-scalar input limit, `q` as filter text and actual available shortcuts. | Passed: contract/interaction tests and final source review. |
| Summary and request/event details remain readable in small views. | Scroll-to-tail cases for long retained text; `d`/Enter/Esc interactions and wrapping. | Passed: contract/interaction tests and final source review. |
| Ctrl+C/Ctrl+Q exit every state; q exits outside filter input. | Help/details/filter/loading/error/privacy interaction tests. | Passed: contract/interaction tests and final source review. |
| Small/zero-area and shrink/grow rendering are safe. | Resize tests with visible selection, no panic and no disclosure. | Passed: contract/interaction tests and final source review. |
| No network, DB, export or credential action is emitted. | Pure state contract, code review and negative tests for old write shortcuts. | Passed: contract/interaction tests and final source review. |
| Final-head required Actions pass. | Exact final commit and completed check/run links. | Required merge gate; authoritative evidence in PR #435 Checks and description. |

## Verification record

All local commands below used the repository-pinned Rust 1.99.1 and the locked
dependency graph. The authoritative hosted head/check/merge record is maintained
in [PR #435](https://github.com/edithatogo/fyi-cli/pull/435) and its
[Checks view](https://github.com/edithatogo/fyi-cli/pull/435/checks); local results
are not substituted for the final-head required gates.

| Check | Observed result |
| --- | --- |
| Initial red regressions on `40261e57be7b33d7bd75275dc6ebb1a415c90a23`. | Three intended failures (invented defaults, release-event quit, overlay exit) and one passing resize regression; also reproduced in hosted Workspace Tests. |
| `cargo test --locked -p fyi-cli tui::` | 39 passed, zero failed or ignored: 15 contract tests and 24 interaction/render tests. |
| Complete render fixtures | 14 full cell-buffer snapshots inspected; final focused run compared fixtures without update mode. |
| `cargo fmt --all -- --check` | Passed. |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | Passed. |
| `cargo test --locked --workspace --all-features` | 380 passed, zero failed, one existing ignored subprocess worker that is invoked by its parent tests; 23 suites including the core doctest. |
| `python3 scripts/verify_frontier_conductor.py` and `git diff --check` | Passed. |
| Independent source review | Privacy and long-text accessibility findings corrected; final review reported no remaining must-fix finding. |
| Final-head required Actions, including unchanged 90% coverage | Required before merge; exact SHA, job results and links are recorded in the PR description and Checks view. Local TUI tests do not assert runtime performance or global coverage. |
| PR merge and branch pruning | Recorded by GitHub's PR merge event and the final workstream report; neither is asserted in advance by this pre-merge file. |

The existing CI excludes `tui.rs` from its aggregate coverage denominator. That
configuration is unchanged. The new contract module participates in aggregate
coverage; complete buffer/interaction tests provide direct presentation evidence.
No claim of inclusive TUI line coverage is made from the existing gate.

## Integration blockers and retained scope

`main.rs` and CLI dispatch remain owned by the separate request-management
workstream. The current identified PR is
[#434](https://github.com/edithatogo/fyi-cli/pull/434). At audit it is an active draft
authority-import/listing implementation, not a merged request/timeline data
contract. Integrate only against merged, verified backend APIs.

The future adapter must load existing data read-only without migrations,
database creation or sync. It must map actual authority data, preserve unknown
attention and partial scope, and supply genuine timeline events. The standalone
presentation cannot claim that those services exist merely because mocked input
can be rendered. The caller must supply the actual terminal size through
`Event::Resize` before navigation begins and whenever the terminal resizes; this
is how scroll/page bounds remain aligned with the frame used for rendering.

The following remain outside this evidence claim:

- CLI launch and real local-data integration.
- Terminal startup/restoration, asynchronous input and manual terminal checks.
- The original greater-than-30-FPS and less-than-20-MB RAM targets.
- Screen-reader compatibility or broader accessibility certification.
- Completion of CLI or MCP portions of the combined track.

The combined track remains open for reconciliation of its unresolved criteria.
Historical combined issue
[#10](https://github.com/edithatogo/fyi-cli/issues/10) is closed; this slice does not
close unrelated issues. Only this track’s registry/index entries are corrected
to keep the combined track open; other workstream records are preserved. A rollback of
this presentation change affects no database schema, stored data, external
service or published package.
