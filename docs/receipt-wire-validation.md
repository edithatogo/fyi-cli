# Reading acquisition receipt JSON safely

`verify-acquisition-receipt` now uses the shared Rust `receipt_wire` ingestion
boundary before the existing v1 schema and self-digest checks. Consumers of
external bytes should call `fyi_core::receipt_wire::parse_receipt_bytes` or
`read_receipt`, rather than first deserializing into `serde_json::Value`.
A pre-parsed map cannot reveal whether duplicate object members were discarded.

The boundary rejects identical or conflicting duplicate names at every object
level, including objects inside arrays and free-form `request_bounds` data.
Names are compared after JSON escape decoding. Repeating the same field name in
different objects is valid. Duplicate diagnostics do not echo field names or values.
Malformed JSON, trailing documents/data, invalid UTF-8 and excessive nesting fail
closed. Existing numeric canonicalisation, exact parsing, schema validation and
self-digest verification remain in force.

Receipt JSON is limited to **16 MiB including whitespace**. The reader consumes
at most 16 MiB + 1 byte, rejects overflow and leaves files unchanged. The bound
is an ingestion policy, not a new receipt schema version or a claim about constant
heap overhead. A receipt at exactly the limit is accepted when otherwise valid.
This limit does not apply to separately captured HTTP bodies or result projections.

Harmless whitespace and key-order variations remain accepted. Verification does
not rewrite receipts into canonical form or silently repair ambiguous input.
The existing Python-generated fixtures remain compatibility controls.

This is not full arbitrary-precision JSON number support, origin authentication,
a digital signature, payload availability proof, or completion of all-command
acquisition coverage. Issue #310 and downstream archive acceptance remain active.
