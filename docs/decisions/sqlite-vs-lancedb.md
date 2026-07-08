# Decision Record: SQLite vs. LanceDB

**Status:** Decided — keep SQLite as the primary datastore.
**Date:** 2026-07-08

## Question

Should `fyi-cli` replace its SQLite-based storage (`crates/fyi-core` migrations: `requests`,
`correspondence`, `authorities`, `sync_metadata`, `sync_outgoing_queue`) with LanceDB, given that
several of the bleeding-edge R&D ideas (semantic search over archived corpora, embeddings-based
similarity) are vector-search workloads that LanceDB specializes in?

## Decision

**Keep SQLite** as the primary transactional/relational store. Do **not** replace it with
LanceDB. If/when semantic search over archived FOI corpora is implemented (see the
`bleeding-edge-features` track), add a **narrow, optional vector-search layer** — most likely
[`sqlite-vec`](https://github.com/asg017/sqlite-vec) (a SQLite extension for vector similarity
search) rather than introducing a second database engine.

## Rationale

1. **Workload mismatch.** fyi-cli's core data (requests, correspondence, authorities, sync
   queues) is relational, transactional, and consistency-sensitive (foreign keys, sync/conflict
   resolution, offline queueing). SQLite is a mature, ACID-compliant, embedded RDBMS — exactly
   the right tool. LanceDB is a columnar, vector-native store optimized for approximate nearest
   neighbor (ANN) search over embeddings; it is not designed as a general-purpose relational
   store and has no equivalent to SQLite's foreign-key/transaction guarantees needed for sync
   conflict handling.
2. **Operational simplicity.** fyi-cli is a single-binary CLI/MCP tool. SQLite ships embedded
   with zero external dependencies (already the case). Introducing LanceDB (which itself vendors
   Lance's columnar format and, in its cloud/server form, has separate operational concerns) adds
   a second storage engine, a second migration story, and a second backup/restore story for very
   little benefit unless/until vector search is actually implemented.
3. **No current vector-search feature exists.** Semantic search/embeddings work is currently only
   a *proposed* bleeding-edge R&D item (see `.conductor/tracks/bleeding-edge-features/`), not an
   implemented feature. Adopting a vector database ahead of an actual embeddings pipeline would
   be premature infrastructure.
4. **`sqlite-vec` closes the gap when needed.** `sqlite-vec` is a lightweight SQLite extension
   providing `vec0` virtual tables for storing/querying vector embeddings directly inside the
   existing SQLite database file — no second engine, no second connection pool, no second backup
   path. It integrates with the existing `sqlx`-based SQLite access layer used throughout
   `fyi-core`.
5. **Multi-jurisdiction DB partitioning already assumes SQLite semantics** — the
   `jurisdiction-abstraction-core` track's `instance_id` column design, foreign keys, and
   sync-conflict reconciliation logic are all built on relational primitives that would need to
   be re-architected on a document/columnar vector store.

## When to revisit

Revisit this decision if/when:
- The `bleeding-edge-features` semantic-search item is scheduled for implementation and
  `sqlite-vec` proves inadequate at the corpus scale involved (very large embeddings, need for
  distributed/cloud-native ANN indexing).
- A future requirement needs true columnar/analytical query performance over large archived
  corpora that SQLite cannot reasonably serve even with extensions.

## Alternatives considered

| Option | Verdict |
|---|---|
| Replace SQLite entirely with LanceDB | Rejected — wrong workload fit, would require re-architecting relational/transactional logic |
| Run SQLite + LanceDB side-by-side (dual storage) | Rejected for now — unnecessary operational complexity with no implemented feature requiring it |
| SQLite + `sqlite-vec` extension (single engine) | **Selected approach**, deferred until semantic search is actually built |
