//! Car / livery / tune identity registry: the cars table, liveries and tunes
//! with their serial generators, tune types, and the AI color assessment.

use std::collections::HashMap;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::{Row, SqlitePool};

use crate::auth::{AdminUser, AuthUser};
use crate::state::{err, ApiError, AppState};

// --- Cars -------------------------------------------------------------------

pub async fn upsert_car(pool: &SqlitePool, c: &Value) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO cars (id, game, make, model, year, class, pi, drive, country, category, decade, status, dlc, code)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET
           game=excluded.game, make=excluded.make, model=excluded.model,
           year=excluded.year, class=excluded.class, pi=excluded.pi,
           drive=excluded.drive, country=excluded.country, category=excluded.category,
           decade=excluded.decade, status=excluded.status, dlc=excluded.dlc,
           code=COALESCE(excluded.code, cars.code)",
    )
    .bind(c["id"].as_str().unwrap_or_default())
    .bind(c["game"].as_str().unwrap_or_default())
    .bind(c["make"].as_str().unwrap_or_default())
    .bind(c["model"].as_str().unwrap_or_default())
    .bind(c["year"].as_i64())
    .bind(c["class"].as_str())
    .bind(c["pi"].as_i64())
    .bind(c["drive"].as_str())
    .bind(c["country"].as_str())
    .bind(c["category"].as_str())
    .bind(c["decade"].as_str())
    .bind(c["status"].as_str())
    .bind(c["dlc"].as_str())
    .bind(c["code"].as_str())
    .execute(pool)
    .await?;
    Ok(())
}

pub fn car_row_to_json(r: &sqlx::sqlite::SqliteRow) -> Value {
    json!({
        "id":       r.get::<String, _>("id"),
        "game":     r.get::<String, _>("game"),
        "make":     r.get::<String, _>("make"),
        "model":    r.get::<String, _>("model"),
        "year":     r.get::<Option<i64>, _>("year"),
        "class":    r.get::<Option<String>, _>("class"),
        "pi":       r.get::<Option<i64>, _>("pi"),
        "drive":    r.get::<Option<String>, _>("drive"),
        "country":  r.get::<Option<String>, _>("country"),
        "category": r.get::<Option<String>, _>("category"),
        "decade":   r.get::<Option<String>, _>("decade"),
        "status":   r.get::<Option<String>, _>("status"),
        "dlc":      r.get::<Option<String>, _>("dlc"),
        "code":     r.get::<Option<String>, _>("code"),
    })
}

pub async fn list_cars(
    State(st): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Value>, ApiError> {
    let rows = if let Some(game) = params.get("game") {
        sqlx::query(
            "SELECT id,game,make,model,year,class,pi,drive,country,category,decade,status,dlc,code
             FROM cars WHERE game = ? ORDER BY make, model, year",
        )
        .bind(game)
        .fetch_all(&st.pool)
        .await
    } else {
        sqlx::query(
            "SELECT id,game,make,model,year,class,pi,drive,country,category,decade,status,dlc,code
             FROM cars ORDER BY game, make, model, year",
        )
        .fetch_all(&st.pool)
        .await
    }
    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let cars: Vec<Value> = rows.iter().map(car_row_to_json).collect();
    Ok(Json(json!(cars)))
}

pub async fn create_car(
    auth: AuthUser,
    State(st): State<AppState>,
    Json(body): Json<Value>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    upsert_car(&st.pool, &body)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let car_id = body.get("id").and_then(Value::as_str).unwrap_or_default().to_string();
    crate::audit::record(&st.pool, &auth.username, "car.create", "car", Some(&car_id), None).await;
    Ok((StatusCode::CREATED, Json(body)))
}

pub async fn seed_cars_if_empty(pool: &SqlitePool) -> anyhow::Result<()> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM cars")
        .fetch_one(pool)
        .await?;
    if count > 0 {
        tracing::info!("db already has {count} cars; skipping car seed");
        return Ok(());
    }
    let Ok(raw) = std::fs::read_to_string("seed/cars.json") else {
        tracing::warn!("no cars seed at seed/cars.json; car registry will be empty");
        return Ok(());
    };
    let cars: Vec<Value> = serde_json::from_str(&raw)?;
    for c in &cars {
        upsert_car(pool, c).await?;
    }
    tracing::info!("seeded {} cars from seed/cars.json", cars.len());
    Ok(())
}

