# Plan — Conductor planning: retain FYI Serde API and receipt contracts

- [ ] Inventory precise stable Serde API/receipt DTOs and dynamic raw-provider fields; freeze golden Alaveteli JSON/CSV/XML plus jurisdiction-specific malformed/duplicate-key fixtures.
- [ ] Independently test Unicode, large IDs, date precision, unknown extensions, float loss, malformed/oversized payloads and source identity/receipt consistency in the offline fixture harness.
- [ ] Confirm any duplicated manual JSON/CSV writing before suggesting consolidation. Require code reduction, manifest/feature stability, upstream ABI/schema and release compatibility; otherwise no-go.
- [ ] Run locked Cargo and hermetic mocked CLI/MCP tests, Clippy, MSRV and privacy/security audits on exact head; no live request or credential inclusion permitted. Existing AGENTS.md/CI and release owner remain authoritative.

