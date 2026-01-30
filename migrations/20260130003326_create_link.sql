CREATE TABLE IF NOT EXISTS link (
    code TEXT PRIMARY KEY CHECK (code = trim(code) AND length(code) > 0 AND length(code) <= 32),
    url TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
