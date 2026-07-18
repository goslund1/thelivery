//! Trash pipeline: uploads stats, orphan scanning, move-to-trash on delete,
//! trash listing/restore/purge. Files are moved, never hard-deleted, until an
//! explicit purge.

use std::path::PathBuf;

use axum::{extract::State, http::StatusCode, Json};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::{Row, SqlitePool};

use crate::auth::{AdminUser, AuthUser};
use crate::state::{err, ApiError, AppState};

// --- Admin ------------------------------------------------------------------

/// Recursively collect all file paths under a directory.
pub fn walk_files(dir: &std::path::Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() { walk_files(&path, out); } else { out.push(path); }
    }
}

/// Collect paths referenced in the images table (relative to uploads_dir, forward-slash separated).
pub async fn referenced_paths(pool: &SqlitePool) -> Result<std::collections::HashSet<String>, ApiError> {
    let rows = sqlx::query("SELECT path, thumb_path, stage_path FROM images")
        .fetch_all(pool)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let mut set = std::collections::HashSet::new();
    for row in &rows {
        let path: String = row.get("path");
        set.insert(path.trim_start_matches('/').trim_start_matches("uploads/").to_string());
        if let Some(tp) = row.get::<Option<String>, _>("thumb_path") {
            set.insert(tp.trim_start_matches('/').trim_start_matches("uploads/").to_string());
        }
        if let Some(sp) = row.get::<Option<String>, _>("stage_path") {
            set.insert(sp.trim_start_matches('/').trim_start_matches("uploads/").to_string());
        }
    }
    Ok(set)
}

pub async fn admin_stats(
    _admin: AdminUser,
    State(st): State<AppState>,
) -> Result<Json<Value>, ApiError> {
    let card_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM cards")
        .fetch_one(&st.pool).await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let rows = sqlx::query("SELECT body FROM cards")
        .fetch_all(&st.pool).await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let image_count: usize = rows.iter()
        .filter_map(|r| serde_json::from_str::<Value>(r.get::<String, _>("body").as_str()).ok())
        .map(|v| v["images"].as_array().map(|a| a.len()).unwrap_or(0))
        .sum();

    let mut all_files = Vec::new();
    walk_files(&st.uploads_dir, &mut all_files);
    let (file_count, uploads_bytes) = all_files.iter().fold((0u64, 0u64), |(n, b), f| {
        let size = std::fs::metadata(f).map(|m| m.len()).unwrap_or(0);
        (n + 1, b + size)
    });

    let db_bytes = std::fs::metadata(&st.db_path).map(|m| m.len()).unwrap_or(0);

    Ok(Json(json!({
        "cardCount": card_count,
        "imageCount": image_count,
        "fileCount": file_count,
        "uploadsDirBytes": uploads_bytes,
        "dbBytes": db_bytes,
    })))
}

pub async fn admin_scan_orphans(
    _admin: AdminUser,
    State(st): State<AppState>,
) -> Result<Json<Value>, ApiError> {
    let refs = referenced_paths(&st.pool).await?;
    let mut all_files = Vec::new();
    walk_files(&st.uploads_dir, &mut all_files);

    let orphan_paths: Vec<String> = all_files.iter()
        .filter_map(|f| {
            let rel = f.strip_prefix(&st.uploads_dir).ok()?;
            // Normalise to forward slashes for the response.
            let s = rel.components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            if s.starts_with("trash/") { return None; } // intentionally moved files
            if !refs.contains(&s) { Some(s) } else { None }
        })
        .collect();

    let db_orphan_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM images WHERE card_id NOT IN (SELECT id FROM cards WHERE deleted_at IS NULL)"
    )
    .fetch_one(&st.pool)
    .await
    .unwrap_or(0);

    Ok(Json(json!({ "count": orphan_paths.len(), "paths": orphan_paths, "dbOrphanCount": db_orphan_count })))
}

