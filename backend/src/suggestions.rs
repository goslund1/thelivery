//! Public tune suggestions: rate limiting, title moderation, submission,
//! and the admin review handlers.

use axum::{
    extract::{ConnectInfo, Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;

use crate::auth::AdminUser;
use crate::state::{err, ApiError, AppState};

pub const SUGGESTION_HOURLY_LIMIT: i64 = 15;   // hard cap per hour
pub const SUGGESTION_BURST_WARN: i64 = 3;      // warn after this many in 5 min
pub const SUGGESTION_BURST_BLOCK: i64 = 4;     // block after this many in 5 min
pub const SUGGESTION_TITLE_MAX: usize = 60;

// Patterns that indicate directed hostility or hate — not general profanity.
pub const BLOCKED_PATTERNS: &[&str] = &[
    "fuck you", "kill yourself", "kys", "go die", "you suck",
    "faggot", "nigger", "nigga", "chink", "spic", "wetback",
    "retard", "cunt", "whore", "bitch ass",
];

pub fn is_title_blocked(title: &str) -> bool {
    let lower = title.to_lowercase();
    BLOCKED_PATTERNS.iter().any(|p| lower.contains(p))
}

// Anonymize an IP for storage: masks the host portion so individuals can't be
// identified while still being useful for rate-limiting by subnet.
// IPv4: last octet zeroed (e.g. 1.2.3.4 → 1.2.3.0)
// IPv6: last 80 bits zeroed (keeps /48 prefix)
pub fn anonymize_ip(ip: &str) -> String {
    match ip.parse::<std::net::IpAddr>() {
        Ok(std::net::IpAddr::V4(v4)) => {
            let [a, b, c, _] = v4.octets();
            format!("{a}.{b}.{c}.0")
        }
        Ok(std::net::IpAddr::V6(v6)) => {
            let s = v6.segments();
            format!("{:x}:{:x}:{:x}::", s[0], s[1], s[2])
        }
        Err(_) => "unknown".to_string(),
    }
}

/// Resolve the real client IP for rate limiting. In production the backend
/// binds to 127.0.0.1 behind Caddy, so ConnectInfo is always localhost and
/// X-Forwarded-For (set by Caddy) carries the real address — without this,
/// every visitor shares one bucket and 5 bad attempts from anyone would lock
/// out everyone. The header is trustworthy here precisely because nothing but
/// the local proxy can reach the port; the first entry is the original client.
pub fn client_ip(headers: &HeaderMap, addr: &std::net::SocketAddr) -> String {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| addr.ip().to_string())
}

// --- Suggestions ------------------------------------------------------------

#[derive(Deserialize)]
pub struct SubmitSuggestionReq {
    card_id: String,
    title: String,
    credit: Option<String>,
    adjustments: Value,
}

pub async fn submit_suggestion(
    State(st): State<AppState>,
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<SubmitSuggestionReq>,
) -> Result<Json<Value>, ApiError> {
    let ip_hash = anonymize_ip(&client_ip(&headers, &addr));

    // DB-based rate limiting — persists across restarts.
    // Two tiers: hourly hard cap, and a burst cap with a warning before blocking.
    let count_hour: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM suggestion_rate_log WHERE ip_hash = ? AND submitted_at > datetime('now', '-1 hour')"
    )
    .bind(&ip_hash)
    .fetch_one(&st.pool)
    .await
    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    if count_hour >= SUGGESTION_HOURLY_LIMIT {
        return Err(err(StatusCode::TOO_MANY_REQUESTS, "Hourly suggestion limit reached — try again later"));
    }

    let count_burst: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM suggestion_rate_log WHERE ip_hash = ? AND submitted_at > datetime('now', '-5 minutes')"
    )
    .bind(&ip_hash)
    .fetch_one(&st.pool)
    .await
    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    if count_burst >= SUGGESTION_BURST_BLOCK {
        return Err(err(StatusCode::TOO_MANY_REQUESTS, "Slow down — too many suggestions in a short period"));
    }

    // Validate title
    let title = req.title.trim().to_string();
    if title.is_empty() || title.chars().count() > SUGGESTION_TITLE_MAX {
        return Err(err(StatusCode::UNPROCESSABLE_ENTITY, format!("Title must be 1–{SUGGESTION_TITLE_MAX} characters")));
    }
    if is_title_blocked(&title) {
        return Err(err(StatusCode::UNPROCESSABLE_ENTITY, "Title contains prohibited content"));
    }

    let credit = req.credit.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    let adjustments = req.adjustments.to_string();
    if adjustments.len() > 65_536 {
        return Err(err(StatusCode::UNPROCESSABLE_ENTITY, "adjustments payload too large"));
    }

    sqlx::query(
        "INSERT INTO suggestions (card_id, title, credit, adjustments, ip) VALUES (?, ?, ?, ?, ?)"
    )
    .bind(&req.card_id)
    .bind(&title)
    .bind(&credit)
    .bind(&adjustments)
    .bind(&ip_hash)
    .execute(&st.pool)
    .await
    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    sqlx::query("INSERT INTO suggestion_rate_log (ip_hash) VALUES (?)")
        .bind(&ip_hash)
        .execute(&st.pool)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    // Warn if they've just hit the burst threshold (this submission was accepted).
    let slowdown = count_burst + 1 >= SUGGESTION_BURST_WARN;
    Ok(Json(json!({ "ok": true, "slowdown": slowdown })))
}

