-- Preserve the legacy local evidence tables without introducing acquisition.
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