pub async fn admin_delete_orphans(
    _admin: AdminUser,
    State(st): State<AppState>,
) -> Result<Json<Value>, ApiError> {
    let refs = referenced_paths(&st.pool).await?;
    let mut all_files = Vec::new();
    walk_files(&st.uploads_dir, &mut all_files);

    let trash_dir = st.uploads_dir.join("trash");
    let _ = std::fs::create_dir_all(&trash_dir);

    let mut moved = 0u64;
    for f in &all_files {
        if let Ok(rel) = f.strip_prefix(&st.uploads_dir) {
            let s = rel.components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            if s.starts_with("trash/") { continue; }
            if s.starts_with("recovered/") { continue; }
            if !refs.contains(&s) {
                if let Some(trash_name) = move_to_trash(f, &trash_dir) {
                    let original_path = format!("/uploads/{s}");
                    let _ = sqlx::query(
                        "INSERT INTO trash_log (trash_filename, original_path, reason) VALUES (?, ?, 'orphan')",
                    )
                    .bind(&trash_name)
                    .bind(&original_path)
                    .execute(&st.pool)
                    .await;
                    moved += 1;
                }
            }
        }
    }

    // Also delete image rows whose card_id references a non-existent (and non-trashed) card.
    let db_orphans_deleted = sqlx::query(
        "DELETE FROM images WHERE card_id NOT IN (SELECT id FROM cards WHERE deleted_at IS NULL)"
    )
    .execute(&st.pool)
    .await
    .map(|r| r.rows_affected())
    .unwrap_or(0);

    Ok(Json(json!({ "moved": moved, "dbOrphansDeleted": db_orphans_deleted })))
}

/// Move a single file to the trash directory. Returns the new basename in trash.
pub fn move_to_trash(src: &std::path::Path, trash_dir: &std::path::Path) -> Option<String> {
    if !src.exists() { return None; }
    let original_name = src.file_name()?.to_str()?;
    let uuid = uuid::Uuid::new_v4().to_string().replace('-', "");
    let trash_name = format!("{}_{original_name}", &uuid[..8]);
    let dest = trash_dir.join(&trash_name);
    std::fs::rename(src, dest).ok()?;
    Some(trash_name)
}

/// True when every component of the path (after the /uploads/ prefix is
/// stripped) is a plain name — no `..`, `.`, or root components. Joining
/// unchecked input into uploads_dir is not enough: `PathBuf::starts_with`
/// compares components lexically, so `uploads/../x` passes it.
pub fn is_safe_upload_path(path_str: &str) -> bool {
    use std::path::Component;
    let stripped = path_str.trim_start_matches('/').trim_start_matches("uploads/");
    !stripped.is_empty()
        && std::path::Path::new(stripped)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
}

/// Move an image (and its thumb/stage variants) to trash, log the event, and
/// remove the images table row. Called when an image is explicitly removed from
/// a card (reason = 'user_delete') or auto-detected as orphaned (reason = 'orphan').
pub async fn trash_image(
    pool: &SqlitePool,
    uploads_dir: &std::path::Path,
    trash_dir: &std::path::Path,
    path_str: &str,
    reason: &str,
) -> bool {
    if !is_safe_upload_path(path_str) {
        tracing::warn!("rejected unsafe trash path: {path_str:?}");
        return false;
    }
    let row = sqlx::query(
        "SELECT id, card_id, path, thumb_path, stage_path FROM images WHERE path = ?",
    )
    .bind(path_str)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    let rel = |p: &str| -> std::path::PathBuf {
        let r = p.trim_start_matches('/').trim_start_matches("uploads/");
        uploads_dir.join(r)
    };

    if let Some(row) = row {
        let img_id: i64 = row.get("id");
        let card_id: Option<String> = row.try_get("card_id").unwrap_or(None);
        let orig_path: String = row.get("path");
        let thumb_path: Option<String> = row.try_get("thumb_path").unwrap_or(None);
        let stage_path: Option<String> = row.try_get("stage_path").unwrap_or(None);

        let trash_orig  = move_to_trash(&rel(&orig_path), trash_dir);
        let trash_thumb = thumb_path.as_deref().and_then(|p| move_to_trash(&rel(p), trash_dir));
        let trash_stage = stage_path.as_deref().and_then(|p| move_to_trash(&rel(p), trash_dir));

        let primary = trash_orig.as_deref().unwrap_or("").to_string();
        let _ = sqlx::query(
            "INSERT INTO trash_log (trash_filename, trash_thumb_filename, trash_stage_filename,
               original_path, original_thumb_path, original_stage_path,
               card_id, images_row_id, reason)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&primary)
        .bind(&trash_thumb)
        .bind(&trash_stage)
        .bind(&orig_path)
        .bind(&thumb_path)
        .bind(&stage_path)
        .bind(&card_id)
        .bind(img_id)
        .bind(reason)
        .execute(pool)
        .await;

        let _ = sqlx::query("DELETE FROM images WHERE id = ?")
            .bind(img_id)
            .execute(pool)
            .await;
        true
    } else {
        // No images row — lone file (thumb/stage of already-handled image, or legacy).
        let target = rel(path_str);
        if target.starts_with(uploads_dir) {
            move_to_trash(&target, trash_dir).is_some()
        } else {
            false
        }
    }
}

