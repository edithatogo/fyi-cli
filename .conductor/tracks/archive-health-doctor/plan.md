# Plan: archive-health-doctor

## Phase 1: Report assembly

- [x] 1.1 Read capture ledger + manifest + sync_state to compute freshness/coverage.
- [x] 1.2 Stable JSON schema; truncate large gap lists with counts + samples.
- [x] 1.3 Tests: synthetic fixtures; determinism.

## Phase 2: CLI + docs

- [x] 2.1 `fyi archive-health` command + `--output`.
- [x] 2.2 Schema file + docs; cross-link to fyi-archive `doctor`.
