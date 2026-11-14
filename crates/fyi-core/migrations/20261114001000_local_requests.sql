-- Local registration is independent of the remote API request and sync tables.
-- Retain the Python field order, nullability, defaults and local identifier space.
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

-- Legacy adoption does not synthesize events for unknown historical transitions.
-- Each native transition is inserted in the same transaction as its request row.
CREATE TABLE IF NOT EXISTS local_request_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tracked_request_id INTEGER NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('created', 'status_changed')),
    old_status TEXT,
    new_status TEXT NOT NULL,
    occurred_at TEXT NOT NULL,
    FOREIGN KEY(tracked_request_id) REFERENCES tracked_requests(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_local_request_events_request
    ON local_request_events(tracked_request_id, id);
