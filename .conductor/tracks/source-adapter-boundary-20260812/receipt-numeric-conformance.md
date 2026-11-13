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

- [ ] Reproduce current numeric byte/hash mismatches with a frozen CPython 3.13.5
  corpus: 29 explicit boundary cases and 31 seeded finite bit patterns (seed 310).
- [ ] Verify the corpus independently in Python; test Rust numeric parsing,
  canonical bytes, an entire self-digested Python receipt, nesting and tampering.
- [ ] Use the existing Serde serializer with a narrow finite-float formatter;
  retain string escaping and integer serialization. Enable exact float parsing
  using serde_json's existing float_roundtrip feature rather than a new parser.
- [ ] Run property-based bit-preserving round trips, workspace tests, strict
  formatting/lint, Python tests, schema fixtures and all final-head hosted checks.
- [ ] Record red/green verification evidence and merge only the tested head.

## Review focus and boundaries

Cover signed zero; subnormals and largest finite doubles; exponents on either
side of -4 and 16; integer versus floating-point representation; nested arrays,
objects and Unicode escaping. Keep old immutable evidence unchanged. Full JSON
Schema equivalence, arbitrary-precision integers, duplicate-key wire parsing,
all-command receipt coverage and downstream shadow parity remain separate #310
requirements. Do not close #310 or claim end-to-end archival parity here.

## Evidence

Local environment: Python 3.13.5 is available; Rust is not installed and GitHub
cannot be resolved by the local container. Rust red/green and full workspace
verification therefore use hosted PR checks, not a claimed local build.
