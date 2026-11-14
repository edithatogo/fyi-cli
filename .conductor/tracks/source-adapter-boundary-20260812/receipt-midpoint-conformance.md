# Receipt shortest-decimal midpoint conformance — #310

## Scope and reproduced defect

Follow up the merged #426 numeric correction without changing receipt v1,
network commands, evidence files or the wider acceptance gates. Rust upstream
issue rust-lang/rust#130936 reports shortest-decimal formatting that resolves
some midpoint ties differently from CPython. The actual Rust 1.99.1 path was
then tested, rather than relying on that report alone.

## Plan and evidence

- [x] Freeze 15 upstream counterexamples as binary64 bits plus independently
  generated CPython 3.13.5 canonical bytes, and create a self-digested receipt.
- [x] Verify the numeric Python oracles locally: 79 tests passed (62 existing,
  17 additional). Python is an independent test oracle, not a new application adapter.
- [x] Run the two new Rust regressions against the unchanged #426 formatter.
  Red head `f5ce31282701e2bcd8e2741f770f65d9ddd63d2b`, CI run `43534639113`,
  Workspace Tests job `132260548311`: 0 passed, 2 failed. All 15 byte comparisons
  differed; the independently hashed Python receipt failed with a digest mismatch.
  Existing preceding CLI and core unit suites passed.
- [~] Use Serde JSON's default shortest-digit output and transform only its
  fixed/scientific notation into Python conventions. No new dependency,
  handwritten decimal rounding, logarithms or float scaling is introduced.
- [~] Retain both failing regressions unchanged and expand CLI verification to
  both exponent and midpoint receipt fixtures, asserting evidence is unchanged.
- [ ] Verify final-head workspace, MSRV, security, coverage and other checks;
  record the actual successful head and merge checkpoint in the PR evidence.

## Design and review notes

The scalar `serde_json::to_string(&value)` call uses Serde's default formatter,
not the custom receipt formatter, so it is not recursive. Sign, decimal-point
position and exponent are separated from its shortest significant digits.
Trimming insignificant zeroes changes notation only; signed zero and floating
point `.0` are retained. Serde still owns integer/string/container serialization
and exact float parsing remains enabled. Existing boundary/property tests remain.

A numeric round trip alone did not expose this defect: both decimal spellings
round to the same binary64 value, but yield different cryptographic digests.
The independent Python bytes and whole-receipt fixture are therefore essential.
A receipt self-digest is an integrity check, not a digital signature or proof of
source authenticity.

## Boundaries

Existing evidence is never rewritten. Older incompatible Rust receipt hashes
require explicit reconciliation, not silent acceptance. Full schema parity,
arbitrary-precision integers, duplicate-key wire handling, all-command receipt
coverage and downstream shadow parity remain open work for issue #310.
The issue stays active and archival cutover remains disabled.

Local Rust execution is unavailable; hosted runs supply the actual Rust red/green
and full-repository verification. Source: https://github.com/rust-lang/rust/issues/130936
