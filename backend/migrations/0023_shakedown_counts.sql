-- Shakedown's stage 3 (B-7): a test build's counts beside its run, totals
-- only (the counts file); the raw record never leaves the Mac (D-H6).
-- Additive: a new table, nothing existing changes.
CREATE TABLE shakedown_counts (
    seq        INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id     TEXT NOT NULL REFERENCES shakedown_runs(id),
    text       TEXT NOT NULL,
    device     TEXT NOT NULL DEFAULT '',
    username   TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX shakedown_counts_run ON shakedown_counts (run_id, seq);
