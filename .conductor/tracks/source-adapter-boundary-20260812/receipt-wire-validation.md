# Strict receipt wire validation — implementation plan for #310

**Goal:** Reject ambiguous receipt JSON before canonicalisation can discard duplicate keys.
**Architecture:** Reuse Serde's tokenizer and existing schema/self-digest verifier. Add a
recursive duplicate-aware value deserializer and a bounded reader; use them at the
existing offline CLI file boundary. No new application dependency or command.
**Tech stack:** Rust, serde, serde_json, existing fyi-core receipt contracts.
**Spec:** Existing source-adapter spec and receipt-v1 integrity contract.

## Scope and design decisions

The existing CLI reads an entire file then deserializes into `serde_json::Value`.
The hypothesis is that duplicate keys are silently collapsed before verification.
A strict raw-byte boundary must reject identical and conflicting duplicate names,
including escape-equivalent names and duplicates in arrays/free-form objects.
An already constructed Value cannot prove its original wire input was unambiguous.

Use a Serde Visitor, not regex or a handwritten JSON tokenizer. Preserve exact
numeric parsing and canonical output from #426/#428. Continue accepting harmless
whitespace/key-order variations: requiring byte-canonical input would break the
existing pretty-printed Python fixture without addressing the actual contract.

The new wire ingestion policy limits a receipt document to **16 MiB**, including
whitespace, reading at most limit + 1 bytes. This bounds input buffering, not the
size of separately captured HTTP payloads, and does not claim constant heap usage.
Keep Serde's recursion protection. Errors must not echo rejected duplicate values.

## Tasks and review focus

- [ ] Run eight offline CLI tests against the unchanged implementation: six duplicate
  classes, oversized document, and valid pretty-printed compatibility control.
- [ ] Add `fyi_core::receipt_wire::{parse_receipt_bytes, read_receipt}` and
  `MAX_RECEIPT_BYTES`; wire the CLI to the shared reader.
- [ ] Exercise malformed UTF-8/JSON, trailing data, deep nesting, complete schema/hash
  verification, payload-preservation and reader errors. Test the exact size boundary,
  byte budget and nested duplicate scope without a live network.
- [ ] Keep prior numeric/receipt test suites unchanged; run strict final-head CI,
  MSRV, coverage, security and Conductor checks before merging.
- [ ] Record the actual red/green and merge evidence; keep issue #310 active.

The CLI change is confined to receipt input ingestion. Existing evidence is never
rewritten. This does not implement all-command receipt coverage, arbitrary-precision
integers, archival cutover or downstream shadow parity. Local Rust is unavailable;
hosted checks provide Rust execution evidence, not a claimed local build.
