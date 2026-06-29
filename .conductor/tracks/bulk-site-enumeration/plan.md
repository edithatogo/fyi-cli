# Plan: bulk-site-enumeration

## Phase 1: Polite HTTP foundation (shared)

- [~] 1.1 Async `httpx` client with contactable `User-Agent` + `robots.txt` cache.
- [~] 1.2 `httpx-limiter` token-bucket (~1 req/s + jitter) + `tenacity` backoff on
      429/5xx.
- [~] 1.3 Tests: robots disallow honoured; injected 429 backs off; rate cap enforced
      (timed).

## Phase 2: Feed-based discovery

- [x] 2.1 Search-feed walker: build a search URL for a date window (+authority/status),
      paginate the `.json`/Atom entries to completion.
- [x] 2.2 Emit JSONL of `{request_id, url_title, title, authority, state, created_at}`.
- [x] 2.3 Resume/checkpoint: discovery cursor persisted so an interrupt re-runs cheaply.
- [x] 2.4 Tests with mocked HTTP fixtures (multi-page, empty page, redirect-to-url_title).

## Phase 3: ID backfill (optional gap-fill)

- [x] 3.1 `--backfill-ids`: probe a numeric ID range, follow the `url_title` redirect,
      skip 404s, emit the same JSONL shape.
- [x] 3.2 Reconciliation helper comparing feed-walk vs backfill sets per window.
- [x] 3.3 Tests for redirect-follow + 404 skip + dedup.

## Phase 4: Authorities

- [x] 4.1 Locate + fetch the official bodies spreadsheet/JSON; parse into the
      authorities table.
- [x] 4.2 `fyi import-authorities` reads upstream when no CSV is provided.
- [x] 4.3 Tests for parse + idempotent upsert.

## Phase 5: CLI surface + docs

- [x] 5.1 `fyi discover` + flags wired to the argparse parser (`cli.py`).
- [ ] 5.2 README/API_KEY_SETUP docs updated; ethics note pointing to fyi-archive's
      `docs/ethics-and-compliance.md`.
- [ ] 5.3 Opt-in `@smoke` live test (gated, single small window).
