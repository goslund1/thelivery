//! Card Catalog API — Axum + SQLite (single-user, no auth).
//!
//! Routes:
//!   GET    /api/cards        -> [Card]   (ordered by catalogNumber)
//!   GET    /api/cards/:id    -> Card
//!   PUT    /api/cards/:id    -> Card     (whole-object upsert)
//!   POST   /api/cards        -> Card     (create; body must include id)
//!   DELETE /api/cards/:id    -> 204
//!   POST   /api/images       -> { path, thumbPath, stagePath } (multipart upload)
//!   GET    /uploads/*        -> static files
//!   GET    /api/health       -> "ok"

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use argon2::password_hash::rand_core::{OsRng, RngCore};
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use axum::{
    async_trait,
    extract::{DefaultBodyLimit, FromRequestParts, Multipart, Path, State},
    http::{header::AUTHORIZATION, request::Parts, HeaderName, HeaderValue, StatusCode},
    routing::{get, post},
    Json, Router,
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{sqlite::SqliteConnectOptions, Row, SqlitePool};
use tower_http::{
    cors::CorsLayer,
    services::{ServeDir, ServeFile},
    set_header::SetResponseHeaderLayer,
};

#[derive(Clone)]
struct AppState {
    pool: SqlitePool,
    uploads_dir: PathBuf,
    jwt_secret: Arc<Vec<u8>>,
}

type ApiError = (StatusCode, String);

fn err(code: StatusCode, msg: impl ToString) -> ApiError {
    (code, msg.to_string())
}

// --- Authentication ---------------------------------------------------------

#[derive(Serialize, Deserialize)]
struct Claims {
    sub: String, // username
    exp: usize,  // expiry (unix seconds)
}

const TOKEN_TTL_SECS: u64 = 7 * 24 * 3600; // 7 days

fn make_token(username: &str, secret: &[u8]) -> anyhow::Result<String> {
    let exp = (SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + TOKEN_TTL_SECS) as usize;
    let claims = Claims { sub: username.to_string(), exp };
    Ok(encode(&Header::default(), &claims, &EncodingKey::from_secret(secret))?)
}

// JWT signing secret: from JWT_SECRET in prod; a random ephemeral secret in dev
// (so we never ship a known-weak default). Ephemeral secrets reset tokens on
// restart, which is fine for dev — set JWT_SECRET in production.
fn load_jwt_secret() -> Arc<Vec<u8>> {
    match std::env::var("JWT_SECRET") {
        Ok(s) if !s.is_empty() => Arc::new(s.into_bytes()),
        _ => {
            tracing::warn!(
                "JWT_SECRET not set — using a random ephemeral secret (logins reset on restart). Set JWT_SECRET in production."
            );
            let mut bytes = [0u8; 32];
            OsRng.fill_bytes(&mut bytes);
            Arc::new(bytes.to_vec())
        }
    }
}

/// Extractor that requires a valid `Authorization: Bearer <jwt>`. Handlers that
/// take it are gated behind authentication; returns 401 on missing/invalid/
/// expired tokens.
struct AuthUser(#[allow(dead_code)] String);

#[async_trait]
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = ApiError;
    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "))
            .ok_or_else(|| err(StatusCode::UNAUTHORIZED, "missing bearer token"))?;
        let data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(&state.jwt_secret),
            &Validation::default(),
        )
        .map_err(|_| err(StatusCode::UNAUTHORIZED, "invalid or expired token"))?;
        Ok(AuthUser(data.claims.sub))
    }
}

#[derive(Deserialize)]
struct LoginReq {
    username: String,
    password: String,
}

/// Verify credentials against the users table and issue a JWT. Uses a generic
/// "invalid credentials" error for both unknown users and bad passwords.
async fn login(State(st): State<AppState>, Json(req): Json<LoginReq>) -> Result<Json<Value>, ApiError> {
    let row = sqlx::query("SELECT password_hash FROM users WHERE username = ?")
        .bind(&req.username)
        .fetch_optional(&st.pool)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let ok = row
        .and_then(|r| PasswordHash::new(&r.get::<String, _>("password_hash")).ok().map(|ph| {
            Argon2::default()
                .verify_password(req.password.as_bytes(), &ph)
                .is_ok()
        }))
        .unwrap_or(false);
    if !ok {
        return Err(err(StatusCode::UNAUTHORIZED, "invalid credentials"));
    }
    let token = make_token(&req.username, &st.jwt_secret)
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(json!({ "token": token, "username": req.username })))
}

