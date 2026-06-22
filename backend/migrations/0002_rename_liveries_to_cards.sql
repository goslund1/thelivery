-- Rename the entity from "livery" to "card": the catalog is a generic
-- card-gallery, not Forza-specific. Schema is otherwise unchanged (id,
-- catalog_number, body JSON). Existing rows keep their data; the body JSON is
-- normalized to the new section-based shape at startup (see normalize_bodies()).
ALTER TABLE liveries RENAME TO cards;
