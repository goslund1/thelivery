//! Images: the images table as single source of truth (inject on read, sync on
//! write), multipart upload with structured naming, re-file migration, and
//! figure-path repair.

use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    Json,
};
use serde_json::{json, Value};
use sqlx::{Row, SqlitePool};

use crate::auth::{AdminUser, AuthUser};
use crate::state::{err, ApiError, AppState};

/// Fetch images for a card from the authoritative images table.
pub async fn fetch_images_for_card(pool: &SqlitePool, card_id: &str) -> Vec<Value> {
    let rows = sqlx::query(
        "SELECT id, path, thumb_path, stage_path, alt_text, sort_order, car_id, livery_id, image_role, included \
         FROM images WHERE card_id = ? ORDER BY sort_order ASC",
    )
    .bind(card_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    rows.iter().map(image_row_to_json).collect()
}

fn image_row_to_json(r: &sqlx::sqlite::SqliteRow) -> Value {
    json!({
        "id":        r.get::<i64, _>("id"),
        "path":      r.get::<String, _>("path"),
        "thumbPath": r.get::<Option<String>, _>("thumb_path"),
        "stagePath": r.get::<Option<String>, _>("stage_path"),
        "alt":       r.get::<Option<String>, _>("alt_text").unwrap_or_default(),
        "order":     r.get::<i64, _>("sort_order"),
        "carId":     r.get::<Option<String>, _>("car_id"),
        "liveryId":  r.get::<Option<i64>, _>("livery_id"),
        "imageRole": r.get::<Option<String>, _>("image_role").unwrap_or_else(|| "gallery".into()),
        "included":  r.get::<i64, _>("included") != 0,
    })
}

/// Fetch every image row in one query, grouped by card_id. Used by list_cards
/// to inject images for the whole catalog without a per-card query.
pub async fn fetch_all_images_grouped(pool: &SqlitePool) -> std::collections::HashMap<String, Vec<Value>> {
    let rows = sqlx::query(
        "SELECT id, card_id, path, thumb_path, stage_path, alt_text, sort_order, car_id, livery_id, image_role, included \
         FROM images ORDER BY card_id, sort_order ASC",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let mut map: std::collections::HashMap<String, Vec<Value>> = std::collections::HashMap::new();
    for r in &rows {
        let card_id: Option<String> = r.get("card_id");
        let Some(card_id) = card_id else { continue };
        map.entry(card_id).or_default().push(image_row_to_json(r));
    }
    map
}

/// Replace body["images"] with rows from the images table.
/// Falls back to body["images"] as-is when no DB rows exist (unmigrated card).
pub async fn inject_images(pool: &SqlitePool, body: &mut Value) {
    let card_id = match body.get("id").and_then(Value::as_str) {
        Some(id) => id.to_string(),
        None => return,
    };
    let db_images = fetch_images_for_card(pool, &card_id).await;
    if !db_images.is_empty() {
        body["images"] = json!(db_images);
    }
}

/// Sync card body images into the images table; strip paths from body (store only id+meta).
/// Images with a numeric id → UPDATE metadata.
/// Images with a path but no numeric id → INSERT or find-by-path then UPDATE.
pub async fn sync_card_images(pool: &SqlitePool, card_id: &str, body: &mut Value) -> Result<(), sqlx::Error> {
    let images = match body.get("images").and_then(Value::as_array) {
        Some(arr) if !arr.is_empty() => arr.clone(),
        _ => return Ok(()),
    };

    let mut synced: Vec<Value> = Vec::new();
    for img in &images {
        let db_id: Option<i64> = img.get("id").and_then(Value::as_i64);
        let path    = img.get("path").and_then(Value::as_str).map(String::from);
        let alt     = img.get("alt").and_then(Value::as_str).unwrap_or_default().to_string();
        let order   = img.get("order").and_then(Value::as_i64).unwrap_or(0);
        let car_id  = img.get("carId").and_then(Value::as_str).map(String::from);
        let livery_id: Option<i64> = img.get("liveryId").and_then(Value::as_i64);

        let included: Option<i64> = img.get("included").and_then(Value::as_bool).map(|b| b as i64);

        let final_id: i64 = if let Some(id) = db_id {
            sqlx::query(
                "UPDATE images SET alt_text = ?, sort_order = ?, car_id = ?, livery_id = COALESCE(?, livery_id), included = COALESCE(?, included) WHERE id = ? AND card_id = ?",
            )
            .bind(&alt).bind(order).bind(&car_id).bind(livery_id).bind(included).bind(id).bind(card_id)
            .execute(pool).await?;
            id
        } else if let Some(ref p) = path {
            let existing: Option<i64> = sqlx::query_scalar(
                "SELECT id FROM images WHERE card_id = ? AND path = ?",
            )
            .bind(card_id).bind(p)
            .fetch_optional(pool).await?;

            if let Some(existing_id) = existing {
                sqlx::query(
                    "UPDATE images SET alt_text = ?, sort_order = ?, car_id = ?, livery_id = COALESCE(?, livery_id), included = COALESCE(?, included) WHERE id = ?",
                )
                .bind(&alt).bind(order).bind(&car_id).bind(livery_id).bind(included).bind(existing_id)
                .execute(pool).await?;
                existing_id
            } else {
                let thumb  = img.get("thumbPath").and_then(Value::as_str).map(String::from);
                let stage  = img.get("stagePath").and_then(Value::as_str).map(String::from);
                let livery: Option<i64> = img.get("liveryId").and_then(Value::as_i64);
                sqlx::query(
                    "INSERT INTO images (card_id, path, thumb_path, stage_path, car_id, alt_text, sort_order, livery_id) \
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(card_id).bind(p).bind(&thumb).bind(&stage)
                .bind(&car_id).bind(&alt).bind(order).bind(livery)
                .execute(pool).await?.last_insert_rowid()
            }
        } else {
            continue; // no id and no path — skip
        };

        synced.push(json!({ "id": final_id, "alt": alt, "order": order, "carId": car_id, "included": included.map(|v| v != 0) }));
    }

    body["images"] = json!(synced);
    Ok(())
}

/// Resolve an image id to its full filesystem path.
pub async fn image_path_for_id(st: &AppState, image_id: i64) -> Option<std::path::PathBuf> {
    let row = sqlx::query("SELECT path FROM images WHERE id = ?")
        .bind(image_id)
        .fetch_optional(&st.pool)
        .await
        .ok()
        .flatten()?;
    let path_str: String = row.get("path");
    // path is stored as /uploads/filename — strip the leading /uploads/ prefix.
    let rel = path_str.trim_start_matches("/uploads/");
    Some(st.uploads_dir.join(rel))
}

/// Sanitise a string into a filesystem-safe slug: lowercase, collapse
/// non-alphanumeric runs to a single underscore, strip leading/trailing underscores.
pub fn slugify(s: &str) -> String {
    let mut out = String::new();
    let mut last_under = true; // start true so we never get a leading _
    for ch in s.chars() {
        if ch.is_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_under = false;
        } else if !last_under {
            out.push('_');
            last_under = true;
        }
    }
    if out.ends_with('_') { out.pop(); }
    out
}

/// Card folder: {card_name_slug}_{card_id}  e.g. smokin_abc123
/// Falls back to "misc" when either field is absent.
pub fn card_folder(name: &str, card_id: &str) -> String {
    let slug = slugify(name);
    match (slug.is_empty(), card_id.is_empty()) {
        (false, false) => format!("{slug}_{card_id}"),
        (false, true)  => slug,
        (true,  false) => format!("card_{card_id}"),
        (true,  true)  => "misc".into(),
    }
}

/// Build a structured image stem from car + livery DB data.
/// Pattern: {GAME}_{make}_{model}_{year}_{livery}_{NNN}_{YYYYMMDD}_{uuid6}
/// Falls back to a UUID stem when car data is absent.
pub async fn build_image_stem(
    pool: &sqlx::SqlitePool,
    card_id: &str,
    car_id: &Option<String>,
    livery_id: Option<i64>,
    file_index: Option<u32>,
    image_role: &str,
) -> String {
    let uuid_full = uuid::Uuid::new_v4().to_string();
    let uuid6 = uuid_full.replace('-', "");
    let uuid6 = &uuid6[..6];

    // Date from SQLite so we don't need chrono.
    let date: String = sqlx::query_scalar("SELECT strftime('%Y%m%d', 'now')")
        .fetch_one(pool).await.unwrap_or_else(|_| "00000000".into());

    // RefImg path: no car, sequential across all refimg images for this card.
    if image_role == "refimg" {
        let card_slug: String = sqlx::query_scalar(
            "SELECT json_extract(body, '$.name') FROM cards WHERE id = ?"
        )
        .bind(card_id)
        .fetch_optional(pool).await.ok().flatten().unwrap_or_default();
        let slug = slugify(&card_slug);

        let existing: i64 = if !card_id.is_empty() {
            sqlx::query_scalar(
                "SELECT COUNT(*) FROM images WHERE card_id = ? AND image_role = 'refimg'"
            )
            .bind(card_id)
            .fetch_one(pool).await.unwrap_or(0)
        } else {
            file_index.map(|i| i as i64).unwrap_or(0)
        };
        let nn = existing as u32 + 1;
        return format!("{slug}_RefImg{nn:02}_{date}_{uuid6}");
    }

    let Some(cid) = car_id else {
        // No car and not a refimg — simple fallback.
        return format!("img_{date}_{uuid6}");
    };

    // Look up car fields.
    let car_row = sqlx::query(
        "SELECT game, make, model, year FROM cars WHERE id = ?"
    )
    .bind(cid)
    .fetch_optional(pool).await.ok().flatten();

    let Some(car) = car_row else {
        return format!("img_{date}_{uuid6}");
    };

    let game: String = car.try_get("game").unwrap_or_else(|_| "FHX".into());
    let make: String = car.try_get("make").unwrap_or_default();
    let model: String = car.try_get("model").unwrap_or_default();
    let year: Option<i64> = car.try_get("year").ok().flatten();
    let year_s = year.map(|y| y.to_string()).unwrap_or_else(|| "xxxx".into());

    // Look up livery name.
    let livery_slug = if let Some(lid) = livery_id {
        let name: Option<String> = sqlx::query_scalar(
            "SELECT name FROM liveries WHERE id = ?"
        )
        .bind(lid)
        .fetch_optional(pool).await.ok().flatten();
        name.map(|n| slugify(&n)).filter(|s| !s.is_empty())
    } else {
        None
    };

    // NNN: count of images already stored for this card + livery combo, +1.
    let existing: i64 = if !card_id.is_empty() {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM images WHERE card_id = ? AND livery_id IS ?"
        )
        .bind(card_id)
        .bind(livery_id)
        .fetch_one(pool).await.unwrap_or(0)
    } else {
        file_index.map(|i| i as i64).unwrap_or(0)
    };
    let nnn = existing as u32 + 1;

    let mut parts = vec![
        game.to_uppercase(),
        slugify(&make),
        slugify(&model),
        year_s,
    ];
    if let Some(ls) = livery_slug {
        parts.push(ls);
    }
    parts.push(format!("{nnn:03}"));
    parts.push(date);
    parts.push(uuid6.to_string());

    parts.retain(|p| !p.is_empty());
    parts.join("_")
}

/// Accept multipart fields (in any order before the file field):
///   cardName, cardSubtitle, cardCollections (comma-separated), fileIndex (optional u32)
///
/// Folder layout:
///   uploads/{folder}/{stem}.jpg                      ← original (JPEG)
///   uploads/{folder}/Lowres_Assets/{stem}_200w.jpg   ← thumb
///   uploads/{folder}/Lowres_Assets/{stem}_1000w.jpg  ← stage
///
/// stem is {:03} when fileIndex is supplied, otherwise a UUID.
/// If the numbered file already exists (duplicate import), a short UUID suffix is appended.
pub async fn upload_image(
    State(st): State<AppState>,
    auth: AuthUser,
    mut multipart: Multipart,
) -> Result<Json<Value>, ApiError> {
    let mut card_name = String::new();
    let mut card_id = String::new();
    let mut car_id: Option<String> = None;
    let mut livery_id: Option<i64> = None;
    let mut file_index: Option<u32> = None;
    let mut image_role = "gallery".to_string();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| err(StatusCode::BAD_REQUEST, e))?
    {
        match field.name() {
            Some("cardName") => {
                card_name = field.text().await.unwrap_or_default();
                continue;
            }
            // Legacy fields accepted but unused — folder no longer uses subtitle/collections.
            Some("cardSubtitle") | Some("cardCollections") => {
                let _ = field.text().await;
                continue;
            }
            Some("cardId") => {
                card_id = field.text().await.unwrap_or_default();
                continue;
            }
            Some("carId") => {
                let v = field.text().await.unwrap_or_default();
                car_id = if v.is_empty() { None } else { Some(v) };
                continue;
            }
            Some("liveryId") => {
                livery_id = field.text().await.ok().and_then(|s| s.parse().ok());
                continue;
            }
            Some("fileIndex") => {
                file_index = field.text().await.ok().and_then(|s| s.parse().ok());
                continue;
            }
            Some("imageRole") => {
                let v = field.text().await.unwrap_or_default();
                if v == "refimg" { image_role = v; }
                continue;
            }
            _ => {}
        }

        // This field is the file.
        let data = field
            .bytes()
            .await
            .map_err(|e| err(StatusCode::BAD_REQUEST, e))?;

        let folder = card_folder(&card_name, &card_id);
        let card_dir = st.uploads_dir.join(&folder);
        let lowres_dir = card_dir.join("Lowres_Assets");

        std::fs::create_dir_all(&card_dir)
            .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
        std::fs::create_dir_all(&lowres_dir)
            .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;

        // Decode image — all uploads become JPEG.
        let img = image::load_from_memory(&data)
            .map_err(|e| err(StatusCode::BAD_REQUEST, e))?;

        // Build stem from car identity when available.
        // Format: {game}_{make}_{model}_{year}_{livery}_{NNN}_{YYYYMMDD}_{uuid6}
        // Falls back to {folder}_{uuid} when no car is assigned.
        let stem = build_image_stem(&st.pool, &card_id, &car_id, livery_id, file_index, &image_role).await;

        // Generate resized variants before naming so we can embed actual dimensions.
        let (orig_w, orig_h) = (img.width(), img.height());
        let thumb = img.thumbnail(200, u32::MAX);
        let (thumb_w, thumb_h) = (thumb.width(), thumb.height());
        let stage = img.thumbnail(1000, u32::MAX);
        let (stage_w, stage_h) = (stage.width(), stage.height());

        let orig_name  = format!("{stem}_{orig_w}x{orig_h}.jpg");
        let thumb_name = format!("{stem}_{thumb_w}x{thumb_h}.jpg");
        let stage_name = format!("{stem}_{stage_w}x{stage_h}.jpg");

        img.save_with_format(card_dir.join(&orig_name), image::ImageFormat::Jpeg)
            .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
        let _ = thumb.save_with_format(lowres_dir.join(&thumb_name), image::ImageFormat::Jpeg);
        let _ = stage.save_with_format(lowres_dir.join(&stage_name), image::ImageFormat::Jpeg);

        let path       = format!("/uploads/{folder}/{orig_name}");
        let thumb_path = format!("/uploads/{folder}/Lowres_Assets/{thumb_name}");
        let stage_path = format!("/uploads/{folder}/Lowres_Assets/{stage_name}");

        // Insert a row into the images table when the card_id is known.
        let img_included: i64 = if image_role == "refimg" { 0 } else { 1 };
        let db_id: Option<i64> = if !card_id.is_empty() {
            let result = sqlx::query(
                "INSERT INTO images (card_id, path, thumb_path, stage_path, car_id, livery_id, image_role, included) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&card_id)
            .bind(&path)
            .bind(&thumb_path)
            .bind(&stage_path)
            .bind(&car_id)
            .bind(&livery_id)
            .bind(&image_role)
            .bind(img_included)
            .execute(&st.pool)
            .await;
            result.ok().map(|r| r.last_insert_rowid())
        } else {
            None
        };

        if let Some(id) = db_id {
            crate::audit::record(
                &st.pool, &auth.username, "image.upload", "image",
                Some(&id.to_string()), Some(json!({ "path": path, "cardId": card_id })),
            ).await;
        }
        return Ok(Json(json!({
            "id":        db_id,
            "path":      path,
            "thumbPath": thumb_path,
            "stagePath": stage_path,
            "carId":     car_id,
        })));
    }
    Err(err(StatusCode::BAD_REQUEST, "no file field in upload"))
}

// --- Image migration ---------------------------------------------------------
//
// POST /api/admin/images/migrate
// Body: { imageIds: [1,2,3], carId: "fh5-ferrari-...", liveryId: 7 }
//
// For each image:
//  1. Read the existing file from disk
//  2. Re-encode + save under new folder/filename scheme
//  3. Move old file + thumbs to uploads/bin/
//  4. Update the images table row with new paths, car_id, livery_id
//
// Returns a list of updated image records.

/// Scan all card bodies for text sections whose figurePath no longer exists on disk.
/// For each stale path, replaces it with the card's lead image stage_path (or path).
/// Returns { repaired, cleared } — cleared means the card had no images to fall back to.
pub async fn admin_repair_figure_paths(
    _admin: AdminUser,
    State(st): State<AppState>,
) -> Result<Json<Value>, ApiError> {
    let rows = sqlx::query("SELECT id, body FROM cards")
        .fetch_all(&st.pool)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let mut repaired = 0u64;
    let mut cleared = 0u64;

    for row in &rows {
        let card_id: String = row.get("id");
        let body_str: String = row.get("body");
        let mut body: Value = match serde_json::from_str(&body_str) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let sections = match body.get_mut("sections").and_then(Value::as_array_mut) {
            Some(s) => s,
            None => continue,
        };

        let mut changed = false;
        for section in sections.iter_mut() {
            let obj = match section.as_object_mut() {
                Some(o) => o,
                None => continue,
            };
            if obj.get("type").and_then(Value::as_str) != Some("text") { continue; }

            let figure_path = match obj.get("figurePath").and_then(Value::as_str) {
                Some(p) if !p.is_empty() => p.to_string(),
                _ => continue,
            };

            // Check if the file still exists on disk.
            let rel = figure_path.trim_start_matches('/').trim_start_matches("uploads/");
            if st.uploads_dir.join(rel).exists() { continue; }

            // Extract the zero-padded sequence number from the old stem (e.g. "_019.jpg" → "019").
            let old_stem = std::path::Path::new(&figure_path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("");
            let nnn = old_stem.rsplit('_').next().unwrap_or("");

            // Try to find an image for this card whose new path contains "_{nnn}_".
            // This works because the migration preserves sort order, so the sequence
            // number in the new structured name matches the old sequential suffix.
            let img = if !nnn.is_empty() && nnn.chars().all(|c| c.is_ascii_digit()) {
                let pattern = format!("%_{nnn}_%");
                sqlx::query(
                    "SELECT stage_path, path FROM images WHERE card_id = ? AND (path LIKE ? OR stage_path LIKE ?) LIMIT 1",
                )
                .bind(&card_id)
                .bind(&pattern)
                .bind(&pattern)
                .fetch_optional(&st.pool)
                .await
                .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?
            } else {
                None
            };

            // Fall back to the card's lead image if no sequence match found.
            let img = match img {
                Some(r) => Some(r),
                None => sqlx::query(
                    "SELECT stage_path, path FROM images WHERE card_id = ? ORDER BY sort_order ASC LIMIT 1",
                )
                .bind(&card_id)
                .fetch_optional(&st.pool)
                .await
                .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?,
            };

            if let Some(img) = img {
                let new_path: Option<String> = img
                    .try_get::<Option<String>, _>("stage_path").unwrap_or(None)
                    .or_else(|| img.try_get::<String, _>("path").ok());
                if let Some(p) = new_path {
                    obj.insert("figurePath".into(), json!(p));
                    repaired += 1;
                } else {
                    obj.remove("figurePath");
                    cleared += 1;
                }
            } else {
                obj.remove("figurePath");
                cleared += 1;
            }
            changed = true;
        }

        if changed {
            let new_body = serde_json::to_string(&body)
                .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
            sqlx::query("UPDATE cards SET body = ? WHERE id = ?")
                .bind(&new_body)
                .bind(&card_id)
                .execute(&st.pool)
                .await
                .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
        }
    }

    Ok(Json(json!({ "repaired": repaired, "cleared": cleared })))
}

pub async fn admin_migrate_images(
    _admin: AdminUser,
    State(st): State<AppState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let ids: Vec<i64> = body["imageIds"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_i64()).collect())
        .unwrap_or_default();

    if ids.is_empty() {
        return Err(err(StatusCode::BAD_REQUEST, "imageIds is required"));
    }

    let car_id: Option<String> = body["carId"].as_str()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    let livery_id: Option<i64> = body["liveryId"].as_i64();

    // Bin directory — moved originals land here, never auto-deleted.
    let bin_dir = st.uploads_dir.join("trash");
    std::fs::create_dir_all(&bin_dir)
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let mut results: Vec<Value> = Vec::new();

    for img_id in ids {
        // Fetch existing image row.
        let row = sqlx::query(
            "SELECT id, card_id, path, thumb_path, stage_path FROM images WHERE id = ?"
        )
        .bind(img_id)
        .fetch_optional(&st.pool)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;

        let Some(row) = row else { continue; };

        let card_id: String = row.try_get("card_id").unwrap_or_default();
        let old_path: String = row.try_get("path").unwrap_or_default();
        let old_thumb: Option<String> = row.try_get("thumb_path").unwrap_or(None);
        let old_stage: Option<String> = row.try_get("stage_path").unwrap_or(None);

        // Resolve the current file on disk.
        let rel = old_path.trim_start_matches('/').trim_start_matches("uploads/");
        let src_file = st.uploads_dir.join(rel);
        if !src_file.exists() {
            // File already gone — just update metadata and continue.
            sqlx::query(
                "UPDATE images SET car_id = ?, livery_id = ? WHERE id = ?"
            )
            .bind(&car_id)
            .bind(livery_id)
            .bind(img_id)
            .execute(&st.pool)
            .await.ok();
            continue;
        }

        // Read and re-encode the image.
        let data = std::fs::read(&src_file)
            .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
        let img = image::load_from_memory(&data)
            .map_err(|e| err(StatusCode::BAD_REQUEST, e))?;

        // Look up card name for the folder.
        let card_name: String = sqlx::query_scalar(
            "SELECT json_extract(body, '$.name') FROM cards WHERE id = ?"
        )
        .bind(&card_id)
        .fetch_optional(&st.pool).await.ok().flatten().unwrap_or_default();

        let folder = card_folder(&card_name, &card_id);
        let card_dir = st.uploads_dir.join(&folder);
        let lowres_dir = card_dir.join("Lowres_Assets");
        std::fs::create_dir_all(&card_dir)
            .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
        std::fs::create_dir_all(&lowres_dir)
            .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;

        let stem = build_image_stem(&st.pool, &card_id, &car_id, livery_id, None, "gallery").await;

        let (orig_w, orig_h) = (img.width(), img.height());
        let thumb = img.thumbnail(200, u32::MAX);
        let (thumb_w, thumb_h) = (thumb.width(), thumb.height());
        let stage = img.thumbnail(1000, u32::MAX);
        let (stage_w, stage_h) = (stage.width(), stage.height());

        let orig_name  = format!("{stem}_{orig_w}x{orig_h}.jpg");
        let thumb_name = format!("{stem}_{thumb_w}x{thumb_h}.jpg");
        let stage_name = format!("{stem}_{stage_w}x{stage_h}.jpg");

        img.save_with_format(card_dir.join(&orig_name), image::ImageFormat::Jpeg)
            .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
        let _ = thumb.save_with_format(lowres_dir.join(&thumb_name), image::ImageFormat::Jpeg);
        let _ = stage.save_with_format(lowres_dir.join(&stage_name), image::ImageFormat::Jpeg);

        let new_path       = format!("/uploads/{folder}/{orig_name}");
        let new_thumb_path = format!("/uploads/{folder}/Lowres_Assets/{thumb_name}");
        let new_stage_path = format!("/uploads/{folder}/Lowres_Assets/{stage_name}");

        // Move old files to bin (ignore errors — file may already be in bin or missing).
        let bin_stem = src_file.file_stem().and_then(|s| s.to_str()).unwrap_or("old").to_string();
        let old_dir  = src_file.parent().unwrap_or(&st.uploads_dir);
        let old_lowres = old_dir.join("Lowres_Assets");
        let _ = std::fs::rename(&src_file, bin_dir.join(format!("{bin_stem}.jpg")) );
        if let Some(ref tp) = old_thumb {
            let tr = tp.trim_start_matches('/').trim_start_matches("uploads/");
            let _ = std::fs::rename(st.uploads_dir.join(tr), bin_dir.join(format!("{bin_stem}_200w.jpg")) );
        }
        if let Some(ref sp) = old_stage {
            let sr = sp.trim_start_matches('/').trim_start_matches("uploads/");
            let _ = std::fs::rename(st.uploads_dir.join(sr), bin_dir.join(format!("{bin_stem}_1000w.jpg")));
        }
        // Suppress the unused warning — these are used in the rename calls above.
        let _ = &old_lowres;

        // Update the images row.
        sqlx::query(
            "UPDATE images SET path = ?, thumb_path = ?, stage_path = ?, car_id = ?, livery_id = ? WHERE id = ?"
        )
        .bind(&new_path)
        .bind(&new_thumb_path)
        .bind(&new_stage_path)
        .bind(&car_id)
        .bind(livery_id)
        .bind(img_id)
        .execute(&st.pool)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;

        results.push(json!({
            "id":        img_id,
            "path":      new_path,
            "thumbPath": new_thumb_path,
            "stagePath": new_stage_path,
            "carId":     car_id,
            "liveryId":  livery_id,
        }));
    }

    Ok(Json(json!({ "migrated": results })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::test_pool;

    // ── slugify / card_folder ───────────────────────────────────────────────

    #[test]
    fn slugify_collapses_and_trims() {
        assert_eq!(slugify("Smokin' McLaren P1 GTR!"), "smokin_mclaren_p1_gtr");
        assert_eq!(slugify("  --already--slugged--  "), "already_slugged");
        assert_eq!(slugify("UPPER case 123"), "upper_case_123");
        assert_eq!(slugify(""), "");
        assert_eq!(slugify("!!!"), "");
    }

    #[test]
    fn card_folder_covers_all_fallbacks() {
        assert_eq!(card_folder("Smokin", "abc123"), "smokin_abc123");
        assert_eq!(card_folder("Smokin", ""), "smokin");
        assert_eq!(card_folder("", "abc123"), "card_abc123");
        assert_eq!(card_folder("!!!", ""), "misc");
    }

    // ── sync_card_images branch coverage ────────────────────────────────────

    async fn insert_image(pool: &SqlitePool, card_id: &str, path: &str) -> i64 {
        sqlx::query(
            "INSERT INTO images (card_id, path, thumb_path, stage_path, sort_order) VALUES (?, ?, ?, ?, 0)",
        )
        .bind(card_id).bind(path)
        .bind(format!("{path}.thumb")).bind(format!("{path}.stage"))
        .execute(pool).await.unwrap().last_insert_rowid()
    }

    #[tokio::test]
    async fn numeric_id_updates_metadata_and_strips_paths() {
        let pool = test_pool().await;
        let id = insert_image(&pool, "7", "/uploads/x/001.jpg").await;

        let mut body = json!({ "id": "7", "images": [
            { "id": id, "path": "/uploads/x/001.jpg", "alt": "new alt", "order": 4,
              "carId": "fh5-a", "included": false }
        ]});
        sync_card_images(&pool, "7", &mut body).await.unwrap();

        // DB row updated.
        let row = sqlx::query("SELECT alt_text, sort_order, car_id, included FROM images WHERE id = ?")
            .bind(id).fetch_one(&pool).await.unwrap();
        assert_eq!(row.get::<String, _>("alt_text"), "new alt");
        assert_eq!(row.get::<i64, _>("sort_order"), 4);
        assert_eq!(row.get::<Option<String>, _>("car_id").as_deref(), Some("fh5-a"));
        assert_eq!(row.get::<i64, _>("included"), 0);

        // Body stripped to id + meta only.
        let img = &body["images"][0];
        assert_eq!(img["id"], id);
        assert!(img.get("path").is_none());
        assert!(img.get("thumbPath").is_none());
    }

    #[tokio::test]
    async fn path_only_image_inserts_new_row() {
        let pool = test_pool().await;
        let mut body = json!({ "id": "7", "images": [
            { "path": "/uploads/x/new.jpg", "thumbPath": "/uploads/x/new_t.jpg",
              "stagePath": "/uploads/x/new_s.jpg", "order": 0, "alt": "fresh" }
        ]});
        sync_card_images(&pool, "7", &mut body).await.unwrap();

        let img = &body["images"][0];
        assert!(img["id"].is_i64(), "body gained the new DB id");
        let path: String = sqlx::query_scalar("SELECT path FROM images WHERE id = ?")
            .bind(img["id"].as_i64().unwrap())
            .fetch_one(&pool).await.unwrap();
        assert_eq!(path, "/uploads/x/new.jpg");
    }

    #[tokio::test]
    async fn path_only_image_finds_existing_row_instead_of_duplicating() {
        let pool = test_pool().await;
        let id = insert_image(&pool, "7", "/uploads/x/001.jpg").await;

        let mut body = json!({ "id": "7", "images": [
            { "path": "/uploads/x/001.jpg", "order": 2, "alt": "updated" }
        ]});
        sync_card_images(&pool, "7", &mut body).await.unwrap();

        assert_eq!(body["images"][0]["id"], id, "matched by path, no new row");
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM images WHERE card_id = '7'")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(count, 1);
        let sort: i64 = sqlx::query_scalar("SELECT sort_order FROM images WHERE id = ?")
            .bind(id).fetch_one(&pool).await.unwrap();
        assert_eq!(sort, 2);
    }

    #[tokio::test]
    async fn image_with_neither_id_nor_path_is_dropped() {
        let pool = test_pool().await;
        let mut body = json!({ "id": "7", "images": [ { "alt": "ghost", "order": 0 } ]});
        sync_card_images(&pool, "7", &mut body).await.unwrap();
        assert_eq!(body["images"].as_array().unwrap().len(), 0);
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM images")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn absent_livery_and_included_are_preserved_via_coalesce() {
        let pool = test_pool().await;
        let id = insert_image(&pool, "7", "/uploads/x/001.jpg").await;
        // Real parent rows — livery_id has a FK to liveries (which FKs cars).
        sqlx::query("INSERT INTO cars (id, game, make, model) VALUES ('fh5-a', 'FH5', 'Make', 'Model')")
            .execute(&pool).await.unwrap();
        let livery_id = sqlx::query(
            "INSERT INTO liveries (car_id, serial, name) VALUES ('fh5-a', 'FH5-X-L001', 'Test Livery')",
        )
        .execute(&pool).await.unwrap().last_insert_rowid();
        sqlx::query("UPDATE images SET livery_id = ?, included = 0 WHERE id = ?")
            .bind(livery_id).bind(id).execute(&pool).await.unwrap();

        // Body omits liveryId and included — the DB values must survive the sync.
        let mut body = json!({ "id": "7", "images": [ { "id": id, "order": 0 } ]});
        sync_card_images(&pool, "7", &mut body).await.unwrap();

        let row = sqlx::query("SELECT livery_id, included FROM images WHERE id = ?")
            .bind(id).fetch_one(&pool).await.unwrap();
        assert_eq!(row.get::<Option<i64>, _>("livery_id"), Some(livery_id));
        assert_eq!(row.get::<i64, _>("included"), 0);
    }
}
