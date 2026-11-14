# Receipt shortest-decimal midpoint conformance — #310

## Scope and hypothesis

Follow up the merged #426 numeric correction without changing receipt v1,
network commands, evidence files or the remaining #310 acceptance gates.
Rust upstream issue rust-lang/rust#130936 reports shortest-decimal formatting
that resolves some decimal midpoint ties differently from CPython. Its status
alone does not prove the current compiler/path fails; the regression must run.

## Plan

- [x] Freeze the 15 upstream counterexamples as binary64 bits plus independently
  generated CPython 3.13.5 canonical bytes, and create a self-digested receipt.
- [x] Verify all numeric Python oracles locally: 79 tests passed (62 existing,
  17 additional). Python is an independent test oracle, not a new application adapter.
- [ ] Run the two new Rust regression tests against the unchanged #426 formatter.
- [ ] If the mismatch reproduces, use Serde JSON's default shortest digits and
  transform only their notation into Python's fixed/scientific conventions.
  Reuse the existing dependency; no custom decimal-rounding algorithm or fallback.
- [ ] Verify midpoint and original corpora, exact parsing, signed zero, nested
  types, bit-preserving property tests and CLI evidence preservation.
- [ ] Inspect final-head hosted workspace, MSRV, security, coverage and other
  checks; merge only after success and record the actual checkpoint.

## Boundaries

Existing evidence must never be rewritten. Older incompatible Rust receipt
hashes require explicit reconciliation, not silent acceptance. Full schema
parity, arbitrary-precision integers, duplicate-key wire handling, all-command
receipt coverage and downstream shadow parity remain open #310 work.
Do not close #310 or enable archival cutover on this increment.

Source: https://github.com/rust-lang/rust/issues/130936
