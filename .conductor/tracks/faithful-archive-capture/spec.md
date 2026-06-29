# Track: faithful-archive-capture

## Objective

For each request discovered by `bulk-site-enumeration`, capture a **faithful,
archival-grade** snapshot and write it as **WARC 1.1** records packaged into **WACZ**,
with a **content-addressed attachment store** for cross-request dedup. WARC/WACZ is
the source of truth; the existing SQLite `request_snapshots` table and any derived
JSON/Parquet are views over it.

## Background

Today `fyi_system.fetch.fetch_request_page` does a single `requests.get` for the
request `.json` and stores raw JSON in SQLite. It does **not**:

- capture the rendered HTML page,
- download attachment binaries (it only lists attachment URLs),
- write WARC/WACZ,
- dedupe attachments across requests.

This track adds the missing faithful-capture layer while reusing the existing fetch
primitive and snapshot schema.

## Scope

- `fyi capture <request_id|url_title>` command:
  1. Fetch `/request/{id}.json` (follow redirect to `url_title`) — reuse
     `fetch_request_page` semantics.
  2. Fetch the rendered HTML page (`/request/{url_title}`).
  3. Enumerate attachment URLs from the JSON (existing `extract_request_artifacts`
     heuristics) and **download each binary** via the polite HTTP client.
- **WARC 1.1** writer (`warcio`): one record per resource (json, html, each
  attachment) with correct `WARC-Date`, `Content-Type`, `WARC-Payload-Digest`
  (sha256), `WARC-Target-URI`, and `WARC-Record-ID`.
- **WACZ** packaging (`py-wacz`): package the run's WARCs + a `datapackage.json` +
  WACZ index into `dist/site_snapshots/<YYYYMMDD>.wacz`. Multi-segment/appendable so
  annual snapshots compose without re-packing.
- **Content-addressed attachments**: store each attachment once under
  `data/attachments/<sha8>/<sha256>` (or content-addressed inside the WARC payload
  store); the request record references it by sha256.
- **Derived store** (for easy `load_dataset`/DuckDB consumption): write
  `data/raw/requests/<authority>/<id>/{request.json, page.html, snapshot_meta.json}`
  and an attachments index — derived from the WARC so the two never drift.
- Hard caps: `--max-bytes`, `--max-runtime-minutes`, `--max-disk-gb`; abort cleanly.

## Out of scope

- Bulk enumeration (that's `bulk-site-enumeration`).
- Diff/change-detection (that's `archival-content-diff`).
- OCR / text extraction / normalisation (phase 1 non-goal).

## Acceptance criteria

- [ ] `fyi capture <id>` produces a WARC containing the json + html + every
      attachment, each with a sha256 payload digest; packaged into a valid WACZ
      replayable in `pywb`/ReplayWeb.page.
- [ ] The same attachment attached to two requests is stored once
      (content-addressed dedup verified by hash).
- [ ] The derived `data/raw/requests/<id>/` view reconciles 1:1 with the WARC.
- [ ] Caps abort cleanly and flush partial state.
- [ ] Tests: WARC record validity (via `warcio` reader), WACZ package validity, dedup,
      derived-store reconciliation; respx-mocked HTTP.

## Risks

- Large attachments / total volume → caps + WACZ segmentation.
- Alaveteli attachment URL forms vary → tolerant extractor (extend existing
  `extract_request_artifacts`); record raw entries for forensics.
