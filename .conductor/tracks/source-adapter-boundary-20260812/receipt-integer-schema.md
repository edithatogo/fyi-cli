# Receipt integer-schema conformance increment — 14 November 2026

Owner: existing source-adapter-boundary-20260812 track; issue #310; PR #432.
Status: implementation submitted, final-head hosted verification required.

## Defect and contract

JSON Schema 2020-12 treats a number with zero fractional part as an integer.
The Python validator therefore accepts `1.0`; the Rust accessor previously
required an integer-encoded serde_json value. The shared accessor now accepts
finite, nonnegative, integral values without rewriting the receipt or digest.
Native u64 values retain their exact precision, including u64::MAX. Floating
values must be strictly below 2^64 before conversion; using u64::MAX as an
inclusive float bound would silently saturate. Negative zero is accepted.

This is not unrestricted JSON Schema parity: Rust retains checked u64 counters
and stronger totals/privacy/time-order semantics. The shared corpus separately
records schema validity and Rust acceptance, including these differences.

Reference: https://json-schema.org/understanding-json-schema/reference/numeric

## Test-first evidence

- Test-only source checkpoint: 61a9b3e0ce7b2b86481931dc7dff00745e196ea8.
- CI run 43551696570, Workspace Tests job 132315378633: four new tests failed
  with `receipt missing nonnegative integer bytes`; the property test reduced
  to count=0. The negative fractional and overflow tests passed.
- The Python Tests job 132315378290 passed independently on that checkpoint.
- Test fixture correction 7e3c90a6a20eeb6ba3bf319016727e8409212eb7 isolates
  negative-zero encoding from unrelated retry-count consistency.

## Acceptance

The shared 18-case corpus covers integral decimal counters, negative zero,
retry counts, native u64 maximum, the largest binary64 value below 2^64,
capacity rejection, fractions, negative values, booleans, strings, nulls,
status bounds and inconsistent totals. Rust tests exercise strict wire parsing,
actual raw/result byte verification and immutable publication without changing
numeric representation; two 128-case property tests cover integral/fractional
values. Extend the existing Linux/macOS/Windows portability job with this suite.
No new dependency, receipt version, Python application feature or HTTP client.

## Remaining parent acceptance

#310 and #309 remain open. This increment does not establish all-command
receipts, CDX/replay migration, raw-header/attachment/WARC/WACZ linkage,
mid-write fault injection, DNS rebinding protection or downstream NZ shadow
parity. Preserve `archive_cutover_allowed=false` and retained Python paths.
