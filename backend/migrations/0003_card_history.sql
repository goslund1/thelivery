CREATE TABLE IF NOT EXISTS card_history (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    card_id     TEXT    NOT NULL,
    version     INTEGER NOT NULL,
    body        TEXT    NOT NULL,
    saved_at    TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_card_history_lookup
    ON card_history (card_id, version DESC);
