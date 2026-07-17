//! Social sharing: the OG share page, compositor-backed card PNG, live
//! preview, and OG overlay presets.

use axum::{
    body::Body,
    extract::{Host, Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;

use crate::auth::AuthUser;
use crate::compositor;
use crate::images::{image_path_for_id, inject_images};
use crate::state::{err, ApiError, AppState};

// --- Share page -------------------------------------------------------------

/// Returns a minimal HTML page with Open Graph meta tags for link unfurls.
/// URL shape: /share/{card-id}/{ignored-slug}
/// The slug is for human readability and SEO — the card is looked up by id only.
/// Real browsers are redirected to the app via <meta http-equiv="refresh">.
pub async fn share_card(
    State(st): State<AppState>,
    Host(host): Host,
    Path(id): Path<String>,
    axum::extract::OriginalUri(uri): axum::extract::OriginalUri,
) -> axum::response::Response<String> {
    // Strip a trailing slug segment so /share/3/smokin-mclaren-p1-gtr still works.
    let card_id = id.split('/').next().unwrap_or(&id);

    let row = sqlx::query("SELECT body FROM cards WHERE id = ? AND deleted_at IS NULL")
        .bind(card_id)
        .fetch_optional(&st.pool)
        .await
        .ok()
        .flatten();

    let Some(row) = row else {
        return axum::response::Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header("content-type", "text/html; charset=utf-8")
            .body("<html><body>Card not found.</body></html>".into())
            .unwrap();
    };

    let mut body: Value = serde_json::from_str(row.get::<String, _>("body").as_str())
        .unwrap_or(Value::Null);
    inject_images(&st.pool, &mut body).await;

    let card_name = body.get("name").and_then(Value::as_str).unwrap_or("Livery");
    let subtitle  = body.get("subtitle").and_then(Value::as_str).unwrap_or("");

    // Pull the first car name from the recipe section's cars[] hierarchy.
    let first_car_name = body.get("sections")
        .and_then(Value::as_array)
        .and_then(|secs| secs.iter().find(|s| s.get("type").and_then(Value::as_str) == Some("forza_recipe")))
        .and_then(|recipe| recipe.get("cars"))
        .and_then(Value::as_array)
        .and_then(|cars| cars.first())
        .and_then(|car| car.get("carName"))
        .and_then(Value::as_str)
        .unwrap_or("");

    // First share code from the first tune of the first car.
    let share_code = body.get("sections")
        .and_then(Value::as_array)
        .and_then(|secs| secs.iter().find(|s| s.get("type").and_then(Value::as_str) == Some("forza_recipe")))
        .and_then(|recipe| recipe.get("cars"))
        .and_then(Value::as_array)
        .and_then(|cars| cars.first())
        .and_then(|car| car.get("tunes"))
        .and_then(Value::as_array)
        .and_then(|tunes| tunes.first())
        .and_then(|tune| tune.get("shareCode"))
        .and_then(Value::as_str)
        .unwrap_or("");

    // Point og:image at the card.png route (absolute URL required by OG spec).
    // The route currently redirects to the lead photo; later it will return a
    // compositor-generated PNG with the card's overlay preset applied.
    let has_images = body.get("images")
        .and_then(Value::as_array)
        .map(|imgs| !imgs.is_empty())
        .unwrap_or(false);
    let scheme = if host.starts_with("localhost") || host.starts_with("127.") { "http" } else { "https" };
    let og_image = if has_images {
        format!("{scheme}://{host}/share/{card_id}/card.png")
    } else {
        String::new()
    };

    // Build a canonical URL from the request path (no query string).
    let canonical = uri.path().to_string();

    let og_title = if first_car_name.is_empty() {
        card_name.to_string()
    } else {
        format!("{card_name} — {first_car_name}")
    };

    let mut description_parts = Vec::new();
    if !subtitle.is_empty() { description_parts.push(subtitle.to_string()); }
    if !share_code.is_empty() { description_parts.push(format!("Share code: {share_code}")); }
    let og_description = if description_parts.is_empty() {
        "View this livery on The Livery Catalog.".to_string()
    } else {
        description_parts.join(" · ")
    };

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>{og_title}</title>
  <meta property="og:title" content="{og_title}">
  <meta property="og:description" content="{og_description}">
  <meta property="og:type" content="website">
  <meta property="og:url" content="{canonical}">
  {og_image_tag}
  <meta name="twitter:card" content="summary_large_image">
  <meta name="twitter:title" content="{og_title}">
  <meta name="twitter:description" content="{og_description}">
  <meta http-equiv="refresh" content="0;url=/">
</head>
<body>
  <p>Redirecting to <a href="/">The Livery Catalog</a>…</p>
</body>
</html>"#,
        og_image_tag = if og_image.is_empty() {
            String::new()
        } else {
            format!(r#"<meta property="og:image" content="{og_image}">"#)
        },
    );

    axum::response::Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "text/html; charset=utf-8")
        .body(html)
        .unwrap()
}

/// Returns a compositor-generated PNG when the card has a share_overlay_config,
/// otherwise redirects to the lead photo (fallback while presets are being built).
pub async fn share_card_png(
    State(st): State<AppState>,
    Path(id): Path<String>,
) -> axum::response::Response<Body> {
    let row = sqlx::query("SELECT body FROM cards WHERE id = ? AND deleted_at IS NULL")
        .bind(&id)
        .fetch_optional(&st.pool)
        .await
        .ok()
        .flatten();

    let Some(row) = row else {
        return axum::response::Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::empty())
            .unwrap();
    };

    let mut body: Value = serde_json::from_str(row.get::<String, _>("body").as_str())
        .unwrap_or(Value::Null);
    inject_images(&st.pool, &mut body).await;

    // If the card has a saved overlay config, run the compositor.
    if let Some(config) = body.get("shareOverlayConfig")
        .and_then(|v| serde_json::from_value::<compositor::OgConfig>(v.clone()).ok())
    {
        if let Some(png) = image_path_for_id(&st, config.photo_id).await
            .and_then(|p| compositor::compose(&p, &config).ok())
        {
            return axum::response::Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "image/png")
                .header("cache-control", "public, max-age=300")
                .body(Body::from(png))
                .unwrap();
        }
    }

    // Fallback: redirect to the lead photo.
    let lead_path = body.get("images")
        .and_then(Value::as_array)
        .and_then(|imgs| imgs.first())
        .and_then(|img| img.get("path").and_then(Value::as_str))
        .map(str::to_owned);

    match lead_path {
        Some(path) => axum::response::Response::builder()
            .status(StatusCode::FOUND)
            .header("location", path)
            .body(Body::empty())
            .unwrap(),
        None => axum::response::Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::empty())
            .unwrap(),
    }
}

