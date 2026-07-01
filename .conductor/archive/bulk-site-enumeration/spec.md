# Track: bulk-site-enumeration

## Objective

Give `fyi-cli` the ability to **enumerate the entire public corpus** of fyi.org.nz,
so it can drive a full historical seed of `fyi-archive`. Today `fyi-cli` can fetch a
single request by ID and read feeds, but has no way to list every request on the site.

## Background

fyi.org.nz runs Alaveteli. Verified facts (2026-06-27):

- Reads need **no auth**.
- Append `.json` to most URLs for a structured version.
- `/request/{id}.json` **302-redirects** to `/request/{url_title}.json` (must follow
  redirects).
- There is **no `list.json`** bulk endpoint.
- Enumeration is via **advanced-search Atom feeds** + their `.json` equivalents
  (`/search/...?...&output=json`-style), which support filtering by date range,
  status, and authority, and are **paginated**.
- An **official spreadsheet listing every public body** is also published; that seeds
  the authorities index.

## Scope

- A polite async HTTP client (`httpx` + `httpx-limiter` + `tenacity`): descriptive
  contactable `User-Agent`, `robots.txt` cache + respect, ~1 req/s token bucket with
  jitter, exponential backoff on 429/5xx. (Shared with the capture track.)
- `fyi discover` command:
  - `--date-from` / `--date-to` (windowed enumeration of request creation dates).
  - `--authority` (optional filter).
  - `--status` (optional filter).
  - `--paginate` over the search feed's pages until exhausted within the window.
  - Emits a stream/JSONL of discovered `{request_id, url_title, ...}` records.
- `fyi discover --backfill-ids` (optional gap-fill): probe a sequential ID range
  following the `url_title` redirect, skipping 404s, to catch any requests the
  feed-based walk missed (deleted/hidden-but-resolvable).
- `fyi import-authorities` enhancement: fetch the official bodies spreadsheet (or its
  JSON form) into the authorities table, instead of CSV-only.

## Out of scope

- Capturing request content/attachments (that's `faithful-archive-capture`).
- Diffing (that's `archival-content-diff`).

## Acceptance criteria

- [ ] `fyi discover --date-from 2024-01-01 --date-to 2024-02-01` returns a complete,
      deduplicated, paginated list of requests created in that window, resilient to a
      mid-run interrupt.
- [ ] The polite HTTP client respects `robots.txt` and never exceeds the configured
      rate; backoff recovers from an injected 429.
- [ ] `--backfill-ids` reconciles to the same set as the feed walk on a sampled window
      (documented agreement + documented divergences).
- [ ] `fyi import-authorities` can populate the authorities table from the upstream
      source without a local CSV.
- [ ] Unit + hypothesis tests; respx-mocked HTTP; no live network in CI beyond an
      opt-in `@smoke` test.

## Risks

- Alaveteli feed pagination conventions vary by deployment → tolerant parser, raw
  entry retention for forensics.
- Aggressive crawling impact → hard caps + slow default rate; documented runbook.
