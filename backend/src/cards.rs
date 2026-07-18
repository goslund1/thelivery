//! Cards: CRUD + history, seeding, startup body normalization/migration,
//! seed export/reload, and soft-deleted card administration.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde_json::{json, Value};
use sqlx::{Row, SqlitePool};

use crate::auth::{AdminUser, AuthUser};
use crate::images::{fetch_all_images_grouped, inject_images, sync_card_images};
use crate::state::{err, ApiError, AppState};

// --- Seed -------------------------------------------------------------------

/// Import seed/cards.json on first run (when the table is empty).
pub async fn seed_if_empty(pool: &SqlitePool, seed_path: &str) -> anyhow::Result<()> {
    let count: i64 = sqlx::query("SELECT COUNT(*) AS c FROM cards")
        .fetch_one(pool)
        .await?
        .get("c");
    if count > 0 {
        tracing::info!("db already has {count} cards; skipping seed");
        return Ok(());
    }
    let Ok(raw) = std::fs::read_to_string(seed_path) else {
        tracing::warn!("no seed file at {seed_path}; starting empty");
        return Ok(());
    };
    let cards: Vec<Value> = serde_json::from_str(&raw)?;
    for c in &cards {
        // A top-level `deletedAt` marks a soft-deleted (purgatoried) card: seed
        // it, then restore its deleted_at so it lands in the admin trash rather
        // than the live catalog. The marker is seed-file-only — strip it from
        // the stored body.
        let mut body = c.clone();
        let deleted_at = body
            .as_object_mut()
            .and_then(|o| o.remove("deletedAt"))
            .and_then(|v| v.as_str().map(String::from));
        upsert(pool, &body).await?;
        if let Some(ts) = deleted_at {
            let id = body.get("id").and_then(Value::as_str).unwrap_or_default();
            sqlx::query("UPDATE cards SET deleted_at = ? WHERE id = ?")
                .bind(&ts)
                .bind(id)
                .execute(pool)
                .await?;
        }
    }
    tracing::info!("seeded {} cards from {seed_path}", cards.len());
    Ok(())
}

/// Insert-or-replace a card row from its JSON object.
pub async fn upsert(pool: &SqlitePool, body: &Value) -> Result<(), sqlx::Error> {
    let id = body.get("id").and_then(Value::as_str).unwrap_or_default().to_string();
    let catalog_number = body.get("catalogNumber").and_then(Value::as_i64).unwrap_or(0);
    let body_str = body.to_string();
    sqlx::query(
        "INSERT INTO cards (id, catalog_number, body) VALUES (?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET catalog_number = excluded.catalog_number, body = excluded.body, deleted_at = NULL",
    )
    .bind(&id)
    .bind(catalog_number)
    .bind(&body_str)
    .execute(pool)
    .await?;
    Ok(())
}

/// Migrate any rows still in the old shape (top-level inspiration/designNotes/
/// recipe fields, per-image `isLead`) into the new card shape: a generic ordered
/// `sections` array, with the lead image moved to `order` 0 and `isLead` dropped.
/// Idempotent — rows that already carry `sections` are left untouched.

pub async fn normalize_bodies(pool: &SqlitePool) -> anyhow::Result<()> {
    let rows = sqlx::query("SELECT body FROM cards").fetch_all(pool).await?;
    let mut migrated = 0;
    for row in &rows {
        let raw: String = row.get("body");
        let Ok(mut v) = serde_json::from_str::<Value>(&raw) else {
            continue;
        };
        let mut changed = false;
        // Step 1: migrate old-shape cards (no sections key at all).
        if v.get("sections").is_none() {
            normalize_card(&mut v);
            changed = true;
        }
        // Step 2: ensure all 3 standard sections exist (handles sections:[]).
        if ensure_standard_sections(&mut v) {
            changed = true;
        }
        // Step 3: sync body images to images table, strip paths from body.
        let card_id = v.get("id").and_then(Value::as_str).map(String::from);
        if let Some(card_id) = card_id {
            let has_path_images = v.get("images")
                .and_then(Value::as_array)
                .map(|arr| arr.iter().any(|img| img.get("path").is_some()))
                .unwrap_or(false);
            if has_path_images {
                match sync_card_images(pool, &card_id, &mut v).await {
                    Ok(()) => changed = true,
                    // Leave the body un-stripped so paths survive for a retry
                    // on the next boot, but say so — a silent skip here once
                    // hid a broken FK for an entire reseed.
                    Err(e) => tracing::warn!("image sync failed for card {card_id}: {e}"),
                }
            }
        }
        // Step 4: migrate variants[] → cars[].tunes[] in any forza_recipe section.
        if migrate_variants_to_cars(&mut v) {
            changed = true;
        }
        if changed {
            // Deliberately not upsert(): its ON CONFLICT clause clears
            // deleted_at (save = undelete), which would resurrect purgatoried
            // cards during a startup migration pass.
            sqlx::query("UPDATE cards SET catalog_number = ?, body = ? WHERE id = ?")
                .bind(v.get("catalogNumber").and_then(Value::as_i64).unwrap_or(0))
                .bind(v.to_string())
                .bind(v.get("id").and_then(Value::as_str).unwrap_or_default())
                .execute(pool)
                .await?;
            migrated += 1;
        }
    }
    if migrated > 0 {
        tracing::info!("normalized {migrated} card(s) to the sections shape");
    }
    Ok(())
}

