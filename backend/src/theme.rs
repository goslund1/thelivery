//! Site theme: default palette, seeding, and the get/put handlers.

use axum::{extract::State, http::StatusCode, Json};
use serde_json::Value;
use sqlx::SqlitePool;

use crate::auth::AuthUser;
use crate::state::{err, ApiError, AppState};

// --- Theme ------------------------------------------------------------------

pub const THEME_DEFAULT: &str = r##"{
  "colors": {
    "base":       "#0b0b0d",
    "panel":      "#15151a",
    "panelEdge":  "#23232b",
    "accent":     "#c9a227",
    "highlight":  "#d6478f",
    "fg":         "#ece9e4",
    "muted":      "#7a7e87",
    "panelWell":  "#0e0e11",
    "mutedLight": "#a8a4ab"
  },
  "tuning": {
    "tires":        "#29C5F6",
    "gearing":      "#1FD1A5",
    "alignment":    "#E63DD0",
    "arb":          "#8A2BE2",
    "springs":      "#F4831F",
    "damping":      "#E8650F",
    "aero":         "#5BDB2E",
    "brakes":       "#FF3B2F",
    "differential": "#1E6FE0"
  },
  "fonts": {
    "mono":    "JetBrains Mono",
    "display": "Archivo Black"
  },
  "ambiance": "dark",
  "effects": {
    "glassOpacity": 82
  }
}"##;

pub async fn seed_theme_if_empty(pool: &SqlitePool) -> anyhow::Result<()> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM theme")
        .fetch_one(pool)
        .await?;
    if count == 0 {
        sqlx::query("INSERT INTO theme (id, body) VALUES (1, ?)")
            .bind(THEME_DEFAULT)
            .execute(pool)
            .await?;
        tracing::info!("seeded default theme");
    }
    Ok(())
}

pub async fn get_theme(State(st): State<AppState>) -> Result<Json<Value>, ApiError> {
    let body: String = sqlx::query_scalar("SELECT body FROM theme WHERE id = 1")
        .fetch_optional(&st.pool)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?
        .unwrap_or_else(|| THEME_DEFAULT.to_string());
    let v: Value = serde_json::from_str(&body)
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(v))
}

pub async fn put_theme(
    auth: AuthUser,
    State(st): State<AppState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    // Snapshot for the audit trail — `detail.prev` reverses this overwrite.
    let prev: Option<String> = sqlx::query_scalar("SELECT body FROM theme WHERE id = 1")
        .fetch_optional(&st.pool)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let body_str = body.to_string();
    sqlx::query(
        "INSERT INTO theme (id, body) VALUES (1, ?)
         ON CONFLICT(id) DO UPDATE SET body = excluded.body"
    )
    .bind(&body_str)
    .execute(&st.pool)
    .await
    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let prev_json = prev.and_then(|p| serde_json::from_str::<Value>(&p).ok());
    crate::audit::record(
        &st.pool, &auth.username, "theme.update", "theme", None,
        prev_json.map(|p| serde_json::json!({ "prev": p })),
    ).await;
    Ok(Json(body))
}
