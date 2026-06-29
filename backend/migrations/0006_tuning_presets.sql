CREATE TABLE IF NOT EXISTS tuning_presets (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  name       TEXT    NOT NULL,
  body       TEXT    NOT NULL,  -- JSON: { [key: string]: number }
  created_at TEXT    NOT NULL DEFAULT (datetime('now'))
);