pub fn ensure_standard_sections(v: &mut Value) -> bool {
    let Some(obj) = v.as_object_mut() else { return false };
    let arr = match obj.entry("sections").or_insert_with(|| json!([])).as_array_mut() {
        Some(a) => a,
        None => return false,
    };
    let existing: std::collections::HashSet<String> = arr.iter()
        .filter_map(|s| s.get("key").and_then(Value::as_str).map(String::from))
        .collect();
    let mut added = false;
    if !existing.contains("inspiration") {
        arr.push(json!({ "type": "text", "key": "inspiration", "label": "Inspiration", "body": "" }));
        added = true;
    }
    if !existing.contains("notes") {
        arr.push(json!({ "type": "text", "key": "notes", "label": "Design Notes", "body": "" }));
        added = true;
    }
    if !existing.contains("recipe") {
        arr.push(json!({ "type": "forza_recipe", "key": "recipe", "label": "Tune / Build Parts",
            "tuneName": "", "shareCode": "", "coreSpecs": {}, "upgrades": [], "adjustments": [] }));
        added = true;
    }
    added
}

// Reshape any forza_recipe section that still uses the old flat variants[] array
// into the new cars[].tunes[] hierarchy. Idempotent — skips sections that already
// have cars[] or have no variants[].
pub fn migrate_variants_to_cars(v: &mut Value) -> bool {
    let sections = match v.get_mut("sections").and_then(Value::as_array_mut) {
        Some(s) => s,
        None => return false,
    };
    let mut changed = false;
    for section in sections.iter_mut() {
        let is_recipe = section.get("type").and_then(Value::as_str) == Some("forza_recipe");
        if !is_recipe { continue; }
        // Skip if already migrated (has cars key) or nothing to migrate.
        if section.get("cars").is_some() { continue; }
        let variants = match section.get("variants").and_then(Value::as_array) {
            Some(arr) if !arr.is_empty() => arr.clone(),
            _ => continue,
        };
        // Each variant becomes a CardCar with one CardTune inside.
        let cars: Vec<Value> = variants.iter().map(|var| {
            json!({
                "carId":     var.get("carId").cloned().unwrap_or(json!("")),
                "carName":   var.get("carName").cloned(),
                "liveryId":  var.get("liveryId").cloned(),
                "liveryName":var.get("liveryName").cloned(),
                "tunes": [{
                    "tuneName":      var.get("tuneName").cloned().unwrap_or(json!("")),
                    "tuneType":      var.get("tuneType").cloned(),
                    "shareCode":     var.get("shareCode").cloned().unwrap_or(json!("")),
                    "coreSpecs":     var.get("coreSpecs").cloned().unwrap_or(json!({})),
                    "upgrades":      var.get("upgrades").cloned().unwrap_or(json!([])),
                    "adjustments":   var.get("adjustments").cloned().unwrap_or(json!([])),
                    "isSuggested":   var.get("isSuggested").cloned(),
                    "pendingPresetId": var.get("pendingPresetId").cloned(),
                }]
            })
        }).collect();
        if let Some(obj) = section.as_object_mut() {
            obj.insert("cars".to_string(), json!(cars));
            obj.remove("variants");
            changed = true;
        }
    }
    changed
}

