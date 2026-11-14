# Local TUI runtime acceptance

Scope: PR #448, continuing merged #435 under the existing
`rust-mcp-tui-implementation` track. This is a bounded metadata-view increment;
local and acquired timelines remain explicitly unavailable. The combined track
stays open. These instructions do not retire Python or qualify acquisition/MCP.

## Executable contracts

Run against the exact candidate checkout, with the pinned Rust toolchain:

```sh
cargo test --locked -p fyi-core --test local_overview
cargo test --locked -p fyi-cli tui::
cargo test --locked -p fyi-cli --test tui_cli_tests
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
python scripts/verify_frontier_conductor.py
```

The Linux executable test invokes a Python 3 standard-library PTY fixture. It
uses invented local data, with no live endpoint. It exercises real launch,
privacy/reveal, metadata/details, unavailable history, shrink/grow, q, Ctrl+C,
SIGTERM, failed data, terminal flags/cursor/alternate-screen restoration and
unchanged files. The Rust runtime tests inject startup/draw/input/cleanup failures,
pending data and a panic. They complement, rather than certify, platform-specific
manual experience. The existing workspace and coverage Actions execute these tests
without changing thresholds or exclusions.

## Manual terminal and accessibility acceptance — OPEN

No operator session, terminal emulator, screen reader or assistive-technology
result is certified by the automated PTY test. Record the exact commit, release
build command, OS, terminal/version, dimensions, font/scaling, assistive technology,
operator, date and each observed result. Use synthetic data only.

1. Prepare a throwaway database with `init-db --db tui-acceptance.db`, import an
   invented authority CSV and register synthetic draft and other-status requests
   with the accepted local commands. Do not use a live portal or personal data.
2. Launch `fyi-cli tui --db tui-acceptance.db`. Verify default privacy in all four
   tabs, details/help and after resize: no titles, identifiers, labels, counts or
   timestamps should appear. `/` must not permit a privacy-revealing filter.
3. Reveal with `p`. Check real authority names, status text, selection markers,
   draft metadata and the supplied timestamp. Confirm Timeline says unavailable
   and does not invent history. Repeat with empty and >500-row synthetic datasets;
   verify complete/partial scope and loaded versus total counts.
4. Navigate using Tab/Shift+Tab, arrows/j/k, Home/End and Page Up/Page Down. Check
   visible selection and readable labels without color. Exercise filter typing,
   Escape, help and scrolling long Unicode titles/details. Confirm there is no
   write, export, credential or network action.
5. Resize through 120x40, 80x24, 40x12, 36x10 and below the minimum, then grow.
   Check clipping, wrapping, focus, selection, contrast and high-DPI/large fonts.
   No hidden data should be exposed while resizing.
6. With the target screen reader enabled, verify reading order, active tab,
   selection, privacy state, status, errors and keyboard discoverability. Record
   actual failures; static labels and buffer tests are not screen-reader support.
7. Hide with `p`, quit normally and via Ctrl+C, and reopen. Verify raw mode is off,
   normal echo/cursor/style and the original screen return, and the new session
   is redacted. Test missing/unsupported databases and redirected stdout. External
   terminal capture files are outside the program's ability to retract content.

| Platform / access mode | Current acceptance |
| --- | --- |
| Linux PTY, synthetic data | Executable test supplied; exact-head result recorded in PR evidence. |
| Linux terminal emulator, visual keyboard use | Open: no manual operator evidence. |
| macOS terminal and VoiceOver | Open: no manual operator evidence. |
| Windows Terminal and NVDA or Narrator | Open: no manual operator evidence. |
| Screen reader reading order and announcements | Open: must be observed with the actual assistive technology. |

## Original performance budgets — OPEN

The original spec requires **>30 FPS** and **<20 MB RAM**. No elapsed CI test time,
debug/coverage build, synthetic buffer benchmark or cached build establishes
those targets. The runtime is event-driven; idle redraw frequency is not FPS.
Qualification must distinguish input-to-visible-frame latency, active redraw rate,
database-load latency and full-process memory.

Build an uninstrumented release candidate with
`cargo build --locked --release -p fyi-cli` (no `dhat-on`). On Linux, run in a real
terminal, for example:

```sh
/usr/bin/time -v -o tui-rss.txt target/release/fyi-cli tui --db tui-acceptance.db
```

Measure empty, 500-row and >500-row databases with long Unicode text at the
dimensions above. Record hardware, OS, database size/journal mode, warm/cold runs,
at least five repeated trials and exact head. Use the process maximum RSS,
including the input/SQLite worker threads; report the tool's units. Until the
spec chooses otherwise, apply the stricter decimal budget: RSS in KiB times 1024
must be **below 20,000,000 bytes**. Keep startup and steady-state results separate.

For the interactivity target, use timestamped input injection and terminal-frame
observation/capture on the same machine. Sustain navigation/resize/filter updates,
report active completed frames per second plus median/p95/worst input-to-visible
latency, and retain the synthetic workload and tool/version. A >30-FPS claim
requires measured active throughput above 30, with stalls identified; do not infer
it from the 50-ms input poll timeout or an idle loop. Record the input/render cost
separately from terminal-emulator presentation cost where possible.

No manual performance measurements have yet been supplied. If either original
budget fails, keep that criterion open and profile a separate bounded follow-up;
do not relax the number or label the combined track complete.

## Integration and rollback

The sole Issue fixes and PR merges coordinator owns the final master refresh,
exact-head required Actions and merge/pruning slot. A draft implementation head
may be verified independently but is not automatically merge-ready. No new
migration or stored data format is introduced; rollback is a focused revert of
the adapter/launch increment.
