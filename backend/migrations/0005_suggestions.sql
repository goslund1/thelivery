CREATE TABLE IF NOT EXISTS suggestions (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  card_id     TEXT    NOT NULL,
  title       TEXT    NOT NULL,
  credit      TEXT,
  adjustments TEXT    NOT NULL,
  submitted_at TEXT   NOT NULL DEFAULT (datetime('now')),
  ip          TEXT    NOT NULL,
  reviewed    INTEGER NOT NULL DEFAULT 0
);