/// Seed the liveries registry on first run. Rows keep their explicit ids —
/// seeded card images reference liveries by id (images.livery_id FK), so the
/// ids must survive the trip through the seed file.
pub async fn seed_liveries_if_empty(pool: &SqlitePool, seed_path: &str) -> anyhow::Result<()> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM liveries")
        .fetch_one(pool)
        .await?;
    if count > 0 {
        tracing::info!("db already has {count} liveries; skipping livery seed");
        return Ok(());
    }
    let Ok(raw) = std::fs::read_to_string(seed_path) else {
        tracing::warn!("no liveries seed at {seed_path}; livery registry will be empty");
        return Ok(());
    };
    let liveries: Vec<Value> = serde_json::from_str(&raw)?;
    for l in &liveries {
        sqlx::query(
            "INSERT INTO liveries (id, car_id, serial, name, is_factory, share_code, color_primary, color_secondary, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, COALESCE(?, datetime('now')))",
        )
        .bind(l.get("id").and_then(Value::as_i64))
        .bind(l.get("car_id").and_then(Value::as_str))
        .bind(l.get("serial").and_then(Value::as_str))
        .bind(l.get("name").and_then(Value::as_str))
        .bind(l.get("is_factory").and_then(Value::as_i64).unwrap_or(0))
        .bind(l.get("share_code").and_then(Value::as_str))
        .bind(l.get("color_primary").and_then(Value::as_str))
        .bind(l.get("color_secondary").and_then(Value::as_str))
        .bind(l.get("created_at").and_then(Value::as_str))
        .execute(pool)
        .await?;
    }
    tracing::info!("seeded {} liveries from {seed_path}", liveries.len());
    Ok(())
}

// --- AI color assessment ----------------------------------------------------

pub const COLOR_TAXONOMY: &[&str] = &[
    "Red", "Blue", "Green", "Yellow", "Orange", "Purple", "Pink",
    "White", "Black", "Silver", "Grey", "Gold", "Bronze", "Teal", "Multi",
];

