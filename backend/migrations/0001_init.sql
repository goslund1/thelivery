-- Single-user livery catalog. One row per livery; the full Livery object is
-- stored as JSON in `body`, with id + catalog_number lifted out for keying and
-- ordering. New schema changes must be added as new migration files, never by
-- editing this one.
CREATE TABLE liveries (
    id             TEXT    PRIMARY KEY,
    catalog_number INTEGER NOT NULL,
    body           TEXT    NOT NULL  -- full Livery JSON
);
