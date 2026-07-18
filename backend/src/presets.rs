//! Tuning presets: saved adjustment/upgrade bundles (build + baseline kinds).

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;

use crate::auth::{AdminUser, AuthUser};
use crate::state::{err, ApiError, AppState};

// --- Tuning Presets ---------------------------------------------------------

#[derive(Deserialize)]
pub struct CreatePresetReq {
    name: String,
    values: Value,
    #[serde(default = "default_preset_kind")]
    kind: String,
    #[serde(default)]
    upgrades: Option<Value>,
    #[serde(default)]
    baseline_id: Option<i64>,
}

pub fn default_preset_kind() -> String { "build".to_string() }

pub async fn list_tuning_presets(
    State(st): State<AppState>,
) -> Result<Json<Value>, ApiError> {
    let rows = sqlx::query("SELECT id, name, body, kind, upgrades, baseline_id, created_at FROM tuning_presets ORDER BY created_at ASC")
        .fetch_all(&st.pool)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let list: Vec<Value> = rows.iter().map(|r| json!({
        "id":         r.get::<i64, _>("id"),
        "name":       r.get::<String, _>("name"),
        "values":     serde_json::from_str::<Value>(&r.get::<String, _>("body")).unwrap_or(json!({})),
        "kind":       r.get::<String, _>("kind"),
        "upgrades":   r.get::<Option<String>, _>("upgrades")
                        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
                        .unwrap_or(json!([])),
        "baselineId": r.get::<Option<i64>, _>("baseline_id"),
        "createdAt":  r.get::<String, _>("created_at"),
    })).collect();

    Ok(Json(json!(list)))
}

pub async fn create_tuning_preset(
    State(st): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreatePresetReq>,
) -> Result<Json<Value>, ApiError> {
    let name = req.name.trim().to_string();
    if name.is_empty() {
        return Err(err(StatusCode::UNPROCESSABLE_ENTITY, "Name is required"));
    }
    let body = req.values.to_string();
    let kind = if req.kind == "baseline" { "baseline" } else { "build" };
    let upgrades_json = req.upgrades.as_ref().map(|u| u.to_string());
    let baseline_id = req.baseline_id;

    let result = sqlx::query(
        "INSERT INTO tuning_presets (name, body, kind, upgrades, baseline_id) VALUES (?, ?, ?, ?, ?)"
    )
        .bind(&name)
        .bind(&body)
        .bind(kind)
        .bind(&upgrades_json)
        .bind(baseline_id)
        .execute(&st.pool)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let id = result.last_insert_rowid();
    let created_at: String = sqlx::query_scalar("SELECT created_at FROM tuning_presets WHERE id = ?")
        .bind(id)
        .fetch_one(&st.pool)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let upgrades_out = req.upgrades.unwrap_or(json!([]));
    crate::audit::record(&st.pool, &auth.username, "preset.create", "preset", Some(&id.to_string()), None).await;
    Ok(Json(json!({
        "id": id, "name": name, "values": req.values,
        "kind": kind, "upgrades": upgrades_out, "baselineId": baseline_id,
        "createdAt": created_at
    })))
}

pub async fn delete_tuning_preset(
    State(st): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    sqlx::query("DELETE FROM tuning_presets WHERE id = ?")
        .bind(id)
        .execute(&st.pool)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(json!({ "ok": true })))
}
