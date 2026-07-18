-- Forced password change: set when an admin creates a user with a temporary
-- password (Add User UI); cleared when the user changes their password. While
-- set, the auth extractor rejects every authenticated request except
-- PUT /api/me/password.
ALTER TABLE users ADD COLUMN must_change_password INTEGER NOT NULL DEFAULT 0;
