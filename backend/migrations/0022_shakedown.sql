-- Shakedown's store: the hands-on checker (~/Projects/Shakedown), hosted
-- at /shakedown/ and kept here, admin only. The agreement is Shakedown's
-- spec/hosted.md; src/shakedown.rs serves it. Nothing of The Livery's
-- reads or writes these tables.

-- Each install's checklist and map, as uploaded with an upload key. Every
-- upload is kept; the newest per app is the one shown.
CREATE TABLE shakedown_uploads (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    app         TEXT NOT NULL,
    commit_id   TEXT NOT NULL DEFAULT '',
    date        TEXT NOT NULL DEFAULT '',
    checklist   TEXT NOT NULL,
    map         TEXT NOT NULL DEFAULT '',
    key_id      INTEGER,
    uploaded_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX shakedown_uploads_app ON shakedown_uploads (app, id);

-- Upload keys: only a SHA-256 of each is kept (a random 256-bit key needs
-- no slow hash). Good for the upload route and nothing else.
CREATE TABLE shakedown_keys (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    label        TEXT NOT NULL,
    key_hash     TEXT NOT NULL UNIQUE,
    created_by   TEXT NOT NULL,
    created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    last_used_at TEXT,
    revoked_at   TEXT
);

-- A run: one pass over one app's checklist on one build. One open run per
-- app; starting another closes it. `send_json` is the last Send to Phone.
CREATE TABLE shakedown_runs (
    id             TEXT PRIMARY KEY,
    app            TEXT NOT NULL,
    build          TEXT NOT NULL,
    started        TEXT NOT NULL,
    checklist_name TEXT NOT NULL DEFAULT '',
    map_name       TEXT NOT NULL DEFAULT '',
    checklist_text TEXT NOT NULL,
    map_text       TEXT NOT NULL DEFAULT '',
    created_by     TEXT NOT NULL,
    created_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    closed_at      TEXT,
    send_json      TEXT
);
CREATE INDEX shakedown_runs_open ON shakedown_runs (app, closed_at);

-- Every change applied, in order: answers to rows and panes, sends, and
-- build renames. `seq` is the run's (and the stream's) count; `cid` is the
-- page's own id for a change, so one sent twice is applied once.
CREATE TABLE shakedown_changes (
    seq       INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id    TEXT NOT NULL REFERENCES shakedown_runs (id),
    cid       TEXT NOT NULL UNIQUE,
    kind      TEXT NOT NULL CHECK (kind IN ('row', 'pane', 'send', 'build')),
    target    TEXT NOT NULL DEFAULT '',
    fields    TEXT NOT NULL,
    device    TEXT NOT NULL DEFAULT '',
    username  TEXT NOT NULL,
    server_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX shakedown_changes_run ON shakedown_changes (run_id, seq);

-- The merged answer per row and pane: `state` is JSON, each field with its
-- value, when it was given (`at`, ms) and the change that set it (`seq`).
CREATE TABLE shakedown_answers (
    run_id      TEXT NOT NULL REFERENCES shakedown_runs (id),
    kind        TEXT NOT NULL CHECK (kind IN ('row', 'pane')),
    target      TEXT NOT NULL,
    state       TEXT NOT NULL,
    updated_seq INTEGER NOT NULL,
    server_at   TEXT NOT NULL,
    PRIMARY KEY (run_id, kind, target)
);
