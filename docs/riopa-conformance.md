# RIOPA conformance profile

The files in `conformance/riopa/` describe how the native `fyi-core`
provenance chain maps to the RIOPA capture profile. The adapter is additive:
native chain records remain the evidence of record, while the mapped event
stream is a projection for interoperability.

`mapping-profile-v1.json` is the language-neutral field map and conforms to
RIOPA adapter-mapping schema version `1.0.0`. Its `repository` is the portable
`owner/name` identifier required by that schema. Every entry names the native
field, its nullable RIOPA destination, one of the classifications
`exact`, `approximate`, `extension-only`, or `unmapped`, a rationale, and a
JSON Pointer into the native evidence fixture. The profile's
`source_revision` identifies the fyi-cli revision whose native contract was
reviewed.

`native-provenance-fixture-v1.json` contains a synthetic, valid fyi-cli
hash-chain record and emission context. `conformance-report-v1.json` contains
the corresponding output from `emit_riopa_event_stream` plus classification
counts. A central conformance runner can parse these JSON files without Rust
tooling.

The classifications are semantic:

- `exact`: the native value is copied without conversion.
- `approximate`: the value is retained but its scope or meaning is broader.
- `extension-only`: the value is preserved in an fyi-cli extension because
  the common fields have no lossless destination.
- `unmapped`: the value is deliberately absent and the rationale identifies
  the retained evidence or gap.

The checked-in report is test evidence only. It does not assert that a live
FYI capture was performed or that a central RIOPA deployment accepted it.
