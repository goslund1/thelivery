-- Car registry: one row per car per game (FH5/FH6 are separate entries).
-- car_id on cards is nullable — existing cards remain valid; backfill manually.
CREATE TABLE IF NOT EXISTS cars (
    id       TEXT PRIMARY KEY,       -- e.g. fh6-nissan-skyline-gtr-r34
    game     TEXT NOT NULL,          -- FH5 | FH6
    make     TEXT NOT NULL,
    model    TEXT NOT NULL,
    year     INTEGER,
    class    TEXT,                   -- D/C/B/A/S1/S2/X
    pi       INTEGER,                -- stock PI (FH5 only)
    drive    TEXT,                   -- FWD/RWD/AWD (FH6 only)
    country  TEXT,
    category TEXT,                   -- e.g. Modern Sports (FH6 only)
    decade   TEXT,
    status   TEXT,
    dlc      TEXT                    -- null = base game; pack name = paid DLC
);

ALTER TABLE cards ADD COLUMN car_id TEXT REFERENCES cars(id);
