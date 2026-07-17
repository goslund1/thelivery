-- Persistent rate limiting for failed login attempts, mirroring
-- suggestion_rate_log. Stores anonymized IP hashes only — no raw IPs.
CREATE TABLE IF NOT EXISTS login_rate_log (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    ip_hash      TEXT    NOT NULL,
    attempted_at TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_login_rate_log_ip_time ON login_rate_log (ip_hash, attempted_at);