/// Move a list of uploaded image paths to trash (all three variants).
/// Body: { "paths": ["/uploads/folder/001.jpg", ...] }
pub async fn delete_images(
    State(st): State<AppState>,
    auth: AuthUser,
    Json(body): Json<Value>,
) -> StatusCode {
    let paths = match body.get("paths").and_then(Value::as_array) {
        Some(p) => p.clone(),
        None => return StatusCode::NO_CONTENT,
    };

    let trash_dir = st.uploads_dir.join("trash");
    let _ = std::fs::create_dir_all(&trash_dir);

    // Track which images rows we've already handled so thumb/stage don't create dupe log entries.
    let mut handled: std::collections::HashSet<String> = std::collections::HashSet::new();
    for v in &paths {
        let path_str = match v.as_str() {
            Some(s) if !s.is_empty() => s,
            _ => continue,
        };
        // Only process original paths (the images.path column); thumb/stage are covered by trash_image().
        if handled.contains(path_str) { continue; }
        handled.insert(path_str.to_string());
        if trash_image(&st.pool, &st.uploads_dir, &trash_dir, path_str, "user_delete").await {
            // Trash-based, not permanent — reversible via the Admin tab's trash restore.
            crate::audit::record(
                &st.pool, &auth.username, "image.delete", "image", None,
                Some(serde_json::json!({ "path": path_str })),
            ).await;
        }
    }
    StatusCode::NO_CONTENT
}

// --- Admin: trash management ------------------------------------------------

pub async fn admin_list_trash(
    _admin: AdminUser,
    State(st): State<AppState>,
) -> Result<Json<Value>, ApiError> {
    let rows = sqlx::query(
        "SELECT id, trash_filename, trash_thumb_filename, trash_stage_filename,
                original_path, original_thumb_path, original_stage_path,
                card_id, images_row_id, reason, trashed_at
         FROM trash_log ORDER BY trashed_at DESC",
    )
    .fetch_all(&st.pool)
    .await
    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let trash_dir = st.uploads_dir.join("trash");

    let mut logged_names: std::collections::HashSet<String> = std::collections::HashSet::new();

    let mut entries: Vec<Value> = rows.iter().map(|r| {
        let fname: String = r.get("trash_filename");
        logged_names.insert(fname.clone());
        let fpath = trash_dir.join(&fname);
        let bytes = std::fs::metadata(&fpath).map(|m| m.len()).unwrap_or(0);
        json!({
            "id":                 r.get::<i64, _>("id"),
            "trashFilename":      fname,
            "originalPath":       r.get::<String, _>("original_path"),
            "originalThumbPath":  r.get::<Option<String>, _>("original_thumb_path"),
            "originalStagePath":  r.get::<Option<String>, _>("original_stage_path"),
            "cardId":             r.get::<Option<String>, _>("card_id"),
            "reason":             r.get::<String, _>("reason"),
            "trashedAt":          r.get::<String, _>("trashed_at"),
            "onDisk":             fpath.exists(),
            "bytes":              bytes,
        })
    }).collect();

    // Include unlogged files in trash/ as 'unknown' entries (e.g. migration-era files).
    if let Ok(dir_iter) = std::fs::read_dir(&trash_dir) {
        for entry in dir_iter.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if logged_names.contains(&name) { continue; }
            let bytes = std::fs::metadata(entry.path()).map(|m| m.len()).unwrap_or(0);
            entries.push(json!({
                "id":             null,
                "trashFilename":  name,
                "originalPath":   null,
                "reason":         "unknown",
                "trashedAt":      null,
                "onDisk":         true,
                "bytes":          bytes,
            }));
        }
    }

    let total_bytes: u64 = entries.iter().filter_map(|e| e["bytes"].as_u64()).sum();
    Ok(Json(json!({ "entries": entries, "totalBytes": total_bytes })))
}

