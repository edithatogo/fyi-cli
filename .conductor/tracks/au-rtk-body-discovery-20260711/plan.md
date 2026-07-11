# Implementation Plan: AU RightToKnow body discovery

## Phase 1 — Contract and implementation

- [x] Define JSONL body contract and CLI options.
- [x] Normalize catalog tags and serialize stable JSONL records.
- [x] Preserve provenance-bearing JSON mode and expose the shared limiter name.
- [x] Add unit and parser tests for valid, malformed, and encoded catalog rows.
- [x] Run focused and repository quality gates.

## Phase 2 — Handover

- [ ] Open a focused PR closing GitHub issue #135.
- [ ] Record CI evidence and downstream consumer guidance.
