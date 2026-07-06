-- Tracks every file moved to uploads/trash/ so restore is a one-click undo.
-- reason: 'orphan'      — scanner found no images table reference
--         'user_delete' — explicitly removed from a card via the × button
CREATE TABLE IF NOT EXISTS trash_log (
  id                   INTEGER PRIMARY KEY AUTOINCREMENT,
  trash_filename       TEXT NOT NULL,        -- UUID-prefixed basename in uploads/trash/
  trash_thumb_filename TEXT,                 -- thumb basename in trash/, if moved
  trash_stage_filename TEXT,                 -- stage basename in trash/, if moved
  original_path        TEXT NOT NULL,        -- e.g. /uploads/smokin_1/FH6_...jpg
  original_thumb_path  TEXT,
  original_stage_path  TEXT,
  card_id              TEXT,                 -- null for orphans; known for user_delete
  images_row_id        INTEGER,              -- the images PK that was deleted (for user_delete)
  reason               TEXT NOT NULL CHECK (reason IN ('orphan', 'user_delete')),
  trashed_at           TEXT NOT NULL DEFAULT (datetime('now'))
);
