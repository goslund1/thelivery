-- Add curated code field to cars (unique per game, used in serials).
ALTER TABLE cars ADD COLUMN code TEXT;
CREATE UNIQUE INDEX IF NOT EXISTS idx_cars_game_code ON cars(game, code);

-- New: factory color options per car (seeded from scrape).
CREATE TABLE IF NOT EXISTS car_colors (
  id           INTEGER PRIMARY KEY AUTOINCREMENT,
  car_id       TEXT NOT NULL REFERENCES cars(id),
  name         TEXT NOT NULL,        -- "Bayside Blue", "Midnight Purple"
  hex_approx   TEXT,                 -- rough hex, e.g. '#1a3a6e'
  filter_color TEXT,                 -- taxonomy: 'Blue', 'Purple', etc.
  UNIQUE(car_id, name)
);

-- New: managed list of tune types.
CREATE TABLE IF NOT EXISTS tune_types (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  name       TEXT UNIQUE NOT NULL,
  sort_order INTEGER NOT NULL DEFAULT 0
);
INSERT OR IGNORE INTO tune_types (name, sort_order) VALUES
  ('Race',        1),
  ('Drift',       2),
  ('Rally',       3),
  ('Offroad',     4),
  ('Drag',        5),
  ('Stunt',       6),
  ('Gimmick',     7),
  ('Overpowered', 8);

-- New: liveries (paint + vinyl design on a specific car).
CREATE TABLE IF NOT EXISTS liveries (
  id              INTEGER PRIMARY KEY AUTOINCREMENT,
  car_id          TEXT NOT NULL REFERENCES cars(id),
  serial          TEXT UNIQUE NOT NULL,   -- e.g. 'FH6-NISRVGTSP99-L001'
  name            TEXT NOT NULL,          -- "JDM Dreams" / "Bayside Blue (Factory)"
  is_factory      INTEGER NOT NULL DEFAULT 0,
  car_color_id    INTEGER REFERENCES car_colors(id),
  share_code      TEXT,                   -- Forza cosmetic share code
  color_primary   TEXT,                   -- from color taxonomy
  color_secondary TEXT,
  created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_liveries_car_id ON liveries(car_id);

-- New: tunes (performance config linked to a livery).
CREATE TABLE IF NOT EXISTS tunes (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  livery_id     INTEGER NOT NULL REFERENCES liveries(id),
  car_id        TEXT NOT NULL REFERENCES cars(id),
  serial        TEXT UNIQUE NOT NULL,   -- e.g. 'FH6-NISRVGTSP99-L001-T001'
  official_name TEXT,                   -- name as shared on FH servers (immutable)
  type_id       INTEGER REFERENCES tune_types(id),
  share_code    TEXT,
  core_specs    TEXT,   -- JSON
  upgrades      TEXT,   -- JSON
  adjustments   TEXT,   -- JSON
  created_at    TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_tunes_livery_id ON tunes(livery_id);
CREATE INDEX IF NOT EXISTS idx_tunes_car_id    ON tunes(car_id);

-- Extend images with livery/tune refs and serial (nullable — lazy migration).
ALTER TABLE images ADD COLUMN livery_id INTEGER REFERENCES liveries(id);
ALTER TABLE images ADD COLUMN tune_id   INTEGER REFERENCES tunes(id);
ALTER TABLE images ADD COLUMN serial    TEXT;

-- Extend cards with livery/tune refs and serial (nullable — lazy migration).
-- Single-combo cards populate these; multi-combo cards use variants in body JSON.
ALTER TABLE cards ADD COLUMN livery_id INTEGER REFERENCES liveries(id);
ALTER TABLE cards ADD COLUMN tune_id   INTEGER REFERENCES tunes(id);
ALTER TABLE cards ADD COLUMN serial    TEXT;
