# Strict receipt wire validation — implementation plan for #310

**Goal:** Reject ambiguous receipt JSON before canonicalisation can discard duplicate keys.
**Architecture:** Reuse Serde's tokenizer and existing schema/self-digest verifier. Add a
recursive duplicate-aware value deserializer and a bounded reader; use them at the
existing offline CLI file boundary. No new application dependency or command.
**Tech stack:** Rust, serde, serde_json, existing fyi-core receipt contracts.
**Spec:** Existing source-adapter spec and receipt-v1 integrity contract.

## Reproduced defect

Tests-only head `6c1f655073c6da4a85ac479d12148abc4b88c2b9` compiled and ran
against the unchanged implementation. CI run `43538646600`, Workspace Tests
job `132273706019` reported **7 failed, 1 passed**: all six ambiguous duplicate
cases and the oversized document were accepted; the valid compatibility control
passed. Existing preceding CLI and numeric tests also passed.

## Scope and design decisions

Use a Serde Visitor, not regex or a handwritten JSON tokenizer. Reject identical
and conflicting duplicate decoded names, including escape-equivalent names and
objects inside arrays/free-form bounds. An already constructed Value cannot prove
its original wire input was unambiguous. The new public Rust byte/reader APIs
perform existing schema/self-digest checks before returning a value.

Preserve exact numeric parsing and canonical output from #426/#428. Continue
accepting harmless whitespace/key-order variations: requiring byte-canonical input
would break the existing pretty-printed Python fixture without addressing the gap.

The wire ingestion policy limits a receipt document to **16 MiB**, including
whitespace, reading at most limit + 1 bytes. This bounds input buffering, not
parsed-tree heap overhead or separately captured HTTP bodies. Keep Serde's recursion
protection. Duplicate diagnostics must not echo rejected names or values.

## Tasks and review focus

- [x] Reproduce all eight offline CLI tests against the unchanged implementation;
  keep the seven failing tests unchanged as regression checks.
- [~] Add `fyi_core::receipt_wire::{parse_receipt_bytes, read_receipt}` and
  `MAX_RECEIPT_BYTES`; wire the CLI to the shared verified reader.
- [~] Add 14 core tests covering malformed input, trailing data, deep nesting,
  exact size boundary, finite byte budgets, read errors, complete schema/hash checks,
  nested duplicate scope, original numeric fixtures and property-based duplicates.
- [ ] Verify strict final-head CI, MSRV, coverage, security and Conductor gates.
- [ ] Record the actual successful head and merge evidence; issue #310 stays active.

No network command, source permissions, schema version or evidence file is changed.
This does not implement all-command receipt coverage, arbitrary-precision integers,
archival cutover or downstream shadow parity. Local Rust is unavailable; hosted
checks provide actual Rust execution evidence, not a claimed local build.
