# Rust TUI presentation contract

Status: presentation merged in #435; bounded local runtime under verification in #448, 2026-11-14.
PR: [#435](https://github.com/edithatogo/fyi-cli/pull/435).
Existing track: [rust-mcp-tui-implementation](../.conductor/tracks/rust-mcp-tui-implementation/plan.md).

## Purpose and ownership

The terminal view presents an existing local request snapshot. It retains the
Ratatui `AppState` and `draw_ui` entry points and the existing dependencies. Its
state transitions and rendering are independent of backend services. The four
tabs are Summary, Requests, Timeline and Drafts; Drafts is a read-only view of
supplied draft records.

The TUI workstream owns `crates/fyi-cli/src/tui.rs`, its presentation modules,
fixtures and tests. The request-management workstream owns database handlers and
CLI dispatch. This follow-on increment adds only the TUI launch dispatch and a
bounded read-only repository projection. Acquisition, MCP, localization,
packaging and shared CI remain outside this change. This contract does not establish new backend functionality.

## Owned input data

`PresentationSnapshot` contains owned presentation values. It must not carry a
database, client, keyring, callback or other service handle. The presentation
layer accepts at most **500 requests** and **1,000 timeline items across the
entire snapshot**. Bounds apply across all records, rather than separately to
each request. Oversized snapshots are rejected with a static error; the view does
not silently truncate their collections. A caller-supplied bounded subset must
be identified as partial. Duplicate request identifiers and partial totals
smaller than the loaded request count are also rejected.

| Value | Contract |
| --- | --- |
| Request identifier | Supplied stable identifier used for selection and event association. |
| Title | Display title, subject to privacy and safe-text handling. |
| Authority | Actual authority metadata, or explicitly unavailable. A requester name is never an authority. |
| Status | Typed display status with an explicit unknown case. Unrecognized backend values cannot silently become a known state. |
| Attention | Typed known-required, known-not-required or unknown value. The view does not infer attention from dates, status labels or missing data. |
| Timestamp | Supplied display value or unavailable. Rendering does not read the clock or manufacture dates. |
| Timeline | Supplied events associated with the request; event order comes from the adapter. |
| Scope | `Complete` asserts the entire collection was supplied. `Partial` carries an optional backend total. Loaded row counts and optional backend totals remain distinguishable. |

Credentials, requester names, request URLs, request bodies, correspondence bodies
and raw backend error strings are excluded from this contract. Timeline summaries
must be prepared specifically for presentation; they are not raw log messages.
Synthetic fixtures must use invented data.

All supplied text passes through `safe_text` before it can be displayed or used
by a filter. That function replaces unsafe terminal/control, bidirectional and
invisible formatting characters with U+FFFD and limits the resulting text to
**512 Unicode scalars**, including an ellipsis when truncated. The limit is not
a byte slice and does not split UTF-8. It does not promise to preserve a truncated
grapheme cluster. Sanitization does not establish that text is free of personal
information; the adapter must satisfy the field restrictions before supplying it.

## State and metric semantics

The application distinguishes loading, successful empty data, successful populated
data and failure. A failure has a typed category with a static, generic display
message. Backend diagnostics, paths and exception text are not interpolated into
that message. The module does not log snapshots, input, failures or rendered
content.

Timeline state separately distinguishes:

- Unavailable: the adapter cannot supply a timeline.
- Loading: the adapter has not supplied the requested timeline yet.
- Ready and empty: the adapter supplied a timeline with no events.
- Ready with events: the adapter supplied those events.
- Failed: the timeline could not be loaded; display a generic message.

An updated timestamp is not evidence of a sequence of historical events. The TUI
does not synthesize correspondence, deadlines or status transitions. Summary and
draft counts describe the whole supplied snapshot, with its scope shown; a local
filter does not change those summary counts. Attention counts include only
records with known attention values; unknown values remain visible as unknown
when privacy permits. **Authority labels** counts distinct supplied, sanitized
labels, not authority identities. An optional total is labelled as a backend
total, rather than substituted for the number of loaded rows. For `Complete`,
the loaded count is also the total asserted by the caller's scope declaration.

Ordinary construction contains no demonstration records, fabricated totals,
activity messages or claims about successful network/security operations.

## Privacy and effects

Privacy is **redacted by default**. Snapshot content is hidden in every tab,
detail view, help overlay and filter result. This includes identifiers, titles,
authorities, timestamps, events and data-derived counts. The `/` filter is
disabled while redacted so it cannot disclose hidden content through matching.
Ready snapshots render the same static privacy message regardless of their
contents, counts, statuses or timeline availability.

The operator may press `p` outside text entry to reveal the supplied data in the
current terminal session and press it again to hide the data. This changes only
in-memory presentation state. It does not grant backend access, fetch more data,
persist a preference or change storage/network policy. Hiding clears the filter
query and closes the detail panel. Once content has been revealed, this control
cannot retract terminal captures made outside the program. Clearing/restoring the
terminal on exit is a separate runtime integration requirement.

A valid `set_snapshot` update preserves the session's existing reveal choice and
filter, retains selection by stable identifier where possible, and closes and
resets details. `set_loading` and `set_error` discard prior snapshot data and filtering,
close details, and return to redacted mode. An invalid new snapshot follows that
same error path. The initial `from_snapshot` state is redacted.

Drawing, filtering, selection, resize and error handling cannot initiate network
requests, database mutations, sync, exports, credential tests or file writes.
Input handlers modify presentation state and return only `TuiCommand::None` or
`TuiCommand::Quit`. No write, export or credential action is exposed by a
keybinding or advertised in help.

## Keyboard and rendering contract

| Input | Behavior |
| --- | --- |
| Tab / Shift+Tab | Move between the four tabs, wrapping at either end outside modal panels/text entry. |
| Up / Down / `j` / `k` | Navigate the current visible list, or scroll Summary/details/help. |
| Home / End | Select the first/last visible item, or the start/end of scrollable text. |
| Page Up / Page Down | Move within lists, Summary or details by a bounded page; move to the start/end in help. |
| Enter | Open the selected request's Timeline from Requests/Drafts; open details from Timeline; apply a filter during text entry. |
| `d` | Open selected request details, including the selected event on Timeline; close an already open detail panel. |
| `[` / `]` | Select the previous/next request on Timeline within the filtered snapshot. |
| `/` | Open a new empty filter in Requests when data is revealed and ready. No backend search occurs. |
| F1 / `?` | Open/close help outside filter entry and the detail panel. |
| `p` | Toggle local privacy outside text entry. |
| Ctrl+C / Ctrl+Q | Exit from every state, including help, filtering, loading and failure. |
| `q` / `Q` | Exit outside text entry, including help/details; enter ordinary query text while filtering. |
| Esc | Close help/details; clear and close a filter; return from Timeline to Requests; exit other ordinary views. |

List/event navigation clamps at the first/last item instead of wrapping. Release
events are ignored. Repeat events permit list/text navigation and filter typing,
but do not repeatedly toggle privacy or modal state. Filter input is bounded to
64 Unicode scalars. It matches sanitized titles, authority labels, static status
labels and identifiers with a case-insensitive substring comparison. Typing in
the filter cannot invoke ordinary navigation or write commands as a side effect.

The layout adapts to terminals at least 36 columns by 10 rows and provides a
static resize message below that size. Zero-area frames are safe. Selection has
a visible `>` row marker and bracketed active-tab label independent of color.
Navigation keeps the selected row visible; refresh and filtering clamp selection
to the available rows. Summary and detail panels scroll, so clipped table cells
do not make their retained text inaccessible. Wrapping uses the existing Ratatui
grapheme/width operations for both drawing and scroll bounds. Shrinking and
growing the terminal must not panic, disclose hidden data or leave selection
outside the visible content. Labels distinguish status and attention without
relying on color alone.

## Bounded local runtime increment (#448)

The baseline is master `39591dcf91bb50ed847b0d36bdc3bd670a7e60a7`, containing
merged #435 and the accepted #434/#440/#442 local APIs. No frozen retirement patch
or inaccessible bundle is treated as integrated. The shared Conductor registry,
Python retirement ledger, source acquisition, MCP and localization are unchanged.

Launch with `fyi-cli tui --db existing.db` (or the global `--db` before `tui`).
The default path remains `fyi_system.db`. Both stdin and stdout must be terminals;
redirected input/output and `TERM=dumb` fail before database or terminal access.
JSON output is rejected with a static JSON error on stderr. Success/ordinary quit
returns 0; launch, data, input, drawing, restoration or caught panic failure returns
1. Failed data remains a navigable static error view until quit. Error text never
contains database paths, SQL, terminal diagnostics or panic payloads.

The adapter calls the accepted `DbPool::open_local_existing(path, true)`; opening
never creates or migrates a database. A new `local_request_overview` repository
projection reuses `LocalRequestSummary`, joins `authorities` by the actual local
relationship, and reads the exact total and first 500 IDs in one read transaction.
SQL limits returned text before allocation; the existing presentation sanitizer
then applies its 512-scalar bound. The database is closed before the owned
snapshot is supplied. There is no refresh, polling of data, sync or external client.
For more than 500 requests, scope is `Partial` with the exact backend total; the
loaded rows are explicitly the first 500 local IDs. Filtering searches these rows.

Only local IDs, titles, actual authority names, typed status and supplied update
timestamps enter presentation. Unknown attention stays unknown. Request bodies,
tags, URLs, requester names, correspondence and raw status/error strings are not
supplied. Every timeline is `Unavailable`: this increment does not project local
history or stored acquisition provenance. In particular, legacy creation/update
timestamps are not converted into invented events. A future history increment
must qualify genuine events and the existing global 1,000-event limit explicitly.

The runtime draws an honest loading state and concurrently waits for data, input
and shutdown. Crossterm poll/read run on the same dedicated worker thread, with a
32-event bounded queue and a 50-ms poll timeout. Quit remains responsive while
data is pending. Drawing/key handling only operate on the owned `AppState`.
Initial navigation and each redraw receive the actual terminal size, and resize
events are forwarded. There is no idle animation or periodic redraw loop.

An effect-owning guard covers partial startup, ordinary quit, errors and unwind.
It clears the alternate screen, resets styles, shows the cursor, leaves the
alternate screen and restores raw mode, attempting remaining cleanup steps after
an error. The input queue is closed before its worker is joined. Ctrl+C/Ctrl+Q
remain key-based exits; externally delivered SIGINT/SIGTERM/SIGHUP are also handled
on Unix. Caught panics use static diagnostics after restoration; the previous
panic hook is restored. No claim covers uncatchable process termination, external
terminal capture deletion or a broken terminal transport.

TUI dispatch precedes tracing and heap-profiler initialization, including
all-features builds. Viewing a snapshot creates no profiling file and emits no
captured content through verbose diagnostics. Privacy starts redacted on every
launch; reveal remains a session-only explicit action.

The runtime verification and remaining manual work are specified in
[the runtime acceptance procedure](tui-runtime-acceptance.md). The sole
**Issue fixes and PR merges** coordinator allocates final reconciliation and merge;
implementation-head checks do not certify an untested future combined head.

## Verification and limits

Acceptance evidence is recorded in the
[TUI evidence record](../.conductor/tracks/rust-mcp-tui-implementation/tui-acceptance-evidence.md).
It must cover deterministic buffer snapshots, keyboard interaction, terminal
resizing, bounded input, unknown/partial data and privacy-negative cases, using
offline synthetic state. Final-head Actions remain the merge gate.

The runtime increment adds adapter, event-loop and Linux pseudo-terminal tests;
those results must be recorded separately from buffer tests. Neither establishes
screen-reader compatibility, the original
greater-than-30-FPS target, less-than-20-MB RAM target or manual terminal
verification. Those claims require separate evidence. Completion of this bounded
presentation slice does not close the combined CLI/MCP/TUI track.