pub fn text_section(key: &str, label: &str, src: &Value) -> Value {
    let mut o = serde_json::Map::new();
    o.insert("type".into(), json!("text"));
    o.insert("key".into(), json!(key));
    o.insert("label".into(), json!(label));
    o.insert("body".into(), src.get("body").cloned().unwrap_or_else(|| json!("")));
    if let Some(fp) = src.get("figurePath") {
        o.insert("figurePath".into(), fp.clone());
    }
    Value::Object(o)
}

pub fn normalize_card(v: &mut Value) {
    let Some(obj) = v.as_object_mut() else { return };

    // Old top-level fields -> ordered sections[].
    let mut sections = Vec::new();
    if let Some(insp) = obj.remove("inspiration") {
        sections.push(text_section("inspiration", "Inspiration", &insp));
    }
    if let Some(notes) = obj.remove("designNotes") {
        sections.push(text_section("notes", "Design Notes", &notes));
    }
    if let Some(mut recipe) = obj.remove("recipe") {
        if let Some(r) = recipe.as_object_mut() {
            r.insert("type".into(), json!("forza_recipe"));
            r.insert("key".into(), json!("recipe"));
            r.insert("label".into(), json!("Tune / Build Parts"));
        }
        sections.push(recipe);
    }
    obj.insert("sections".into(), Value::Array(sections));

    // Drop isLead; keep the lead image at order 0, then renumber.
    if let Some(Value::Array(imgs)) = obj.get_mut("images") {
        let lead_id = imgs
            .iter()
            .find(|im| im.get("isLead").and_then(Value::as_bool).unwrap_or(false))
            .and_then(|im| im.get("id"))
            .and_then(Value::as_str)
            .map(String::from);
        for im in imgs.iter_mut() {
            if let Some(o) = im.as_object_mut() {
                o.remove("isLead");
            }
        }
        imgs.sort_by_key(|im| im.get("order").and_then(Value::as_i64).unwrap_or(0));
        if let Some(lid) = lead_id {
            if let Some(pos) = imgs
                .iter()
                .position(|im| im.get("id").and_then(Value::as_str) == Some(lid.as_str()))
            {
                if pos != 0 {
                    let it = imgs.remove(pos);
                    imgs.insert(0, it);
                }
            }
        }
        for (i, im) in imgs.iter_mut().enumerate() {
            if let Some(o) = im.as_object_mut() {
                o.insert("order".into(), json!(i));
            }
        }
    }
}

pub async fn admin_reload_seed(
    _admin: AdminUser,
    State(st): State<AppState>,
) -> Result<Json<Value>, ApiError> {
    let raw = std::fs::read_to_string(&st.seed_path)
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, format!("read seed failed: {e}")))?;
    let cards: Vec<Value> = serde_json::from_str(&raw)
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, format!("parse seed failed: {e}")))?;

    let seed_ids: std::collections::HashSet<String> = cards.iter()
        .filter_map(|c| c.get("id").and_then(Value::as_str).map(String::from))
        .collect();

    // Upsert every card in the seed file.
    for card in &cards {
        upsert(&st.pool, card).await
            .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    }

    // Delete any cards in the DB not present in the seed.
    let db_ids: Vec<String> = sqlx::query_scalar("SELECT id FROM cards")
        .fetch_all(&st.pool).await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let mut removed = 0u64;
    for id in db_ids {
        if !seed_ids.contains(&id) {
            sqlx::query("DELETE FROM cards WHERE id = ?")
                .bind(&id)
                .execute(&st.pool).await
                .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
            removed += 1;
        }
    }

    Ok(Json(json!({ "upserted": cards.len(), "removed": removed })))
}

pub async fn admin_export_seed(
    _admin: AdminUser,
    State(st): State<AppState>,
) -> Result<Json<Value>, ApiError> {
    let rows = sqlx::query("SELECT body FROM cards WHERE deleted_at IS NULL ORDER BY catalog_number")
        .fetch_all(&st.pool).await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let cards: Vec<Value> = rows.iter()
        .filter_map(|r| serde_json::from_str::<Value>(r.get::<String, _>("body").as_str()).ok())
        .collect();
    let count = cards.len();
    let json_str = serde_json::to_string_pretty(&cards)
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    std::fs::write(&st.seed_path, json_str)
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, format!("write failed: {e}")))?;
    Ok(Json(json!({ "exported": count })))
}

