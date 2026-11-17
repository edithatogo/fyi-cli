# Issue #66: 2026 holiday calendar evidence

## Scope and baseline

PR: [#491](https://github.com/edithatogo/fyi-cli/pull/491).
Implementation checkpoint: `4a4fbd7fc1e54a39a018ce249bf60fab9a4f414e`.
Implementation branch: `codex/issue-66-holiday-calendars-20261117`.
Baseline: `1f0ae6d0b00663d52ccb65904152ca4e933b3486` (`master`).
This increment extends the existing v1 catalog introduced by #470/#473; it does
not create a second deadline engine or claim completion of #65 or parent #39.

The original 13 starters had no per-date source evidence. ACT/NSW omitted the
additional Anzac Day on 27 April, Victoria omitted AFL Grand Final eve on
25 September, and Scotland omitted the exceptional 15 June bank holiday. `GB`
was an ambiguous key for England/Wales. JSON arrays silently lost duplicates
when deserialized into `BTreeSet`, contradicting `uniqueItems` in the schema.

## Delivered source and contribution scope

- 37 source-checked, explicitly scoped 2026 calendars: 13 NZ national/anniversary
  profiles, 20 AU common/state/local profiles, and four UK constituent schedules.
- 444 named date records linked to 30 primary-source records checked on
  17 November 2026, with exact revisions, covered intervals, weekend assumptions,
  source IDs and scope limitations. Source metadata lives beside the runtime
  catalog; the public v1 `HolidayCalendar` structure is retained.
- Whole-day, observed, regional and exceptional dates are distinguished. WA
  local King's Birthday dates replace the state default; partial-day and
  bank-only holidays are not promoted to general full-day exclusions.
- Existing DE/ES/FR arrays/revisions remain unchanged and explicitly unreviewed.
  They are the only permitted legacy exceptions in the provenance contract.
- Strict offline schemas and cross-file validation reject missing/mismatched
  evidence, unknown references, duplicate identifiers/events, invalid chronology
  and date drift. Distinct holidays sharing one date remain representable.
- Rust parsing rejects duplicate dates/weekend codes and malformed contributed
  scope identifiers. Unknown regions have no fallback. Bare embedded `GB` is
  replaced by `GB-ENG` and `GB-WLS` with a documented migration.
- [Calendar guide](../../../../docs/holiday-calendars.md) documents all keys,
  limitations, source qualifications, custom JSON, year rollover and contributions.

## Red/green evidence

On the original catalog, the new Python date/scope regressions failed on missing
`2026-04-27` in ACT and absent `AU-QLD`. The first provenance test failed because
the evidence file did not exist. After implementation, both files pass.

An independent review reproduced a trailing-newline identifier mismatch: the
JSON Schema dollar anchor accepted `NZ-AUK\n` while Rust rejected it. Both
schemas now explicitly reject whitespace in identifiers, with four suffix
regressions. The reviewer also independently compared all eight Australian
state arrays and all eleven local differences to the source packet, including
the WA replacements, with no remaining data defect found in that scope.

Executed locally with Python 3.15.2 / pytest 9.2.0 and Rust toolchain 1.100.0:

| Check | Result |
| --- | --- |
| `python -m pytest tests/test_holiday_calendar_catalog_schema.py tests/test_holiday_calendar_provenance.py -q` | 64 passed |
| `python scripts/verify_holiday_calendars.py` | PASS |
| Ruff lint and format checks on the changed script/tests | PASS |
| `cargo fmt --all -- --check` | PASS |
| `python scripts/verify_frontier_conductor.py` | PASS |
| `git diff --check` | PASS |
| Bounded Rust source harness | 24/24 tests and Clippy with warnings denied passed; unmodified actual module/data and test copies; all 45 registry dependency versions/checksums match the repository lock |
| Full-package Rust execution | Dependency compilation was terminated by SIGKILL in existing `serde_core`, including single-job/debug-disabled retry; no Rust execution pass is claimed here |

The focused Rust file adds nine tests covering complete state/UK date sets,
observed/exceptional-day arithmetic, regional replacements, all admitted NZ
anniversaries, unknown scopes, year boundaries, duplicate JSON, key syntax and
the 256 KiB boundary. Hosted execution and final PR checks must be recorded in PR #491 before merge.
The source harness uses Rust 1.100.0, `CARGO_INCREMENTAL=0`, debug info disabled
and one job only in its own environment; repository profiles/gates are unchanged.
Its source SHA-256 is `e5712a33b1b8696c49c3b7d5ebd2693a44c5ae8308748706b80cb49964cf4776`;
its embedded catalog SHA-256 is `bb229d26e607621ef35648f841f28a49b0b11d9efbc3ea65bcb625bc28e0a397`.

## Acceptance boundaries

Every result remains `ValidationStatus::Unvalidated`. Source data checking is
not human legal approval. Operational applicability, law-specific exclusions,
DE/FR/ES source review, future years and unsupplied localities remain separate.
There is no implicit Chatham Islands, whole-Otago or whole-Westland calendar.
The South Canterbury council source was retrieved through its search index;
Greymouth uses explicit national 2026 data corroborated by a council rule. Those
qualifications are recorded in provenance and the guide.

The concurrent documentation work in PR #490 owns broader #67 locale/template
guidance. This branch changes only its predecessor's calendar section and adds
the dedicated source/contribution guide. When combining the PRs, retain #490's
broader examples and use this branch's catalog counts, `GB` migration, stricter
parser behavior and provenance-validator instructions for the calendar section.

The existing track stays `in_progress` with `archive_allowed: false`. Manual
phase review and legal-applicability acceptance remain unchecked; no automatic
issue closure or archive is requested by this source-data increment.