/// Live preview endpoint for the OG Maker. Accepts the same config shape as
/// share_overlay_config, calls the same compositor, returns PNG bytes.
/// Auth-gated — only the editor calls this.
pub async fn share_preview(
    State(st): State<AppState>,
    _auth: AuthUser,
    Json(config): Json<compositor::OgConfig>,
) -> axum::response::Response<Body> {
    match image_path_for_id(&st, config.photo_id).await {
        Some(path) => match compositor::compose(&path, &config) {
            Ok(png) => axum::response::Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "image/png")
                .header("cache-control", "no-store")
                .body(Body::from(png))
                .unwrap(),
            Err(_) => axum::response::Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::empty())
                .unwrap(),
        },
        None => axum::response::Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::empty())
            .unwrap(),
    }
}

// ── OG Presets ────────────────────────────────────────────────────────────────

pub async fn list_og_presets(
    State(st): State<AppState>,
    _auth: AuthUser,
) -> Result<Json<Vec<Value>>, ApiError> {
    let rows = sqlx::query("SELECT id, name, config, created_at, updated_at FROM og_presets ORDER BY id")
        .fetch_all(&st.pool)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let out = rows.iter().map(|r| json!({
        "id":        r.get::<i64, _>("id"),
        "name":      r.get::<String, _>("name"),
        "config":    serde_json::from_str::<Value>(r.get::<String, _>("config").as_str()).unwrap_or(json!({})),
        "createdAt": r.get::<String, _>("created_at"),
        "updatedAt": r.get::<String, _>("updated_at"),
    })).collect();
    Ok(Json(out))
}

#[derive(Deserialize)]
pub struct OgPresetBody {
    name: String,
    config: Option<Value>,
}

pub async fn create_og_preset(
    State(st): State<AppState>,
    _auth: AuthUser,
    Json(req): Json<OgPresetBody>,
) -> Result<Json<Value>, ApiError> {
    let config = serde_json::to_string(&req.config.unwrap_or(json!({}))).unwrap_or_default();
    let row = sqlx::query(
        "INSERT INTO og_presets (name, config) VALUES (?, ?) RETURNING id, name, config, created_at, updated_at"
    )
    .bind(&req.name)
    .bind(&config)
    .fetch_one(&st.pool)
    .await
    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(json!({
        "id":        row.get::<i64, _>("id"),
        "name":      row.get::<String, _>("name"),
        "config":    serde_json::from_str::<Value>(row.get::<String, _>("config").as_str()).unwrap_or(json!({})),
        "createdAt": row.get::<String, _>("created_at"),
        "updatedAt": row.get::<String, _>("updated_at"),
    })))
}

pub async fn update_og_preset(
    State(st): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<OgPresetBody>,
) -> Result<Json<Value>, ApiError> {
    let config = serde_json::to_string(&req.config.unwrap_or(json!({}))).unwrap_or_default();
    let row = sqlx::query(
        "UPDATE og_presets SET name = ?, config = ?, updated_at = datetime('now')
         WHERE id = ?
         RETURNING id, name, config, created_at, updated_at"
    )
    .bind(&req.name)
    .bind(&config)
    .bind(id)
    .fetch_optional(&st.pool)
    .await
    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    match row {
        Some(r) => Ok(Json(json!({
            "id":        r.get::<i64, _>("id"),
            "name":      r.get::<String, _>("name"),
            "config":    serde_json::from_str::<Value>(r.get::<String, _>("config").as_str()).unwrap_or(json!({})),
            "createdAt": r.get::<String, _>("created_at"),
            "updatedAt": r.get::<String, _>("updated_at"),
        }))),
        None => Err(err(StatusCode::NOT_FOUND, "preset not found")),
    }
}

pub async fn delete_og_preset(
    State(st): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    sqlx::query("DELETE FROM og_presets WHERE id = ?")
        .bind(id)
        .execute(&st.pool)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(StatusCode::NO_CONTENT)
}
