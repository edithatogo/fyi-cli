ALTER TABLE sync_metadata
    ADD COLUMN remote_request_id INTEGER;

CREATE TABLE IF NOT EXISTS sync_outgoing_queue (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    request_id INTEGER NOT NULL,
    operation TEXT NOT NULL DEFAULT 'upsert_request'
        CHECK (operation IN ('upsert_request')),
    payload TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'submitted', 'failed')),
    remote_request_id INTEGER,
    attempts INTEGER NOT NULL DEFAULT 0,
    last_error TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    FOREIGN KEY(request_id) REFERENCES requests(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_sync_outgoing_queue_status
    ON sync_outgoing_queue(status, request_id);
