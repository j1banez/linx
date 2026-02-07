CREATE TABLE link_new (
    code TEXT PRIMARY KEY
        CHECK (
            length(code) > 0
            AND length(code) <= 32
            AND code NOT GLOB '*[^0-9A-Za-z]*'
        ),
    url TEXT NOT NULL,
    clicks INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    last_accessed_at INTEGER
);

INSERT INTO link_new (code, url, clicks, created_at, last_accessed_at)
SELECT code, url, clicks, created_at, last_accessed_at
FROM link;

ALTER TABLE link RENAME TO link_old;
ALTER TABLE link_new RENAME TO link;

DROP TABLE link_old;
