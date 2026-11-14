# Receipt numeric canonicalization — #310

## Contract and scope

The v1 canonical contract is the retained Python serializer:
`json.dumps(sort_keys=True, separators=(",", ":"), ensure_ascii=False, allow_nan=False)`
encoded as UTF-8 with one LF. Rust must not issue or reject receipts solely because
its default finite-float notation differs. This increment changes no network
command, provider permissions, schema version, existing evidence, or archive-cutover gate.

Implementation owner: this existing source-adapter track; parent #309.
The application remains Rust-first. Python additions are independent test oracles only.

## Implementation plan

- [x] Reproduce current numeric byte/hash mismatches with a frozen CPython 3.13.5
  corpus: 29 explicit boundary cases and 31 seeded finite bit patterns (seed 310).
  Red checkpoint: `924684d`; hosted Workspace Tests job `132248038355`, run `43530904310`.
- [x] Independently validate all 60 oracle cases, exact round trips and the signed
  receipt in Python: 62 tests passed locally on Python 3.13.5.
- [~] Use the existing Serde serializer with a narrow finite-float formatter;
  retain string escaping and integer serialization. Enable exact float parsing
  using serde_json's existing float_roundtrip feature rather than a new parser.
- [~] Add a deterministic regression for the minimized property-test failure,
  a nonfinite formatter guard and an offline CLI receipt-verification test.
- [ ] Run final-head workspace tests, property tests, strict formatting/lint,
  Python tests, schema fixtures and all hosted merge gates before merging.

## Root cause and red evidence

The five new Rust tests compiled and all failed against the unchanged implementation.
The serializer emitted `1e-7` where Python's v1 contract requires `1e-07`; a complete
valid Python receipt consequently failed its self-digest check. Default float
parsing changed bits `43abc16d674ec801` to the adjacent value. Property testing
independently minimized another failing input to bits `12099760289525035016`.
The preceding existing CLI/core suites passed. Rustfmt separately identified a
line-wrap issue in the new test constant; that formatting is corrected, not waived.

## Compatibility and review boundaries

The formatter changes notation only, using shortest round-trip digits and the
Python exponent thresholds/sign/padding. It does not round via logarithms or
scale floating-point values. Keep signed zero, exact integer types, strings and
nested containers intact. The parser feature is additive; no package version or
new library dependency is introduced.

Prior Rust-generated receipts with non-Python float spelling may have incompatible
v1 hashes. Preserve them as original evidence for explicit operator reconciliation;
never rewrite or silently accept them via a legacy fallback. The checked-in old
Python receipt remains an unchanged regression oracle. Existing raw payload and
normalized result files are never modified by this correction.

Full JSON Schema equivalence, arbitrary-precision integers, duplicate-key wire
parsing, all-command receipt coverage and downstream shadow parity remain separate
#310 requirements. Do not close #310 or claim end-to-end archival parity here.

Local environment: Python 3.13.5 is available; Rust is not installed and GitHub
cannot be resolved by the local container. Rust red/green and full workspace
verification use hosted PR checks, not a claimed local build.