#[derive(Deserialize)]
pub struct DeleteTrashReq {
    ids: Option<Vec<i64>>,
    all: Option<bool>,
    unknown: Option<bool>, // also wipe unlogged files
}

pub async fn admin_delete_trash(
    _admin: AdminUser,
    State(st): State<AppState>,
    Json(req): Json<DeleteTrashReq>,
) -> Result<Json<Value>, ApiError> {
    let trash_dir = st.uploads_dir.join("trash");
    let mut deleted = 0u64;

    if req.all.unwrap_or(false) {
        // Wipe every file in the trash directory.
        if let Ok(iter) = std::fs::read_dir(&trash_dir) {
            for entry in iter.flatten() {
                if std::fs::remove_file(entry.path()).is_ok() { deleted += 1; }
            }
        }
        sqlx::query("DELETE FROM trash_log")
            .execute(&st.pool)
            .await
            .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    } else {
        let ids = req.ids.clone().unwrap_or_default();
        for id in &ids {
            let row = sqlx::query(
                "SELECT trash_filename, trash_thumb_filename, trash_stage_filename FROM trash_log WHERE id = ?",
            )
            .bind(id)
            .fetch_optional(&st.pool)
            .await
            .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;

            if let Some(row) = row {
                for col in &["trash_filename", "trash_thumb_filename", "trash_stage_filename"] {
                    if let Some(fname) = row.try_get::<Option<String>, _>(col).unwrap_or(None) {
                        if !fname.is_empty() {
                            if std::fs::remove_file(trash_dir.join(&fname)).is_ok() { deleted += 1; }
                        }
                    }
                }
                sqlx::query("DELETE FROM trash_log WHERE id = ?")
                    .bind(id)
                    .execute(&st.pool)
                    .await
                    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
            }
        }
        // Also delete unlogged (unknown) files by filename if requested.
        if req.unknown.unwrap_or(false) {
            if let Ok(iter) = std::fs::read_dir(&trash_dir) {
                let logged: std::collections::HashSet<String> = sqlx::query_scalar(
                    "SELECT trash_filename FROM trash_log",
                )
                .fetch_all(&st.pool)
                .await
                .unwrap_or_default()
                .into_iter()
                .collect();
                for entry in iter.flatten() {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    if !logged.contains(&name) {
                        if std::fs::remove_file(entry.path()).is_ok() { deleted += 1; }
                    }
                }
            }
        }
    }

    Ok(Json(json!({ "deleted": deleted })))
}

#[derive(Deserialize)]
pub struct RestoreTrashReq {
    ids: Vec<i64>,
}

