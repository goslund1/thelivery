//! Audit timeline: records every authenticated mutation so editor actions are
//! reviewable and reversible. Reversal paths: card edits → card_history,
//! card/image deletes → soft-delete/trash restore, overwrites (livery/tune/
//! theme/og-preset updates) → previous state stored in `detail`.

use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::{Row, SqlitePool};

use crate::auth::AdminUser;
use crate::state::{err, ApiError, AppState};

/// Best-effort insert — an audit failure must never fail the user's request.
pub async fn record(
    pool: &SqlitePool,
    username: &str,
    action: &str,
    entity: &str,
    entity_id: Option<&str>,
    detail: Option<Value>,
) {
    let res = sqlx::query(
        "INSERT INTO audit_log (username, action, entity, entity_id, detail) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(username)
    .bind(action)
    .bind(entity)
    .bind(entity_id)
    .bind(detail.map(|d| d.to_string()))
    .execute(pool)
    .await;
    if let Err(e) = res {
        tracing::error!("audit_log insert failed ({action} by {username}): {e}");
    }
}

#[derive(Deserialize)]
pub struct AuditQuery {
    limit: Option<i64>,
    before_id: Option<i64>,
}

/// Admin-only timeline, newest first. `before_id` pages backwards.
pub async fn admin_list_audit(
    _admin: AdminUser,
    State(st): State<AppState>,
    Query(q): Query<AuditQuery>,
) -> Result<Json<Value>, ApiError> {
    let limit = q.limit.unwrap_or(100).clamp(1, 500);
    let before = q.before_id.unwrap_or(i64::MAX);
    let rows = sqlx::query(
        "SELECT id, username, action, entity, entity_id, detail, created_at
         FROM audit_log WHERE id < ? ORDER BY id DESC LIMIT ?",
    )
    .bind(before)
    .bind(limit)
    .fetch_all(&st.pool)
    .await
    .map_err(|e| err(axum::http::StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let entries: Vec<Value> = rows
        .iter()
        .map(|r| {
            let detail = r
                .get::<Option<String>, _>("detail")
                .and_then(|d| serde_json::from_str::<Value>(&d).ok());
            json!({
                "id": r.get::<i64, _>("id"),
                "username": r.get::<String, _>("username"),
                "action": r.get::<String, _>("action"),
                "entity": r.get::<String, _>("entity"),
                "entityId": r.get::<Option<String>, _>("entity_id"),
                "detail": detail,
                "createdAt": r.get::<String, _>("created_at"),
            })
        })
        .collect();
    Ok(Json(json!(entries)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn record_inserts_row_with_detail() {
        let pool = crate::testutil::test_pool().await;
        record(&pool, "geoff", "card.update", "card", Some("3"), Some(json!({"version": 7}))).await;
        let row = sqlx::query("SELECT username, action, entity, entity_id, detail FROM audit_log")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(row.get::<String, _>("username"), "geoff");
        assert_eq!(row.get::<String, _>("action"), "card.update");
        assert_eq!(row.get::<String, _>("entity_id"), "3");
        let detail: Value = serde_json::from_str(&row.get::<String, _>("detail")).unwrap();
        assert_eq!(detail["version"], 7);
    }
}
