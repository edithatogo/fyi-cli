CREATE TABLE IF NOT EXISTS sync_metadata (
    request_id INTEGER PRIMARY KEY,
    last_synced_at TEXT,
    remote_updated_at TEXT,
    local_updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    sync_status TEXT NOT NULL DEFAULT 'dirty'
        CHECK (sync_status IN ('clean', 'dirty', 'pending', 'conflict')),
    conflict_version INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY(request_id) REFERENCES requests(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_sync_metadata_status
    ON sync_metadata(sync_status);

CREATE TABLE IF NOT EXISTS sync_field_changes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    request_id INTEGER NOT NULL,
    field_name TEXT NOT NULL,
    old_value TEXT,
    new_value TEXT,
    changed_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    synced_at TEXT,
    FOREIGN KEY(request_id) REFERENCES requests(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_sync_field_changes_request
    ON sync_field_changes(request_id, synced_at);
