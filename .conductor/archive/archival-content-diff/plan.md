# Plan: archival-content-diff

## Phase 1: Canonical hashing

- [x] 1.1 `canonical_json` (sorted keys, stable separators) + `content_sha256`.
- [x] 1.2 Tests: stable across key reorder/whitespace; hypothesis round-trips.

## Phase 2: Set diff

- [x] 2.1 Compare current captured set vs previous manifest hashes →
      added/updated/removed.
- [x] 2.2 Emit `latest_changes.json` matching `schemas/changes.schema.json`.
- [x] 2.3 Tests: synthetic before/after fixtures; removed detection; idempotent empty.

## Phase 3: Cursor + CLI

- [x] 3.1 `fyi diff --since <cursor>`; high-water mark persisted, advances only on
      success.
- [x] 3.2 Unit tests for cursor semantics.
- [x] 3.3 Docs: distinction from `offline-sync-engine`; read-only posture.
