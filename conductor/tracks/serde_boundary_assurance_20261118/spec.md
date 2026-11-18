# Specification — Conductor planning: retain FYI Serde API and receipt contracts

## Verified current position
- `crates/fyi-core/Cargo.toml` declares `serde` derive, `serde_json` with `float_roundtrip` and `raw_value`, `csv`, `quick-xml` and `toml`. `crates/fyi-core/src/api.rs` defines typed Alaveteli request and correspondence DTOs.
- Existing typed Serde is appropriate; `csv` handles tabular parsing, `quick-xml` handles XML syntax, `toml` handles config and `serde_json` handles JSON encoding. None is replaceable with Serde traits alone.
- Security issue #495 concerns other potentially unmaintained packages, not evidence that Serde can replace them; do not blur dependency-security work into a serialization migration.

## Safety decision
Preserve offline/no-external-send boundary, request/response source evidence, jurisdiction differences, legal/privacy constraints, inexact portal timestamps and unknown provider fields. No mandatory `deny_unknown_fields` on forward-compatible external APIs. A typed wire DTO is not licence, delivery, or evidentiary proof.
