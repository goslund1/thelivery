-- Users for authentication. Passwords are stored only as Argon2 hashes (never
-- plaintext). Users are created via the `livery-backend adduser <name>` CLI;
-- there is no public registration.
CREATE TABLE users (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    username      TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    created_at    TEXT NOT NULL DEFAULT (datetime('now'))
);