pub async fn admin_list_suggestions(
    State(st): State<AppState>,
    _admin: AdminUser,
) -> Result<Json<Value>, ApiError> {
    let rows = sqlx::query(
        "SELECT id, card_id, title, credit, adjustments, submitted_at, ip, status FROM suggestions ORDER BY submitted_at DESC"
    )
    .fetch_all(&st.pool)
    .await
    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let list: Vec<Value> = rows.iter().map(|r| json!({
        "id":           r.get::<i64, _>("id"),
        "cardId":       r.get::<String, _>("card_id"),
        "title":        r.get::<String, _>("title"),
        "credit":       r.get::<Option<String>, _>("credit"),
        "adjustments":  serde_json::from_str::<Value>(&r.get::<String, _>("adjustments")).unwrap_or(Value::Null),
        "submittedAt":  r.get::<String, _>("submitted_at"),
        "ip":           r.get::<String, _>("ip"),
        "status":       r.get::<String, _>("status"),
    })).collect();

    Ok(Json(json!(list)))
}

pub async fn admin_dismiss_suggestion(
    State(st): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    sqlx::query("DELETE FROM suggestions WHERE id = ?")
        .bind(id)
        .execute(&st.pool)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(json!({ "ok": true })))
}

pub async fn admin_like_suggestion(
    State(st): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    sqlx::query("UPDATE suggestions SET status = CASE WHEN status = 'liked' THEN 'pending' ELSE 'liked' END WHERE id = ?")
        .bind(id)
        .execute(&st.pool)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(json!({ "ok": true })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_ip_prefers_forwarded_header() {
        let addr: std::net::SocketAddr = "127.0.0.1:9999".parse().unwrap();
        let mut headers = HeaderMap::new();
        assert_eq!(client_ip(&headers, &addr), "127.0.0.1");

        headers.insert("x-forwarded-for", "203.0.113.7".parse().unwrap());
        assert_eq!(client_ip(&headers, &addr), "203.0.113.7");

        // Multi-hop: first entry is the original client.
        headers.insert("x-forwarded-for", "203.0.113.7, 10.0.0.1".parse().unwrap());
        assert_eq!(client_ip(&headers, &addr), "203.0.113.7");

        // Empty header falls back to the socket address.
        headers.insert("x-forwarded-for", "".parse().unwrap());
        assert_eq!(client_ip(&headers, &addr), "127.0.0.1");
    }
}