pub async fn admin_restore_trash(
    _admin: AdminUser,
    State(st): State<AppState>,
    Json(req): Json<RestoreTrashReq>,
) -> Result<Json<Value>, ApiError> {
    let trash_dir = st.uploads_dir.join("trash");
    let mut restored = 0u64;
    let mut image_ids: Vec<i64> = Vec::new();

    for log_id in &req.ids {
        let row = sqlx::query(
            "SELECT trash_filename, trash_thumb_filename, trash_stage_filename,
                    original_path, original_thumb_path, original_stage_path,
                    card_id FROM trash_log WHERE id = ?",
        )
        .bind(log_id)
        .fetch_optional(&st.pool)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;

        let Some(row) = row else { continue };

        let restore_file = |trash_name: Option<String>, orig: Option<&str>| -> Option<String> {
            let tname = trash_name.filter(|s| !s.is_empty())?;
            let src = trash_dir.join(&tname);
            let orig_path = orig?;
            let rel = orig_path.trim_start_matches('/').trim_start_matches("uploads/");
            let dest = st.uploads_dir.join(rel);
            if let Some(parent) = dest.parent() { let _ = std::fs::create_dir_all(parent); }
            // If original path is occupied, put it in a recovered/ subfolder.
            let actual_dest = if dest.exists() {
                let fname = dest.file_name().and_then(|f| f.to_str()).unwrap_or("file");
                st.uploads_dir.join("recovered").join(fname)
            } else {
                dest.clone()
            };
            if let Some(parent) = actual_dest.parent() { let _ = std::fs::create_dir_all(parent); }
            std::fs::rename(&src, &actual_dest).ok()?;
            let actual_rel = actual_dest.strip_prefix(&st.uploads_dir).ok()?;
            let rel_str = actual_rel.components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            Some(format!("/uploads/{rel_str}"))
        };

        let orig_path: String = row.get("original_path");
        let orig_thumb: Option<String> = row.try_get("original_thumb_path").unwrap_or(None);
        let orig_stage: Option<String> = row.try_get("original_stage_path").unwrap_or(None);
        let card_id: Option<String> = row.try_get("card_id").unwrap_or(None);

        let trash_orig: Option<String>  = row.try_get("trash_filename").unwrap_or(None);
        let trash_thumb: Option<String> = row.try_get("trash_thumb_filename").unwrap_or(None);
        let trash_stage: Option<String> = row.try_get("trash_stage_filename").unwrap_or(None);

        let new_path  = restore_file(trash_orig,  Some(&orig_path));
        let new_thumb = restore_file(trash_thumb, orig_thumb.as_deref());
        let new_stage = restore_file(trash_stage, orig_stage.as_deref());

        if new_path.is_some() || new_thumb.is_some() {
            // Re-insert into images table so it shows up for reassignment.
            let result = sqlx::query(
                "INSERT INTO images (card_id, path, thumb_path, stage_path) VALUES (?, ?, ?, ?)",
            )
            .bind(&card_id)
            .bind(&new_path)
            .bind(&new_thumb)
            .bind(&new_stage)
            .execute(&st.pool)
            .await
            .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
            image_ids.push(result.last_insert_rowid());
            restored += 1;
        }

        sqlx::query("DELETE FROM trash_log WHERE id = ?")
            .bind(log_id)
            .execute(&st.pool)
            .await
            .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    }

    Ok(Json(json!({ "restored": restored, "imageIds": image_ids })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_upload_paths_pass() {
        assert!(is_safe_upload_path("/uploads/smokin_1/photo.jpg"));
        assert!(is_safe_upload_path("uploads/a/b/c.png"));
        assert!(is_safe_upload_path("/uploads/legend-0.png"));
    }

    #[test]
    fn traversal_and_degenerate_paths_are_rejected() {
        assert!(!is_safe_upload_path("/uploads/../secrets.env"));
        assert!(!is_safe_upload_path("/uploads/a/../../data.db"));
        assert!(!is_safe_upload_path("/uploads/./a.jpg"));
        assert!(!is_safe_upload_path("../outside.jpg"));
        assert!(!is_safe_upload_path("/uploads/"));
        assert!(!is_safe_upload_path(""));
    }

    #[tokio::test]
    async fn trash_image_refuses_traversal_paths() {
        let pool = crate::testutil::test_pool().await;
        let uploads = std::env::temp_dir().join("trash_test_uploads");
        let trash = uploads.join("trash");
        std::fs::create_dir_all(&trash).unwrap();
        // Bait file OUTSIDE uploads that a traversal path would reach.
        let bait = std::env::temp_dir().join("trash_test_bait.txt");
        std::fs::write(&bait, "bait").unwrap();

        let moved = trash_image(&pool, &uploads, &trash, "/uploads/../trash_test_bait.txt", "user_delete").await;

        assert!(!moved, "traversal path must be rejected");
        assert!(bait.exists(), "file outside uploads/ must be untouched");
        std::fs::remove_file(&bait).ok();
    }
}