/// `adduser` CLI: prompt for a password and insert an Argon2-hashed user.
async fn add_user(pool: &SqlitePool, username: &str) -> anyhow::Result<()> {
    let password = rpassword::prompt_password(format!("Password for '{username}': "))?;
    let confirm = rpassword::prompt_password("Confirm password: ")?;
    if password != confirm {
        anyhow::bail!("passwords do not match");
    }
    if password.len() < 8 {
        anyhow::bail!("password must be at least 8 characters");
    }
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("hashing failed: {e}"))?
        .to_string();
    match sqlx::query("INSERT INTO users (username, password_hash) VALUES (?, ?)")
        .bind(username)
        .bind(&hash)
        .execute(pool)
        .await
    {
        Ok(_) => {
            println!("✓ user '{username}' created");
            Ok(())
        }
        Err(e) if e.to_string().contains("UNIQUE") => anyhow::bail!("user '{username}' already exists"),
        Err(e) => Err(e.into()),
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_target(false).init();

    let db_path = std::env::var("DATABASE_PATH").unwrap_or_else(|_| "data.db".into());
    let uploads_dir = PathBuf::from(std::env::var("UPLOADS_DIR").unwrap_or_else(|_| "uploads".into()));
    let seed_path = std::env::var("SEED_PATH").unwrap_or_else(|_| "seed/cards.json".into());
    // Directory of the built Vue frontend (Vite `dist`). Empty/missing in dev,
    // where Vite serves the app instead; set in production so this binary serves
    // the SPA itself.
    let frontend_dir = std::env::var("FRONTEND_DIR").unwrap_or_else(|_| "static".into());
    let port: u16 = std::env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(8787);

    std::fs::create_dir_all(&uploads_dir)?;

    let opts = SqliteConnectOptions::new()
        .filename(&db_path)
        .create_if_missing(true);
    let pool = SqlitePool::connect_with(opts).await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    // CLI: `livery-backend adduser <username>` — create a user, then exit.
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("adduser") {
        let Some(username) = args.get(2) else {
            eprintln!("usage: livery-backend adduser <username>");
            std::process::exit(2);
        };
        add_user(&pool, username).await?;
        return Ok(());
    }

    seed_if_empty(&pool, &seed_path).await?;
    normalize_bodies(&pool).await?;

    let state = AppState {
        pool,
        uploads_dir: uploads_dir.clone(),
        jwt_secret: load_jwt_secret(),
    };

    // Serve the built SPA: real files (index.html at "/", hashed assets) are
    // served directly; any other path falls back to index.html. (This app has no
    // client-side router, so "/" is the only real entry point.)
    let spa = ServeDir::new(&frontend_dir)
        .not_found_service(ServeFile::new(format!("{frontend_dir}/index.html")));

    let app = Router::new()
        .route("/api/health", get(|| async { "ok" }))
        .route("/api/login", post(login))
        .route("/api/cards", get(list_cards).post(create_card))
        .route(
            "/api/cards/:id",
            get(get_card).put(put_card).delete(delete_card),
        )
        .route("/api/cards/:id/history", get(list_card_history))
        .route("/api/cards/:id/history/:version", get(get_card_history_version))
        .route("/api/images", post(upload_image).delete(delete_images))
        .nest_service("/uploads", ServeDir::new(uploads_dir))
        .fallback_service(spa)
        .layer(DefaultBodyLimit::max(40 * 1024 * 1024)) // 40 MB per file
        // Stop browsers from MIME-sniffing a response into something executable.
        .layer(SetResponseHeaderLayer::overriding(
            HeaderName::from_static("x-content-type-options"),
            HeaderValue::from_static("nosniff"),
        ))
        .layer(CorsLayer::permissive())
        .with_state(state);

    // Bind address: 0.0.0.0 in dev; set to 127.0.0.1 in production so only the
    // local reverse proxy (Caddy) can reach the backend.
    let bind = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0".into());
    let addr = format!("{bind}:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("backend listening on http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}

/// Import seed/cards.json on first run (when the table is empty).
async fn seed_if_empty(pool: &SqlitePool, seed_path: &str) -> anyhow::Result<()> {
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
        upsert(pool, c).await?;
    }
    tracing::info!("seeded {} cards from {seed_path}", cards.len());
    Ok(())
}

