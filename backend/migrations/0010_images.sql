-- Stable DB records for uploaded images.
-- card_id and car_id match the JSON body; path is the /uploads/... URL.
-- sort_order mirrors the CardImage.order field in the frontend.
-- Rows are created on upload and lazily back-filled on first card save.
CREATE TABLE IF NOT EXISTS images (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  card_id     TEXT NOT NULL,
  path        TEXT NOT NULL,
  thumb_path  TEXT,
  stage_path  TEXT,
  car_id      TEXT,
  alt_text    TEXT,
  sort_order  INTEGER NOT NULL DEFAULT 0,
  created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_images_card_id ON images(card_id);
CREATE INDEX IF NOT EXISTS idx_images_car_id  ON images(car_id);
