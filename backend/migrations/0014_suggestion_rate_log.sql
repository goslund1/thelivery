-- Persistent rate limiting for suggestion submissions.
-- Replaces the in-memory HashMap (which reset on every server restart).
-- Stores anonymized IP hashes only — no raw IPs.
CREATE TABLE IF NOT EXISTS suggestion_rate_log (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    ip_hash      TEXT    NOT NULL,
    submitted_at TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_rate_log_ip_time ON suggestion_rate_log (ip_hash, submitted_at);