/// Insert-or-replace a card row from its JSON object.
async fn upsert(pool: &SqlitePool, body: &Value) -> Result<(), sqlx::Error> {
    let id = body.get("id").and_then(Value::as_str).unwrap_or_default().to_string();
    let catalog_number = body.get("catalogNumber").and_then(Value::as_i64).unwrap_or(0);
    let body_str = body.to_string();
    sqlx::query(
        "INSERT INTO cards (id, catalog_number, body) VALUES (?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET catalog_number = excluded.catalog_number, body = excluded.body",
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
async fn normalize_bodies(pool: &SqlitePool) -> anyhow::Result<()> {
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
        if changed {
            upsert(pool, &v).await?;
            migrated += 1;
        }
    }
    if migrated > 0 {
        tracing::info!("normalized {migrated} card(s) to the sections shape");
    }
    Ok(())
}

fn ensure_standard_sections(v: &mut Value) -> bool {
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

fn text_section(key: &str, label: &str, src: &Value) -> Value {
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

fn normalize_card(v: &mut Value) {
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

async fn list_cards(State(st): State<AppState>) -> Result<Json<Vec<Value>>, ApiError> {
    let rows = sqlx::query("SELECT body FROM cards ORDER BY catalog_number")
        .fetch_all(&st.pool)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let out = rows
        .iter()
        .filter_map(|r| serde_json::from_str::<Value>(r.get::<String, _>("body").as_str()).ok())
        .collect();
    Ok(Json(out))
}

async fn get_card(
    State(st): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let row = sqlx::query("SELECT body FROM cards WHERE id = ?")
        .bind(&id)
        .fetch_optional(&st.pool)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "card not found"))?;
    let body: Value = serde_json::from_str(row.get::<String, _>("body").as_str())
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(body))
}

async fn put_card(
    State(st): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<String>,
    Json(mut body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    // Card 0 (the instructions card) is read-only through the normal API.
    let catalog_number = body.get("catalogNumber").and_then(Value::as_i64).unwrap_or(-1);
    if catalog_number == 0 {
        return Err(err(StatusCode::FORBIDDEN, "card 0 is read-only"));
    }

    body["id"] = json!(id);

    // Snapshot the current state into history before overwriting.
    let existing: Option<String> = sqlx::query_scalar("SELECT body FROM cards WHERE id = ?")
        .bind(&id)
        .fetch_optional(&st.pool)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;

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

    upsert(&st.pool, &body)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(body))
}

async fn list_card_history(
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

async fn get_card_history_version(
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

async fn create_card(
    State(st): State<AppState>,
    _auth: AuthUser,
    Json(body): Json<Value>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    if body.get("id").and_then(Value::as_str).unwrap_or_default().is_empty() {
        return Err(err(StatusCode::BAD_REQUEST, "body.id is required"));
    }
    upsert(&st.pool, &body)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok((StatusCode::CREATED, Json(body)))
}

async fn delete_card(
    State(st): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    sqlx::query("DELETE FROM cards WHERE id = ?")
        .bind(&id)
        .execute(&st.pool)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(StatusCode::NO_CONTENT)
}

/// Sanitise a string into a filesystem-safe slug: lowercase, collapse
/// non-alphanumeric runs to a single underscore, strip leading/trailing underscores.
fn slugify(s: &str) -> String {
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

/// Build the card folder name from metadata fields sent with the upload.
/// Pattern: {fh_tag}_{subtitle_slug}_{name_slug}  e.g. FH5_Nissan_S13_Midnight_Drift
/// Falls back to "misc" when metadata is absent (e.g. legacy uploads).
fn card_folder(name: &str, subtitle: &str, collections: &str) -> String {
    // Pick first FH* collection tag, or "FHX" if none found.
    let fh_tag = collections
        .split(',')
        .map(|c| c.trim())
        .find(|c| c.to_uppercase().starts_with("FH"))
        .unwrap_or("FHX")
        .to_uppercase();

    let sub_slug = slugify(subtitle);
    let name_slug = slugify(name);

    match (sub_slug.is_empty(), name_slug.is_empty()) {
        (false, false) => format!("{fh_tag}_{sub_slug}_{name_slug}"),
        (true,  false) => format!("{fh_tag}_{name_slug}"),
        (false, true)  => format!("{fh_tag}_{sub_slug}"),
        (true,  true)  => "FHX_misc".into(),
    }
}

/// Delete a list of uploaded image paths (all three variants: original, thumb, stage).
/// Body: { "paths": ["/uploads/folder/001.jpg", ...] }
/// Silently skips paths that don't exist or that escape the uploads directory.
async fn delete_images(
    State(st): State<AppState>,
    _auth: AuthUser,
    Json(body): Json<Value>,
) -> StatusCode {
    let paths = match body.get("paths").and_then(Value::as_array) {
        Some(p) => p.clone(),
        None => return StatusCode::NO_CONTENT,
    };
    for v in paths {
        let rel = match v.as_str() {
            Some(s) => s.trim_start_matches('/').trim_start_matches("uploads/"),
            None => continue,
        };
        // Resolve against uploads_dir and verify it stays inside (no path traversal).
        let target = st.uploads_dir.join(rel);
        if !target.starts_with(&st.uploads_dir) { continue; }

        let stem = target.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
        let dir  = target.parent().unwrap_or(&target);
        let lowres = dir.join("Lowres_Assets");

        // Delete original + both variants (ignore missing files).
        let _ = std::fs::remove_file(&target);
        let _ = std::fs::remove_file(lowres.join(format!("{stem}_200w.jpg")));
        let _ = std::fs::remove_file(lowres.join(format!("{stem}_1000w.jpg")));
    }
    StatusCode::NO_CONTENT
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
async fn upload_image(
    State(st): State<AppState>,
    _auth: AuthUser,
    mut multipart: Multipart,
) -> Result<Json<Value>, ApiError> {
    let mut card_name = String::new();
    let mut card_subtitle = String::new();
    let mut card_collections = String::new();
    let mut file_index: Option<u32> = None;

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
            Some("cardSubtitle") => {
                card_subtitle = field.text().await.unwrap_or_default();
                continue;
            }
            Some("cardCollections") => {
                card_collections = field.text().await.unwrap_or_default();
                continue;
            }
            Some("fileIndex") => {
                file_index = field.text().await.ok().and_then(|s| s.parse().ok());
                continue;
            }
            _ => {}
        }

        // This field is the file.
        let data = field
            .bytes()
            .await
            .map_err(|e| err(StatusCode::BAD_REQUEST, e))?;

        let folder = card_folder(&card_name, &card_subtitle, &card_collections);
        let card_dir = st.uploads_dir.join(&folder);
        let lowres_dir = card_dir.join("Lowres_Assets");

        std::fs::create_dir_all(&card_dir)
            .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
        std::fs::create_dir_all(&lowres_dir)
            .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;

        // Decode image — all uploads become JPEG.
        let img = image::load_from_memory(&data)
            .map_err(|e| err(StatusCode::BAD_REQUEST, e))?;

        // Build stem: sequential number when fileIndex provided, UUID otherwise.
        // Guard against collision (e.g. re-importing same batch).
        let stem = match file_index {
            Some(idx) => {
                let candidate = format!("{folder}_{:03}", idx + 1);
                if card_dir.join(format!("{candidate}.jpg")).exists() {
                    let short = &uuid::Uuid::new_v4().to_string()[..6];
                    format!("{candidate}_{short}")
                } else {
                    candidate
                }
            }
            None => format!("{folder}_{}", uuid::Uuid::new_v4()),
        };

        let orig_name  = format!("{stem}.jpg");
        let thumb_name = format!("{stem}_200w.jpg");
        let stage_name = format!("{stem}_1000w.jpg");

        img.save_with_format(card_dir.join(&orig_name), image::ImageFormat::Jpeg)
            .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;

        let _ = img
            .thumbnail(200, u32::MAX)
            .save_with_format(lowres_dir.join(&thumb_name), image::ImageFormat::Jpeg);
        let _ = img
            .thumbnail(1000, u32::MAX)
            .save_with_format(lowres_dir.join(&stage_name), image::ImageFormat::Jpeg);

        return Ok(Json(json!({
            "path":      format!("/uploads/{folder}/{orig_name}"),
            "thumbPath": format!("/uploads/{folder}/Lowres_Assets/{thumb_name}"),
            "stagePath": format!("/uploads/{folder}/Lowres_Assets/{stage_name}"),
        })));
    }
    Err(err(StatusCode::BAD_REQUEST, "no file field in upload"))
}
