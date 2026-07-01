# Track: archival-content-diff

## Objective

Provide **content-addressed change detection** over the archive so the prospective
sync can capture *only* what changed. Compute, between two sync points, the sets of
**added**, **updated**, and **removed** requests keyed by SHA-256 of the captured JSON
payload, and emit `latest_changes.json`.

## Background / distinction from `offline-sync-engine`

The existing `offline-sync-engine` track is about the **personal tracker** —
push/pull/conflict resolution for requests *you* are tracking, with field-level
three-way merge and dirty-flagging. This track is different and complementary: it is
about the **archival corpus** — set-level diff of the *entire* captured site by
content hash, to drive an incremental, churn-free mirror sync. It has no push path
(read-only) and no conflict resolution (the live site is authoritative).

## Scope

- A content hash definition: `content_sha256 = sha256(canonical_json(request_payload))`
  where `canonical_json` is deterministic (sorted keys, stable separators).
- `fyi diff --since <timestamp|cursor>`:
  - Compare the current captured set against the previous manifest's hashes.
  - Emit added (new request_id), updated (same id, changed hash), removed (id gone
    from upstream).
  - Validate output against `schemas/changes.schema.json` (lives in fyi-archive).
- High-water mark: the diff cursor advances only after a successful capture+publish;
  persisted alongside `sync_state.json`.
- Idempotency: a no-change run emits empty sets and does not churn the mirror.

## Out of scope

- Capture (that's `faithful-archive-capture`); this track consumes captured hashes.
- Publishing (that's fyi-archive).

## Acceptance criteria

- [x] `fyi diff` correctly classifies added/updated/removed across a synthetic
      before/after fixture (hypothesis-generated mutations).
- [x] Canonical hashing is stable across runs and unaffected by key reordering.
- [x] `latest_changes.json` validates against `schemas/changes.schema.json`.
- [x] An empty-diff run produces empty sets and does not alter the high-water mark.
- [x] No live network required for the diff itself (operates on captured hashes).

## Risks

- Non-deterministic upstream JSON (whitespace, key order) → canonicalisation handles
  it; raw payload retained for audit.
- Apparent "updates" from cosmetic changes → expose a `--ignore-keys` allowlist
  (future), keep raw hashes now.
