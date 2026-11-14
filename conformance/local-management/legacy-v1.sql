-- Synthetic offline fixture; no real people, credentials, or live endpoints.
-- SCHEMA below is copied verbatim from src/fyi_system/db.py via ast.literal_eval.
-- Legacy Python has no application schema-version marker (PRAGMA user_version=0).

PRAGMA foreign_keys=ON;
CREATE TABLE IF NOT EXISTS authorities (
  slug TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  url TEXT
);
CREATE TABLE IF NOT EXISTS tracked_requests (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  authority_slug TEXT NOT NULL,
  title TEXT NOT NULL,
  body TEXT NOT NULL,
  tags TEXT DEFAULT '',
  fyi_url TEXT,
  fyi_request_id INTEGER,
  status TEXT DEFAULT 'draft',
  created_at TEXT DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
  last_seen_at TEXT,
  last_event_title TEXT,
  FOREIGN KEY(authority_slug) REFERENCES authorities(slug)
);
CREATE TABLE IF NOT EXISTS feed_events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  feed_url TEXT NOT NULL,
  event_id TEXT,
  title TEXT,
  link TEXT,
  published TEXT,
  summary TEXT,
  request_id_guess INTEGER,
  tracked_request_id INTEGER,
  raw_json TEXT,
  seen_at TEXT DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS request_snapshots (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  fyi_request_id INTEGER NOT NULL,
  source_url TEXT NOT NULL,
  raw_json TEXT NOT NULL,
  fetched_at TEXT DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS run_log (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  job_name TEXT NOT NULL,
  status TEXT NOT NULL,
  detail TEXT,
  ran_at TEXT DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS shared_rate_limit_state (
  name TEXT PRIMARY KEY,
  next_allowed_at REAL NOT NULL DEFAULT 0,
  last_acquired_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  last_owner_id TEXT NOT NULL DEFAULT '',
  last_sleep_seconds REAL NOT NULL DEFAULT 0,
  interval_seconds REAL NOT NULL DEFAULT 0,
  jitter_seconds REAL NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS shared_rate_limit_events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL,
  kind TEXT NOT NULL,
  owner_id TEXT NOT NULL DEFAULT '',
  status_code INTEGER,
  delay_seconds REAL NOT NULL DEFAULT 0,
  next_allowed_at REAL NOT NULL DEFAULT 0,
  observed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS http_cache (
  url TEXT PRIMARY KEY,
  etag TEXT,
  last_modified TEXT,
  response_body TEXT,
  cached_at TEXT DEFAULT CURRENT_TIMESTAMP
);

-- Deterministic seed rows; explicit IDs and timestamps prevent clock dependence.
BEGIN;
INSERT INTO authorities (slug, name, url) VALUES ('synthetic-maori', 'Synthetic Te Puna Māori — café', 'https://authority.example.invalid/body/synthetic-maori');
INSERT INTO authorities (slug, name, url) VALUES ('synthetic-local', 'Synthetic Local Authority', NULL);
INSERT INTO tracked_requests (id, authority_slug, title, body, tags, fyi_url, fyi_request_id, status, created_at, updated_at, last_seen_at, last_event_title) VALUES (7, 'synthetic-maori', 'Synthetic records request — Māori café', CAST(X'53594e5448455449432d505249564154452d424f44592d370d0a51756f74656420227265636f7264223b2061706f7374726f7068653a20697427732e0a556e69636f64653a204dc4816f72693b2063616665cc813b20f09fa7aa09656e642e0a' AS TEXT), 'synthetic,private', 'https://portal.example.invalid/request/9007', 9007, 'waiting_response', '2026-01-02 03:04:05', '2026-01-03T04:05:06+00:00', '2026-01-04 05:06:07', 'Synthetic receipt — reçu');
INSERT INTO tracked_requests (id, authority_slug, title, body, tags, fyi_url, fyi_request_id, status, created_at, updated_at, last_seen_at, last_event_title) VALUES (2147483650, 'synthetic-local', 'Synthetic local-only request 🧪', CAST(X'53594e5448455449432d505249564154452d424f44592d4c415247452d49440a4e6f2072656d6f7465207375626d697373696f6e2e0a' AS TEXT), NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL);
INSERT INTO feed_events (id, feed_url, event_id, title, link, published, summary, request_id_guess, tracked_request_id, raw_json, seen_at) VALUES (3, 'https://portal.example.invalid/feed', 'synthetic-event-a', 'Synthetic acknowledgement', 'https://portal.example.invalid/request/9007#event-3', '2026-01-02T10:00:00+10:00', NULL, 9007, 7, CAST(X'7b20226964223a202273796e7468657469632d6576656e742d61222c20227469746c65223a20224dc4816f7269222c202273796e7468657469635f70726976617465223a202253594e5448455449432d505249564154452d464545442d3322207d0a' AS TEXT), '2026-01-02 05:00:00');
INSERT INTO feed_events (id, feed_url, event_id, title, link, published, summary, request_id_guess, tracked_request_id, raw_json, seen_at) VALUES (11, 'https://portal.example.invalid/feed', NULL, NULL, NULL, NULL, CAST(X'53594e5448455449432d505249564154452d53554d4d4152592d31310a53796e746865746963207265636569707420e28094207265c3a7752e' AS TEXT), 9007, 7, NULL, '2026-01-04T05:06:07Z');
INSERT INTO feed_events (id, feed_url, event_id, title, link, published, summary, request_id_guess, tracked_request_id, raw_json, seen_at) VALUES (19, 'https://unrelated.example.invalid/feed', 'synthetic-unlinked', 'Synthetic unrelated feed row', NULL, NULL, '', 9007, NULL, '{"unlinked":true,"synthetic_private":"SYNTHETIC-PRIVATE-UNLINKED"}', '2026-01-05 00:00:00');
INSERT INTO request_snapshots (id, fyi_request_id, source_url, raw_json, fetched_at) VALUES (5, 9007, 'https://portal.example.invalid/request/9007.json', CAST(X'7b2022696e666f5f72657175657374223a207b20226964223a20393030372c20227469746c65223a202253796e74686574696320736e617073686f7420e28094204dc4816f7269222c20226465736372696265645f7374617465223a202277616974696e675f726573706f6e7365222c2022626f6479223a202253594e5448455449432d505249564154452d534e415053484f542d3522207d207d0a' AS TEXT), '2026-01-03 06:07:08');
INSERT INTO request_snapshots (id, fyi_request_id, source_url, raw_json, fetched_at) VALUES (9, 9007, 'https://portal.example.invalid/request/9007.json?version=2', '{"title":"Synthetic response snapshot","described_state":"successful","body":"SYNTHETIC-PRIVATE-SNAPSHOT-9"}', '2026-01-05T06:07:08.123456+00:00');
INSERT INTO request_snapshots (id, fyi_request_id, source_url, raw_json, fetched_at) VALUES (14, 7, 'https://unrelated.example.invalid/request/7.json', '{"title":"Synthetic remote ID 7 is not local ID 7","described_state":"rejected"}', '2026-01-06 00:00:00');
INSERT INTO run_log (id, job_name, status, detail, ran_at) VALUES (4, 'synthetic-offline-job', 'ok', CAST(X'7b202273796e746865746963223a20747275652c202270726976617465223a202253594e5448455449432d505249564154452d52554e2d3422207d0a' AS TEXT), '2026-01-06 07:08:09');
INSERT INTO run_log (id, job_name, status, detail, ran_at) VALUES (8, 'synthetic-null-detail', 'skipped', NULL, NULL);
INSERT INTO shared_rate_limit_state (name, next_allowed_at, last_acquired_at, last_owner_id, last_sleep_seconds, interval_seconds, jitter_seconds) VALUES ('synthetic:portal.example.invalid', 1767323045.125, '2026-01-02T03:04:05+00:00', 'synthetic-owner', 0.125, 2.5, 0.25);
INSERT INTO shared_rate_limit_events (id, name, kind, owner_id, status_code, delay_seconds, next_allowed_at, observed_at) VALUES (6, 'synthetic:portal.example.invalid', 'acquire', 'synthetic-owner', NULL, 0.125, 1767323045.125, '2026-01-02T03:04:05+00:00');
INSERT INTO shared_rate_limit_events (id, name, kind, owner_id, status_code, delay_seconds, next_allowed_at, observed_at) VALUES (12, 'synthetic:unrelated.example.invalid', 'backoff', '', 429, 3.75, 1767323050.5, '2026-01-02 03:04:06');
INSERT INTO http_cache (url, etag, last_modified, response_body, cached_at) VALUES ('https://portal.example.invalid/request/9007.json', '"synthetic-etag-7"', 'Fri, 02 Jan 2026 03:04:05 GMT', CAST(X'7b2022626f6479223a202253594e5448455449432d505249564154452d43414348452d37222c2022756e69636f6465223a20224dc4816f726922207d0d0a' AS TEXT), '2026-01-02 03:04:05');
INSERT INTO http_cache (url, etag, last_modified, response_body, cached_at) VALUES ('https://unrelated.example.invalid/empty', NULL, NULL, NULL, NULL);
COMMIT;
