-- User roles: 'admin' (full power) vs 'editor' (can edit and soft-delete, but
-- never permanently delete anything). Existing users default to admin.
ALTER TABLE users ADD COLUMN role TEXT NOT NULL DEFAULT 'admin' CHECK (role IN ('admin', 'editor'));

-- Audit timeline: one row per authenticated mutation, so every editor action is
-- recordable and reversible. For destructive overwrites (livery/tune/theme
-- updates) `detail` holds the previous state as JSON; card edits are already
-- reversible via card_history, deletes via soft-delete/trash.
CREATE TABLE audit_log (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    username   TEXT NOT NULL,
    action     TEXT NOT NULL,              -- e.g. 'card.update', 'card.delete', 'image.upload'
    entity     TEXT NOT NULL,              -- 'card' | 'image' | 'livery' | 'tune' | 'preset' | 'theme'
    entity_id  TEXT,                       -- id of the affected row (as text)
    detail     TEXT,                       -- JSON: previous state / extra context, nullable
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_audit_log_created ON audit_log (created_at DESC);