// Editor-allowed despite the /api/admin/ path: color assessment runs as part of
// the normal upload flow (PhotoDetail, ImagePicker), and the result is derived,
// recomputable metadata — not a destructive operation.
pub async fn admin_assess_livery_color(
    _auth: AuthUser,
    State(st): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| err(StatusCode::SERVICE_UNAVAILABLE, "ANTHROPIC_API_KEY not configured"))?;

    // Fetch the livery and its lead image.
    let livery = sqlx::query("SELECT id, serial FROM liveries WHERE id = ?")
        .bind(id)
        .fetch_optional(&st.pool)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "livery not found"))?;

    let serial: String = livery.get("serial");

    let img_rows = sqlx::query(
        "SELECT path, thumb_path FROM images WHERE livery_id = ? ORDER BY sort_order ASC",
    )
    .bind(id)
    .fetch_all(&st.pool)
    .await
    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    if img_rows.is_empty() {
        return Err(err(StatusCode::NOT_FOUND, "no images tagged to this livery"));
    }

    // Try each image in order until one resolves on disk.
    let mut img_bytes_opt: Option<Vec<u8>> = None;
    for row in &img_rows {
        let thumb: Option<String> = row.try_get("thumb_path").unwrap_or(None);
        let img_path: String = thumb.filter(|s| !s.is_empty())
            .unwrap_or_else(|| row.get("path"));
        let rel = img_path.trim_start_matches('/').trim_start_matches("uploads/");
        let file_path = st.uploads_dir.join(rel);
        if let Ok(bytes) = std::fs::read(&file_path) {
            img_bytes_opt = Some(bytes);
            break;
        }
    }
    let img_bytes = img_bytes_opt
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "no image files found on disk for this livery"))?;

    // Detect media type from magic bytes (JPEG or PNG; fall back to JPEG).
    let media_type = if img_bytes.starts_with(b"\x89PNG") { "image/png" } else { "image/jpeg" };
    let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &img_bytes);

    let taxonomy = COLOR_TAXONOMY.join(", ");
    let prompt = format!(
        "This is a photo of a car with a custom livery (paint and vinyl wrap design). \
         Identify the 1-2 most dominant colors of the livery itself (ignore the background). \
         Choose ONLY from this list: {taxonomy}. \
         Reply with JSON only, no explanation: \
         {{\"primary\": \"Blue\", \"secondary\": \"Silver\"}} \
         Omit the secondary key if only one color dominates."
    );

    let body = serde_json::json!({
        "model": "claude-haiku-4-5-20251001",
        "max_tokens": 64,
        "messages": [{
            "role": "user",
            "content": [
                {
                    "type": "image",
                    "source": {
                        "type": "base64",
                        "media_type": media_type,
                        "data": b64,
                    }
                },
                { "type": "text", "text": prompt }
            ]
        }]
    });

    let client = reqwest::Client::new();
    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| err(StatusCode::BAD_GATEWAY, e.to_string()))?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();
        return Err(err(StatusCode::BAD_GATEWAY, format!("Anthropic API {status}: {text}")));
    }

    let resp_json: Value = resp.json().await
        .map_err(|e| err(StatusCode::BAD_GATEWAY, e.to_string()))?;

    // Extract the text content from the first choice.
    let text = resp_json["content"]
        .as_array()
        .and_then(|a| a.first())
        .and_then(|b| b["text"].as_str())
        .unwrap_or("");

    // Strip optional markdown fences (```json … ```) before parsing.
    let stripped = text.trim();
    let stripped = stripped.strip_prefix("```json").or_else(|| stripped.strip_prefix("```")).unwrap_or(stripped);
    let stripped = stripped.strip_suffix("```").unwrap_or(stripped).trim();
    let colors: Value = serde_json::from_str(stripped).unwrap_or(Value::Null);
    let primary   = colors["primary"].as_str().filter(|c| COLOR_TAXONOMY.contains(c));
    let secondary = colors["secondary"].as_str().filter(|c| COLOR_TAXONOMY.contains(c));

    if primary.is_none() {
        return Err(err(StatusCode::UNPROCESSABLE_ENTITY,
            format!("model returned unrecognised colors — raw: {text}")));
    }

    sqlx::query(
        "UPDATE liveries SET color_primary = ?, color_secondary = ? WHERE id = ?",
    )
    .bind(primary)
    .bind(secondary)
    .bind(id)
    .execute(&st.pool)
    .await
    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    tracing::info!("color assessment: {serial} → primary={primary:?} secondary={secondary:?}");

    Ok(Json(json!({
        "id":        id,
        "serial":    serial,
        "primary":   primary,
        "secondary": secondary,
    })))
}

// --- Serial generator -------------------------------------------------------

/// Returns the next L### serial for a car, e.g. "FH6-NISRVGTSP99-L003".
/// Reads MAX of existing serials for that car rather than COUNT so deletions
/// don't cause collisions.
pub async fn next_livery_serial(pool: &SqlitePool, car_id: &str) -> Result<String, sqlx::Error> {
    // Fetch car game + code so we can build the prefix.
    let row = sqlx::query("SELECT game, code FROM cars WHERE id = ?")
        .bind(car_id)
        .fetch_one(pool)
        .await?;
    let game: String = row.get("game");
    let code: Option<String> = row.get("code");
    // Fallback: djb2 hash of car_id → 8-char hex, guaranteed unique per id.
    let car_code = code.unwrap_or_else(|| {
        let mut h: u64 = 5381;
        for b in car_id.bytes() { h = h.wrapping_mul(33).wrapping_add(b as u64); }
        format!("{:08X}", h & 0xFFFFFFFF)
    });

    let prefix = format!("{}-{}-L", game, car_code);
    let row2 = sqlx::query("SELECT MAX(CAST(SUBSTR(serial, ?) AS INTEGER)) FROM liveries WHERE car_id = ?")
        .bind((prefix.len() + 1) as i64)
        .bind(car_id)
        .fetch_one(pool)
        .await?;
    let max_n: Option<i64> = row2.try_get::<Option<i64>, _>(0).unwrap_or(None);
    let n = max_n.unwrap_or(0) + 1;
    Ok(format!("{}{:03}", prefix, n))
}

