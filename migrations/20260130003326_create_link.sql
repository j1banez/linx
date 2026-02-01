CREATE TABLE IF NOT EXISTS link (
    code TEXT PRIMARY KEY
        CHECK (
            length(code) > 0
            AND length(code) <= 32
            AND code GLOB '[0-9A-Za-z]*'
        ),
    url TEXT NOT NULL,
    clicks INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    last_accessed_at INTEGER
);