/// Light shape gate for card bodies on PUT/POST. The body stays schema-free
/// JSON by design (new section types need no backend change), so this only
/// rejects payloads that would corrupt the catalog: wrong-typed identity
/// fields, a malformed sections array, or non-array images/tags/collections.
pub fn validate_card_body(body: &Value) -> Result<(), String> {
    let Some(obj) = body.as_object() else {
        return Err("card body must be a JSON object".into());
    };
    match obj.get("name") {
        Some(v) if v.is_string() => {}
        _ => return Err("name must be a string".into()),
    }
    match obj.get("catalogNumber") {
        Some(v) if v.is_i64() || v.is_u64() => {}
        _ => return Err("catalogNumber must be an integer".into()),
    }
    let Some(sections) = obj.get("sections").and_then(Value::as_array) else {
        return Err("sections must be an array".into());
    };
    for s in sections {
        let ok = s.get("type").map(Value::is_string).unwrap_or(false)
            && s.get("key").map(Value::is_string).unwrap_or(false);
        if !ok {
            return Err("every section needs string `type` and `key` fields".into());
        }
    }
    for field in ["images", "tags", "collections"] {
        if let Some(v) = obj.get(field) {
            if !v.is_array() {
                return Err(format!("{field} must be an array"));
            }
        }
    }
    Ok(())
}

pub async fn list_cards(State(st): State<AppState>) -> Result<Json<Vec<Value>>, ApiError> {
    let rows = sqlx::query("SELECT body FROM cards WHERE deleted_at IS NULL ORDER BY catalog_number")
        .fetch_all(&st.pool)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    // One query for all images, grouped by card — avoids a per-card query.
    // Same fallback semantics as inject_images: cards with no DB image rows
    // keep their body images as-is (unmigrated card).
    let images_by_card = fetch_all_images_grouped(&st.pool).await;
    let mut out = Vec::new();
    for row in &rows {
        if let Ok(mut v) = serde_json::from_str::<Value>(row.get::<String, _>("body").as_str()) {
            if let Some(imgs) = v.get("id").and_then(Value::as_str).and_then(|id| images_by_card.get(id)) {
                if !imgs.is_empty() {
                    v["images"] = json!(imgs);
                }
            }
            out.push(v);
        }
    }
    Ok(Json(out))
}

pub async fn get_card(
    State(st): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let row = sqlx::query("SELECT body FROM cards WHERE id = ?")
        .bind(&id)
        .fetch_optional(&st.pool)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "card not found"))?;
    let mut body: Value = serde_json::from_str(row.get::<String, _>("body").as_str())
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    inject_images(&st.pool, &mut body).await;
    Ok(Json(body))
}

pub async fn put_card(
    State(st): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(mut body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    body["id"] = json!(id);
    validate_card_body(&body).map_err(|e| err(StatusCode::UNPROCESSABLE_ENTITY, e))?;

    // Snapshot the current state into history before overwriting.
    let existing: Option<String> = sqlx::query_scalar("SELECT body FROM cards WHERE id = ?")
        .bind(&id)
        .fetch_optional(&st.pool)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let mut snapshot_version: Option<i64> = None;
    if let Some(current_body) = existing {
        let next_version: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(version), 0) + 1 FROM card_history WHERE card_id = ?"
        )
        .bind(&id)
        .fetch_one(&st.pool)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;

        sqlx::query(
            "INSERT INTO card_history (card_id, version, body) VALUES (?, ?, ?)"
        )
        .bind(&id)
        .bind(next_version)
        .bind(&current_body)
        .execute(&st.pool)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
        snapshot_version = Some(next_version);

        // Prune to 20 most recent versions.
        sqlx::query(
            "DELETE FROM card_history WHERE card_id = ? AND version <= (
                SELECT version FROM card_history WHERE card_id = ?
                ORDER BY version DESC LIMIT 1 OFFSET 19
            )"
        )
        .bind(&id)
        .bind(&id)
        .execute(&st.pool)
        .await
        .ok();
    }

    sync_card_images(&st.pool, &id, &mut body)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    upsert(&st.pool, &body)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    inject_images(&st.pool, &mut body).await;
    // prevVersion points at the card_history snapshot taken above — that's the
    // state to restore to reverse this edit.
    crate::audit::record(
        &st.pool, &auth.username, "card.update", "card", Some(&id),
        snapshot_version.map(|v| json!({ "prevVersion": v })),
    ).await;
    Ok(Json(body))
}