/// Returns the next T### serial for a livery, e.g. "FH6-NISRVGTSP99-L001-T002".
pub async fn next_tune_serial(pool: &SqlitePool, livery_id: i64) -> Result<String, sqlx::Error> {
    let row = sqlx::query("SELECT serial FROM liveries WHERE id = ?")
        .bind(livery_id)
        .fetch_one(pool)
        .await?;
    let livery_serial: String = row.get("serial");

    let prefix = format!("{}-T", livery_serial);
    let row2 = sqlx::query("SELECT MAX(CAST(SUBSTR(serial, ?) AS INTEGER)) FROM tunes WHERE livery_id = ?")
        .bind((prefix.len() + 1) as i64)
        .bind(livery_id)
        .fetch_one(pool)
        .await?;
    let max_n: Option<i64> = row2.try_get::<Option<i64>, _>(0).unwrap_or(None);
    let n = max_n.unwrap_or(0) + 1;
    Ok(format!("{}{:03}", prefix, n))
}

// --- Tune types -------------------------------------------------------------

pub async fn list_tune_types(
    State(st): State<AppState>,
) -> Result<Json<Value>, ApiError> {
    let rows = sqlx::query("SELECT id, name, sort_order FROM tune_types ORDER BY sort_order, name")
        .fetch_all(&st.pool)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let types: Vec<Value> = rows.iter().map(|r| json!({
        "id":        r.get::<i64, _>("id"),
        "name":      r.get::<String, _>("name"),
        "sortOrder": r.get::<i64, _>("sort_order"),
    })).collect();
    Ok(Json(json!(types)))
}

#[derive(Deserialize)]
pub struct CreateTuneTypeReq {
    name: String,
    #[serde(rename = "sortOrder")]
    sort_order: Option<i64>,
}

pub async fn create_tune_type(
    _auth: AuthUser,
    State(st): State<AppState>,
    Json(req): Json<CreateTuneTypeReq>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let id = sqlx::query("INSERT INTO tune_types (name, sort_order) VALUES (?, ?)")
        .bind(&req.name)
        .bind(req.sort_order.unwrap_or(0))
        .execute(&st.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("UNIQUE") {
                err(StatusCode::CONFLICT, "tune type name already exists")
            } else {
                err(StatusCode::INTERNAL_SERVER_ERROR, e)
            }
        })?
        .last_insert_rowid();
    Ok((StatusCode::CREATED, Json(json!({ "id": id, "name": req.name }))))
}

// --- Liveries ---------------------------------------------------------------

pub fn livery_row_to_json(r: &sqlx::sqlite::SqliteRow) -> Value {
    json!({
        "id":             r.get::<i64, _>("id"),
        "carId":          r.get::<String, _>("car_id"),
        "serial":         r.get::<String, _>("serial"),
        "name":           r.get::<String, _>("name"),
        "isFactory":      r.get::<bool, _>("is_factory"),
        "carColorId":     r.get::<Option<i64>, _>("car_color_id"),
        "shareCode":      r.get::<Option<String>, _>("share_code"),
        "colorPrimary":   r.get::<Option<String>, _>("color_primary"),
        "colorSecondary": r.get::<Option<String>, _>("color_secondary"),
        "createdAt":      r.get::<String, _>("created_at"),
    })
}

