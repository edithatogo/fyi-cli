-- Enable foreign key support
PRAGMA foreign_keys = ON;

-- Create requests table
CREATE TABLE IF NOT EXISTS requests (
    id INTEGER PRIMARY KEY,
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    user_name TEXT,
    status TEXT,
    created_at TEXT,
    updated_at TEXT,
    url TEXT,
    tags TEXT
);

-- Create correspondence table
CREATE TABLE IF NOT EXISTS correspondence (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    request_id INTEGER NOT NULL,
    direction TEXT NOT NULL,
    body TEXT NOT NULL,
    sent_at TEXT NOT NULL,
    state TEXT,
    attachments TEXT,
    FOREIGN KEY(request_id) REFERENCES requests(id) ON DELETE CASCADE
);