pub async fn list_card_history(
    State(st): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let rows = sqlx::query(
        "SELECT version, saved_at FROM card_history WHERE card_id = ? ORDER BY version DESC"
    )
    .bind(&id)
    .fetch_all(&st.pool)
    .await
    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let versions: Vec<Value> = rows.iter().map(|r| json!({
        "version": r.get::<i64, _>("version"),
        "savedAt": r.get::<String, _>("saved_at"),
    })).collect();

    Ok(Json(json!(versions)))
}

pub async fn get_card_history_version(
    State(st): State<AppState>,
    Path((id, version)): Path<(String, i64)>,
) -> Result<Json<Value>, ApiError> {
    let row = sqlx::query(
        "SELECT body, saved_at FROM card_history WHERE card_id = ? AND version = ?"
    )
    .bind(&id)
    .bind(version)
    .fetch_optional(&st.pool)
    .await
    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?
    .ok_or_else(|| err(StatusCode::NOT_FOUND, "version not found"))?;

    let body: Value = serde_json::from_str(row.get::<String, _>("body").as_str())
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    Ok(Json(json!({ "version": version, "savedAt": row.get::<String, _>("saved_at"), "body": body })))
}

pub async fn create_card(
    State(st): State<AppState>,
    auth: AuthUser,
    Json(mut body): Json<Value>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let card_id = body.get("id").and_then(Value::as_str).unwrap_or_default().to_string();
    if card_id.is_empty() {
        return Err(err(StatusCode::BAD_REQUEST, "body.id is required"));
    }
    validate_card_body(&body).map_err(|e| err(StatusCode::UNPROCESSABLE_ENTITY, e))?;
    sync_card_images(&st.pool, &card_id, &mut body)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    upsert(&st.pool, &body)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    inject_images(&st.pool, &mut body).await;
    crate::audit::record(&st.pool, &auth.username, "card.create", "card", Some(&card_id), None).await;
    Ok((StatusCode::CREATED, Json(body)))
}

pub async fn delete_card(
    State(st): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    sqlx::query("UPDATE cards SET deleted_at = datetime('now') WHERE id = ?")
        .bind(&id)
        .execute(&st.pool)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    // Soft delete — reversible from the Admin tab's purgatory (deleted-cards restore).
    crate::audit::record(&st.pool, &auth.username, "card.delete", "card", Some(&id), None).await;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn admin_list_deleted_cards(
    _admin: AdminUser,
    State(st): State<AppState>,
) -> Result<Json<Value>, ApiError> {
    let rows = sqlx::query(
        "SELECT id, body, deleted_at FROM cards WHERE deleted_at IS NOT NULL ORDER BY deleted_at DESC"
    )
    .fetch_all(&st.pool)
    .await
    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let entries: Vec<Value> = rows.iter().map(|r| {
        let body: String = r.get("body");
        let deleted_at: String = r.get("deleted_at");
        let name = serde_json::from_str::<Value>(&body)
            .ok()
            .and_then(|v| v.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()))
            .unwrap_or_default();
        json!({ "id": r.get::<String, _>("id"), "name": name, "deletedAt": deleted_at })
    }).collect();

    Ok(Json(json!({ "cards": entries })))
}

pub async fn admin_restore_card(
    _admin: AdminUser,
    State(st): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    sqlx::query("UPDATE cards SET deleted_at = NULL WHERE id = ?")
        .bind(&id)
        .execute(&st.pool)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(json!({ "ok": true })))
}

