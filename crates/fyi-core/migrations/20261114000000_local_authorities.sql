-- Local tracking has a different identity and permission boundary from remote sync.
-- Keep the established Python authority projection for explicit future migration.
CREATE TABLE IF NOT EXISTS authorities (
    slug TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    url TEXT
);