pub async fn list_liveries(
    State(st): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Value>, ApiError> {
    let rows = if let Some(car_id) = params.get("carId") {
        sqlx::query(
            "SELECT id,car_id,serial,name,is_factory,car_color_id,share_code,color_primary,color_secondary,created_at
             FROM liveries WHERE car_id = ? ORDER BY serial",
        )
        .bind(car_id)
        .fetch_all(&st.pool)
        .await
    } else {
        sqlx::query(
            "SELECT id,car_id,serial,name,is_factory,car_color_id,share_code,color_primary,color_secondary,created_at
             FROM liveries ORDER BY car_id, serial",
        )
        .fetch_all(&st.pool)
        .await
    }
    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let liveries: Vec<Value> = rows.iter().map(livery_row_to_json).collect();
    Ok(Json(json!(liveries)))
}

#[derive(Deserialize)]
pub struct CreateLiveryReq {
    #[serde(rename = "carId")]
    car_id: String,
    name: String,
    #[serde(rename = "isFactory")]
    is_factory: Option<bool>,
    #[serde(rename = "carColorId")]
    car_color_id: Option<i64>,
    #[serde(rename = "shareCode")]
    share_code: Option<String>,
    #[serde(rename = "colorPrimary")]
    color_primary: Option<String>,
    #[serde(rename = "colorSecondary")]
    color_secondary: Option<String>,
}

pub async fn create_livery(
    auth: AuthUser,
    State(st): State<AppState>,
    Json(req): Json<CreateLiveryReq>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let serial = next_livery_serial(&st.pool, &req.car_id)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let id = sqlx::query(
        "INSERT INTO liveries (car_id, serial, name, is_factory, car_color_id, share_code, color_primary, color_secondary)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&req.car_id)
    .bind(&serial)
    .bind(&req.name)
    .bind(req.is_factory.unwrap_or(false))
    .bind(req.car_color_id)
    .bind(&req.share_code)
    .bind(&req.color_primary)
    .bind(&req.color_secondary)
    .execute(&st.pool)
    .await
    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?
    .last_insert_rowid();

    crate::audit::record(&st.pool, &auth.username, "livery.create", "livery", Some(&id.to_string()), None).await;
    Ok((StatusCode::CREATED, Json(json!({ "id": id, "serial": serial }))))
}

#[derive(Deserialize)]
pub struct UpdateLiveryReq {
    name: Option<String>,
    #[serde(rename = "isFactory")]
    is_factory: Option<bool>,
    #[serde(rename = "carColorId")]
    car_color_id: Option<i64>,
    #[serde(rename = "shareCode")]
    share_code: Option<String>,
    #[serde(rename = "colorPrimary")]
    color_primary: Option<String>,
    #[serde(rename = "colorSecondary")]
    color_secondary: Option<String>,
}

pub async fn update_livery(
    auth: AuthUser,
    State(st): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateLiveryReq>,
) -> Result<Json<Value>, ApiError> {
    // Snapshot the mutable columns first — the audit entry's `detail.prev` is
    // the only way to reverse this overwrite.
    let prev = sqlx::query(
        "SELECT name, is_factory, car_color_id, share_code, color_primary, color_secondary
         FROM liveries WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(&st.pool)
    .await
    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?
    .map(|r| json!({
        "name":           r.get::<Option<String>, _>("name"),
        "isFactory":      r.get::<bool, _>("is_factory"),
        "carColorId":     r.get::<Option<i64>, _>("car_color_id"),
        "shareCode":      r.get::<Option<String>, _>("share_code"),
        "colorPrimary":   r.get::<Option<String>, _>("color_primary"),
        "colorSecondary": r.get::<Option<String>, _>("color_secondary"),
    }));

    let affected = sqlx::query(
        "UPDATE liveries SET
           name            = COALESCE(?, name),
           is_factory      = COALESCE(?, is_factory),
           car_color_id    = COALESCE(?, car_color_id),
           share_code      = COALESCE(?, share_code),
           color_primary   = COALESCE(?, color_primary),
           color_secondary = COALESCE(?, color_secondary)
         WHERE id = ?",
    )
    .bind(&req.name)
    .bind(req.is_factory)
    .bind(req.car_color_id)
    .bind(&req.share_code)
    .bind(&req.color_primary)
    .bind(&req.color_secondary)
    .bind(id)
    .execute(&st.pool)
    .await
    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?
    .rows_affected();

    if affected == 0 {
        return Err(err(StatusCode::NOT_FOUND, "livery not found"));
    }
    crate::audit::record(
        &st.pool, &auth.username, "livery.update", "livery", Some(&id.to_string()),
        prev.map(|p| json!({ "prev": p })),
    ).await;
    Ok(Json(json!({ "id": id })))
}

pub async fn delete_livery(
    _admin: AdminUser,
    State(st): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let affected = sqlx::query("DELETE FROM liveries WHERE id = ?")
        .bind(id)
        .execute(&st.pool)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?
        .rows_affected();
    if affected == 0 {
        return Err(err(StatusCode::NOT_FOUND, "livery not found"));
    }
    Ok(Json(json!({ "deleted": id })))
}

// --- Tunes ------------------------------------------------------------------

pub fn tune_row_to_json(r: &sqlx::sqlite::SqliteRow) -> Value {
    json!({
        "id":           r.get::<i64, _>("id"),
        "liveryId":     r.get::<i64, _>("livery_id"),
        "carId":        r.get::<String, _>("car_id"),
        "serial":       r.get::<String, _>("serial"),
        "officialName": r.get::<Option<String>, _>("official_name"),
        "typeId":       r.get::<Option<i64>, _>("type_id"),
        "shareCode":    r.get::<Option<String>, _>("share_code"),
        "coreSpecs":    r.get::<Option<String>, _>("core_specs"),
        "upgrades":     r.get::<Option<String>, _>("upgrades"),
        "adjustments":  r.get::<Option<String>, _>("adjustments"),
        "createdAt":    r.get::<String, _>("created_at"),
    })
}

pub async fn list_tunes(
    State(st): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Value>, ApiError> {
    let rows = if let Some(livery_id) = params.get("liveryId") {
        sqlx::query(
            "SELECT id,livery_id,car_id,serial,official_name,type_id,share_code,core_specs,upgrades,adjustments,created_at
             FROM tunes WHERE livery_id = ? ORDER BY serial",
        )
        .bind(livery_id.parse::<i64>().unwrap_or(0))
        .fetch_all(&st.pool)
        .await
    } else if let Some(car_id) = params.get("carId") {
        sqlx::query(
            "SELECT id,livery_id,car_id,serial,official_name,type_id,share_code,core_specs,upgrades,adjustments,created_at
             FROM tunes WHERE car_id = ? ORDER BY serial",
        )
        .bind(car_id)
        .fetch_all(&st.pool)
        .await
    } else {
        sqlx::query(
            "SELECT id,livery_id,car_id,serial,official_name,type_id,share_code,core_specs,upgrades,adjustments,created_at
             FROM tunes ORDER BY car_id, serial",
        )
        .fetch_all(&st.pool)
        .await
    }
    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let tunes: Vec<Value> = rows.iter().map(tune_row_to_json).collect();
    Ok(Json(json!(tunes)))
}

#[derive(Deserialize)]
pub struct CreateTuneReq {
    #[serde(rename = "liveryId")]
    livery_id: i64,
    #[serde(rename = "carId")]
    car_id: String,
    #[serde(rename = "officialName")]
    official_name: Option<String>,
    #[serde(rename = "typeId")]
    type_id: Option<i64>,
    #[serde(rename = "shareCode")]
    share_code: Option<String>,
    #[serde(rename = "coreSpecs")]
    core_specs: Option<Value>,
    upgrades: Option<Value>,
    adjustments: Option<Value>,
}

pub async fn create_tune(
    auth: AuthUser,
    State(st): State<AppState>,
    Json(req): Json<CreateTuneReq>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let serial = next_tune_serial(&st.pool, req.livery_id)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let core_specs_str = req.core_specs.as_ref().map(|v| v.to_string());
    let upgrades_str   = req.upgrades.as_ref().map(|v| v.to_string());
    let adjustments_str = req.adjustments.as_ref().map(|v| v.to_string());

    let id = sqlx::query(
        "INSERT INTO tunes (livery_id, car_id, serial, official_name, type_id, share_code, core_specs, upgrades, adjustments)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(req.livery_id)
    .bind(&req.car_id)
    .bind(&serial)
    .bind(&req.official_name)
    .bind(req.type_id)
    .bind(&req.share_code)
    .bind(&core_specs_str)
    .bind(&upgrades_str)
    .bind(&adjustments_str)
    .execute(&st.pool)
    .await
    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?
    .last_insert_rowid();

    crate::audit::record(&st.pool, &auth.username, "tune.create", "tune", Some(&id.to_string()), None).await;
    Ok((StatusCode::CREATED, Json(json!({ "id": id, "serial": serial }))))
}

#[derive(Deserialize)]
pub struct UpdateTuneReq {
    #[serde(rename = "officialName")]
    official_name: Option<String>,
    #[serde(rename = "typeId")]
    type_id: Option<i64>,
    #[serde(rename = "shareCode")]
    share_code: Option<String>,
    #[serde(rename = "coreSpecs")]
    core_specs: Option<Value>,
    upgrades: Option<Value>,
    adjustments: Option<Value>,
}

pub async fn update_tune(
    auth: AuthUser,
    State(st): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateTuneReq>,
) -> Result<Json<Value>, ApiError> {
    // Snapshot for the audit trail — `detail.prev` reverses this overwrite.
    let prev = sqlx::query("SELECT * FROM tunes WHERE id = ?")
        .bind(id)
        .fetch_optional(&st.pool)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?
        .map(|r| tune_row_to_json(&r));

    let core_specs_str  = req.core_specs.as_ref().map(|v| v.to_string());
    let upgrades_str    = req.upgrades.as_ref().map(|v| v.to_string());
    let adjustments_str = req.adjustments.as_ref().map(|v| v.to_string());

    let affected = sqlx::query(
        "UPDATE tunes SET
           official_name = COALESCE(?, official_name),
           type_id       = COALESCE(?, type_id),
           share_code    = COALESCE(?, share_code),
           core_specs    = COALESCE(?, core_specs),
           upgrades      = COALESCE(?, upgrades),
           adjustments   = COALESCE(?, adjustments)
         WHERE id = ?",
    )
    .bind(&req.official_name)
    .bind(req.type_id)
    .bind(&req.share_code)
    .bind(&core_specs_str)
    .bind(&upgrades_str)
    .bind(&adjustments_str)
    .bind(id)
    .execute(&st.pool)
    .await
    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?
    .rows_affected();

    if affected == 0 {
        return Err(err(StatusCode::NOT_FOUND, "tune not found"));
    }
    crate::audit::record(
        &st.pool, &auth.username, "tune.update", "tune", Some(&id.to_string()),
        prev.map(|p| json!({ "prev": p })),
    ).await;
    Ok(Json(json!({ "id": id })))
}

pub async fn delete_tune(
    _admin: AdminUser,
    State(st): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let affected = sqlx::query("DELETE FROM tunes WHERE id = ?")
        .bind(id)
        .execute(&st.pool)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?
        .rows_affected();
    if affected == 0 {
        return Err(err(StatusCode::NOT_FOUND, "tune not found"));
    }
    Ok(Json(json!({ "deleted": id })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn seed_liveries_preserves_explicit_ids() {
        let pool = crate::testutil::test_pool().await;
        // Livery FK target: car row must exist first.
        upsert_car(&pool, &json!({ "id": "fh5-test-car", "game": "FH5", "make": "Test", "model": "Car" }))
            .await
            .unwrap();

        let seed = json!([{
            "id": 24, "car_id": "fh5-test-car", "serial": "FH5-TEST-L001",
            "name": "Faker", "is_factory": 0, "share_code": null,
            "color_primary": "Green", "color_secondary": null,
            "created_at": "2026-07-06 09:25:02"
        }]);
        let path = std::env::temp_dir().join("seed_liveries_test.json");
        std::fs::write(&path, seed.to_string()).unwrap();

        seed_liveries_if_empty(&pool, path.to_str().unwrap()).await.unwrap();

        let id: i64 = sqlx::query_scalar("SELECT id FROM liveries WHERE serial = 'FH5-TEST-L001'")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(id, 24, "seeded livery keeps its explicit id");

        // Second run is a no-op (table non-empty).
        seed_liveries_if_empty(&pool, path.to_str().unwrap()).await.unwrap();
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM liveries")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(count, 1, "reseed on populated table is a no-op");
    }
}