pub async fn admin_purge_card(
    _admin: AdminUser,
    State(st): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    sqlx::query("DELETE FROM cards WHERE id = ? AND deleted_at IS NOT NULL")
        .bind(&id)
        .execute(&st.pool)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_card() -> Value {
        json!({
            "id": "7",
            "catalogNumber": 7,
            "name": "Test Card",
            "subtitle": "",
            "isFavorite": false,
            "isLegend": false,
            "collections": ["FH5"],
            "tags": [],
            "images": [],
            "sections": [
                { "type": "text", "key": "inspiration", "label": "Inspiration", "body": "" },
                { "type": "forza_recipe", "key": "recipe", "label": "Tune / Build Parts",
                  "tuneName": "", "shareCode": "", "coreSpecs": {}, "upgrades": [], "adjustments": [] }
            ]
        })
    }

    #[test]
    fn accepts_a_well_formed_card() {
        assert!(validate_card_body(&valid_card()).is_ok());
    }

    #[test]
    fn accepts_unknown_extra_fields() {
        // Schema-free by design: future fields must pass through untouched.
        let mut c = valid_card();
        c["shareOverlayConfig"] = json!({ "photoId": 3, "textBoxes": [] });
        c["someFutureField"] = json!({ "anything": true });
        assert!(validate_card_body(&c).is_ok());
    }

    #[test]
    fn accepts_unknown_section_types() {
        // New section types need no backend change — only type/key shape is checked.
        let mut c = valid_card();
        c["sections"].as_array_mut().unwrap().push(json!({
            "type": "video", "key": "clips", "url": "/uploads/x.mp4"
        }));
        assert!(validate_card_body(&c).is_ok());
    }

    #[test]
    fn rejects_non_object_body() {
        assert!(validate_card_body(&json!([1, 2, 3])).is_err());
        assert!(validate_card_body(&json!("card")).is_err());
    }

    #[test]
    fn rejects_wrong_typed_identity_fields() {
        let mut c = valid_card();
        c["name"] = json!(42);
        assert!(validate_card_body(&c).is_err());

        let mut c = valid_card();
        c["catalogNumber"] = json!("seven");
        assert!(validate_card_body(&c).is_err());

        let mut c = valid_card();
        c.as_object_mut().unwrap().remove("name");
        assert!(validate_card_body(&c).is_err());
    }

    #[test]
    fn rejects_malformed_sections() {
        let mut c = valid_card();
        c["sections"] = json!("not an array");
        assert!(validate_card_body(&c).is_err());

        let mut c = valid_card();
        c["sections"].as_array_mut().unwrap().push(json!({ "label": "no type or key" }));
        assert!(validate_card_body(&c).is_err());
    }

    #[test]
    fn rejects_non_array_collection_fields() {
        for field in ["images", "tags", "collections"] {
            let mut c = valid_card();
            c[field] = json!({});
            assert!(validate_card_body(&c).is_err(), "{field} should require an array");
        }
    }

    // ── normalize_card (oldest shape → sections[]) ──────────────────────────

    /// A card in the original pre-sections shape: top-level inspiration /
    /// designNotes / recipe, string image ids, isLead flag.
    fn old_shape_card() -> Value {
        json!({
            "id": "3",
            "catalogNumber": 3,
            "name": "Old Shape",
            "inspiration": { "body": "the muse", "figurePath": "/uploads/fig.jpg" },
            "designNotes": { "body": "the notes" },
            "recipe": { "tuneName": "T", "shareCode": "123 456 789",
                        "coreSpecs": {}, "upgrades": [], "adjustments": [] },
            "images": [
                { "id": "a", "path": "/uploads/a.jpg", "order": 0 },
                { "id": "b", "path": "/uploads/b.jpg", "order": 1, "isLead": true },
                { "id": "c", "path": "/uploads/c.jpg", "order": 2 }
            ]
        })
    }

    #[test]
    fn normalize_card_builds_ordered_sections() {
        let mut c = old_shape_card();
        normalize_card(&mut c);

        assert!(c.get("inspiration").is_none(), "old top-level fields removed");
        assert!(c.get("designNotes").is_none());
        assert!(c.get("recipe").is_none());

        let sections = c["sections"].as_array().unwrap();
        let keys: Vec<&str> = sections.iter().map(|s| s["key"].as_str().unwrap()).collect();
        assert_eq!(keys, ["inspiration", "notes", "recipe"]);
        assert_eq!(sections[0]["type"], "text");
        assert_eq!(sections[0]["body"], "the muse");
        assert_eq!(sections[0]["figurePath"], "/uploads/fig.jpg");
        assert_eq!(sections[2]["type"], "forza_recipe");
        assert_eq!(sections[2]["shareCode"], "123 456 789");
    }

    #[test]
    fn normalize_card_moves_lead_image_to_order_zero() {
        let mut c = old_shape_card();
        normalize_card(&mut c);

        let imgs = c["images"].as_array().unwrap();
        let ids: Vec<&str> = imgs.iter().map(|i| i["id"].as_str().unwrap()).collect();
        assert_eq!(ids, ["b", "a", "c"], "lead image first, rest keep relative order");
        for (i, img) in imgs.iter().enumerate() {
            assert_eq!(img["order"], i as i64, "orders renumbered contiguously");
            assert!(img.get("isLead").is_none(), "isLead dropped");
        }
    }

    #[test]
    fn normalize_card_without_recipe_yields_two_sections() {
        let mut c = old_shape_card();
        c.as_object_mut().unwrap().remove("recipe");
        normalize_card(&mut c);
        assert_eq!(c["sections"].as_array().unwrap().len(), 2);
    }

    // ── ensure_standard_sections ────────────────────────────────────────────

    #[test]
    fn ensure_sections_fills_empty_array() {
        let mut c = json!({ "id": "1", "sections": [] });
        assert!(ensure_standard_sections(&mut c));
        let keys: Vec<&str> = c["sections"].as_array().unwrap()
            .iter().map(|s| s["key"].as_str().unwrap()).collect();
        assert_eq!(keys, ["inspiration", "notes", "recipe"]);
    }

    #[test]
    fn ensure_sections_is_idempotent() {
        let mut c = json!({ "id": "1", "sections": [] });
        ensure_standard_sections(&mut c);
        let after_first = c.clone();
        assert!(!ensure_standard_sections(&mut c), "second run reports no change");
        assert_eq!(c, after_first, "second run changes nothing");
    }

    #[test]
    fn ensure_sections_adds_only_missing() {
        let mut c = json!({ "id": "1", "sections": [
            { "type": "text", "key": "notes", "label": "Design Notes", "body": "kept" }
        ]});
        assert!(ensure_standard_sections(&mut c));
        let sections = c["sections"].as_array().unwrap();
        assert_eq!(sections.len(), 3);
        assert_eq!(sections[0]["body"], "kept", "existing section untouched and first");
    }

    // ── migrate_variants_to_cars ────────────────────────────────────────────

    fn card_with_variants() -> Value {
        json!({ "id": "5", "sections": [
            { "type": "text", "key": "inspiration", "label": "Inspiration", "body": "" },
            { "type": "forza_recipe", "key": "recipe", "label": "Tune / Build Parts",
              "tuneName": "top", "shareCode": "111", "coreSpecs": {}, "upgrades": [], "adjustments": [],
              "variants": [
                  { "carId": "fh5-a", "carName": "Car A", "tuneName": "A tune", "shareCode": "222",
                    "coreSpecs": { "Engine": "V8" }, "upgrades": [], "adjustments": [], "isSuggested": true },
                  { "carId": "fh5-b", "tuneName": "B tune", "shareCode": "",
                    "coreSpecs": {}, "upgrades": [], "adjustments": [] }
              ] }
        ]})
    }

    #[test]
    fn variants_become_cars_with_one_tune_each() {
        let mut c = card_with_variants();
        assert!(migrate_variants_to_cars(&mut c));

        let recipe = &c["sections"][1];
        assert!(recipe.get("variants").is_none(), "variants removed");
        let cars = recipe["cars"].as_array().unwrap();
        assert_eq!(cars.len(), 2);
        assert_eq!(cars[0]["carId"], "fh5-a");
        assert_eq!(cars[0]["carName"], "Car A");
        let tune = &cars[0]["tunes"][0];
        assert_eq!(tune["tuneName"], "A tune");
        assert_eq!(tune["shareCode"], "222");
        assert_eq!(tune["coreSpecs"]["Engine"], "V8");
        assert_eq!(tune["isSuggested"], true);
        assert_eq!(cars[1]["tunes"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn variants_migration_is_idempotent() {
        let mut c = card_with_variants();
        migrate_variants_to_cars(&mut c);
        let after_first = c.clone();
        assert!(!migrate_variants_to_cars(&mut c), "second run reports no change");
        assert_eq!(c, after_first);
    }

    #[test]
    fn variants_migration_skips_sections_with_cars_or_without_variants() {
        // Already migrated: cars[] present → untouched even if variants also present.
        let mut c = json!({ "id": "1", "sections": [
            { "type": "forza_recipe", "key": "recipe", "cars": [{ "carId": "x", "tunes": [] }],
              "variants": [{ "carId": "y" }] }
        ]});
        assert!(!migrate_variants_to_cars(&mut c));
        assert!(c["sections"][0].get("variants").is_some(), "left as-is when cars[] exists");

        // Plain card: no variants anywhere.
        let mut c = json!({ "id": "2", "sections": [
            { "type": "forza_recipe", "key": "recipe", "tuneName": "" }
        ]});
        assert!(!migrate_variants_to_cars(&mut c));
    }

    // ── normalize_bodies (end-to-end against in-memory DB) ──────────────────

    #[tokio::test]
    async fn normalize_bodies_migrates_and_is_idempotent() {
        let pool = crate::testutil::test_pool().await;

        // Seed one old-shape row (no sections, top-level fields, isLead).
        let old = old_shape_card();
        sqlx::query("INSERT INTO cards (id, catalog_number, body) VALUES (?, ?, ?)")
            .bind("3").bind(3i64).bind(old.to_string())
            .execute(&pool).await.unwrap();

        normalize_bodies(&pool).await.unwrap();

        let body: String = sqlx::query_scalar("SELECT body FROM cards WHERE id = '3'")
            .fetch_one(&pool).await.unwrap();
        let v: Value = serde_json::from_str(&body).unwrap();
        let keys: Vec<&str> = v["sections"].as_array().unwrap()
            .iter().map(|s| s["key"].as_str().unwrap()).collect();
        assert_eq!(keys, ["inspiration", "notes", "recipe"]);
        // Step 3 synced body images into the images table: the body now holds
        // integer DB ids (paths stripped), and the lead (b.jpg) sits at sort 0.
        assert!(v["images"][0]["id"].is_i64(), "string ids replaced by DB PKs");
        assert!(v["images"][0].get("path").is_none(), "paths stripped from body");
        let lead_path: String = sqlx::query_scalar(
            "SELECT path FROM images WHERE card_id = '3' AND sort_order = 0",
        )
        .fetch_one(&pool).await.unwrap();
        assert_eq!(lead_path, "/uploads/b.jpg", "lead image moved to front");

        // Second run must be a no-op: stored body byte-identical.
        normalize_bodies(&pool).await.unwrap();
        let body2: String = sqlx::query_scalar("SELECT body FROM cards WHERE id = '3'")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(body, body2, "normalize_bodies is idempotent");
    }

    // ── seed_if_empty: deletedAt marker ─────────────────────────────────────

    #[tokio::test]
    async fn seed_honors_deleted_at_marker() {
        let pool = crate::testutil::test_pool().await;

        let mut purgatoried = valid_card();
        purgatoried["id"] = json!("8");
        purgatoried["catalogNumber"] = json!(8);
        purgatoried["deletedAt"] = json!("2026-07-17 12:55:29");
        // Path image: forces normalize_bodies to sync + rewrite this card, the
        // path that once resurrected purgatoried cards via upsert().
        purgatoried["images"] = json!([{ "path": "/uploads/purg.jpg", "order": 0 }]);
        let seed = json!([valid_card(), purgatoried]);
        let path = std::env::temp_dir().join("seed_deleted_at_test.json");
        std::fs::write(&path, seed.to_string()).unwrap();

        seed_if_empty(&pool, path.to_str().unwrap()).await.unwrap();

        let active: Option<String> = sqlx::query_scalar("SELECT deleted_at FROM cards WHERE id = '7'")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(active, None, "unmarked card seeds as live");
        let deleted: Option<String> = sqlx::query_scalar("SELECT deleted_at FROM cards WHERE id = '8'")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(deleted.as_deref(), Some("2026-07-17 12:55:29"), "marker restores deleted_at");
        let body: String = sqlx::query_scalar("SELECT body FROM cards WHERE id = '8'")
            .fetch_one(&pool).await.unwrap();
        let v: Value = serde_json::from_str(&body).unwrap();
        assert!(v.get("deletedAt").is_none(), "marker is stripped from the stored body");

        // Second run must be a no-op (table non-empty → seed skipped).
        seed_if_empty(&pool, path.to_str().unwrap()).await.unwrap();
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM cards")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(count, 2, "reseed on populated db is a no-op");

        // The startup migration pass must not resurrect purgatoried cards
        // when it rewrites their bodies (image sync strips paths).
        normalize_bodies(&pool).await.unwrap();
        let deleted: Option<String> = sqlx::query_scalar("SELECT deleted_at FROM cards WHERE id = '8'")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(deleted.as_deref(), Some("2026-07-17 12:55:29"), "normalize preserves deleted_at");
        let synced: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM images WHERE card_id = '8'")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(synced, 1, "purgatoried card's images still sync to the images table");
    }
}
