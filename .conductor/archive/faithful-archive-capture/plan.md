# Plan: faithful-archive-capture

## Phase 1: Per-resource fetch

- [x] 1.1 Reuse/factor `fetch_request_page` to also return the resolved `url_title`.
- [x] 1.2 Fetch rendered HTML (`/request/{url_title}`) via the polite client.
- [x] 1.3 Enumerate + download attachment binaries (extend
      `extract_request_artifacts`); record content_type/size/sha256.
- [x] 1.4 Tests (respx): redirect-follow, attachment discovery variants, 404 skip.

## Phase 2: WARC writer

- [x] 2.1 `warcio` writer: one WARC 1.1 record per resource with payload sha256
      digest, correct headers, record ids.
- [x] 2.2 Append-only run WARC under `data/warc/<runid>.warc.gz`.
- [x] 2.3 Tests: round-trip read with `warcio`, digest correctness, header validity.

## Phase 3: WACZ packaging

- [x] 3.1 `py-wacz` packaging → `dist/site_snapshots/<YYYYMMDD>.wacz` with
      `datapackage.json` + index.
- [x] 3.2 Appendable/multi-segment so snapshots compose.
- [x] 3.3 Tests: WACZ opens in `warcio`/`pywb` replay; index correct.

## Phase 4: Content-addressed attachments + derived store

- [x] 4.1 Store attachments once by sha256; reference by hash from request records.
- [x] 4.2 Derived `data/raw/requests/<authority>/<id>/` view regenerated from WARC.
- [x] 4.3 Reconciliation test: derived view == WARC contents.

## Phase 5: Caps + CLI + docs

- [x] 5.1 `fyi capture` command + `--max-bytes/--max-runtime-minutes/--max-disk-gb`.
- [x] 5.2 Cap-enforcement + clean-abort tests.
- [x] 5.3 Docs: WARC/WACZ layout, replay instructions, ethics cross-link.
