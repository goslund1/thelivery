-- Single-row site theme configuration.
-- The CHECK (id = 1) constraint enforces exactly one row.
CREATE TABLE IF NOT EXISTS theme (
    id   INTEGER PRIMARY KEY CHECK (id = 1),
    body TEXT NOT NULL
);
