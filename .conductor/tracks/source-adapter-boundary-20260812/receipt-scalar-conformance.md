# Receipt scalar conformance — PR #420

Related: #310 and #309. Both remain active; this is a bounded increment within
source-adapter-boundary-20260812, not full schema parity or archive cutover.

## Defects and contract

The Rust verifier previously accepted arbitrary nonempty adapter identifiers and
any existing rate-limit name value. The packaged v1 schema requires an adapter
identifier matching `^[a-z0-9][a-z0-9._-]*$`, and a rate-limit name that is a string
or null. The implementation now checks both constraints without a new dependency.
An empty string remains valid for the rate-limit name, as specified by the schema.

The shared corpus `conformance/acquisition/receipt-v1-scalar-cases.json` contains
21 valid/invalid cases. Python checks the actual packaged Draft 2020-12 schema;
Rust patches the Python-origin golden receipt and recomputes the receipt digest
before verification, so a stale digest cannot hide the scalar-validation defect.

## Executed red evidence

Test-only commit: `e501a62c9922efc030821bccff140997cd931e16`.

Hosted CI run `43419847030`, Workspace Tests job `131866809712`, compiled and ran
`cargo test --workspace --all-features`. Both new Rust tests failed:

- `adapter_identifiers_match_packaged_schema`: `adapter-uppercase` returned
  `Ok(())`, but the case requires rejection.
- `rate_limit_names_match_packaged_schema`: `rate-name-number` returned `Ok(())`,
  but the case requires rejection.

The Python Tests job `131866809340` passed on the same test-only commit. A focused
local Python 3.13.5 run of the 21 schema cases also passed. The local copies of the
schema and golden fixture were checked against their Git blob hashes; neither the
schema validator nor application behavior was stubbed. No local Rust execution
is claimed. Final-head green evidence belongs in the PR checks and merge record.

## Remaining acceptance

- Full receipt schema/canonicalization parity, including numeric, Unicode and
  date-time edge cases, remains open.
- Complete network-command coverage, cross-process crash/Windows verification,
  raw/WARC linkage and downstream shadow parity remain open.
- The offline AI security lane still requires its provisioned self-hosted runner;
  a queued job is not a passed security scan.
