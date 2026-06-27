//! Card Catalog API — Axum + SQLite (single-user, no auth).
//!
//! Routes:
//!   GET    /api/cards        -> [Card]   (ordered by catalogNumber)
//!   GET    /api/cards/:id    -> Card
//!   PUT    /api/cards/:id    -> Card     (whole-object upsert)
//!   POST   /api/cards        -> Card     (create; body must include id)
//!   DELETE /api/cards/:id    -> 204
//!   POST   /api/images       -> { path } (multipart upload to uploads/)
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
    extract::{FromRequestParts, Multipart, Path, State},
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
/// take it are gated behind authentication; its presence in a handler signature
/// is what protects that route. Returns 401 on missing/invalid/expired tokens.
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

type ApiError = (StatusCode, String);

fn err(code: StatusCode, msg: impl ToString) -> ApiError {
    (code, msg.to_string())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_target(false).init();

    let db_path = std::env::var("DATABASE_PATH").unwrap_or_else(|_| "data.db".into());

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

    let uploads_dir = PathBuf::from(std::env::var("UPLOADS_DIR").unwrap_or_else(|_| "uploads".into()));
    let seed_path = std::env::var("SEED_PATH").unwrap_or_else(|_| "seed/cards.json".into());
    // Directory of the built Vue frontend (Vite `dist`). Empty/missing in dev,
    // where Vite serves the app instead; set in production so this binary serves
    // the SPA itself.
    let frontend_dir = std::env::var("FRONTEND_DIR").unwrap_or_else(|_| "static".into());
    let port: u16 = std::env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(8787);

    std::fs::create_dir_all(&uploads_dir)?;
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
        .route("/api/images", post(upload_image))
        .nest_service("/uploads", ServeDir::new(uploads_dir))
        .fallback_service(spa)
        // Stop browsers from MIME-sniffing a response into something executable
        // (defense-in-depth for user-served /uploads files).
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
        if v.get("sections").is_some() {
            continue; // already new shape
        }
        normalize_card(&mut v);
        upsert(pool, &v).await?;
        migrated += 1;
    }
    if migrated > 0 {
        tracing::info!("normalized {migrated} card(s) to the sections shape");
    }
    Ok(())
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
    // The URL id wins, to keep the row id and body id consistent.
    body["id"] = json!(id);
    upsert(&st.pool, &body)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(body))
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

/// Detect a raster image type from its magic bytes. Returns a safe extension, or
/// None for anything that isn't an allowed image. The client-supplied filename is
/// never trusted — the extension is derived from the actual content so an
/// attacker can't upload, say, an .html/.svg that would be served and executed
/// from this origin. SVG is intentionally NOT allowed (it can carry script).
fn sniff_image_ext(buf: &[u8]) -> Option<&'static str> {
    if buf.len() >= 3 && buf[0] == 0xFF && buf[1] == 0xD8 && buf[2] == 0xFF {
        return Some("jpg");
    }
    if buf.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Some("png");
    }
    if buf.starts_with(b"GIF87a") || buf.starts_with(b"GIF89a") {
        return Some("gif");
    }
    if buf.len() >= 12 && &buf[0..4] == b"RIFF" && &buf[8..12] == b"WEBP" {
        return Some("webp");
    }
    None
}

/// Accept a single multipart file field, validate it is a real image, store it
/// under uploads/ with a server-chosen UUID name + content-derived extension, and
/// return its URL path.
async fn upload_image(
    State(st): State<AppState>,
    _auth: AuthUser,
    mut multipart: Multipart,
) -> Result<Json<Value>, ApiError> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| err(StatusCode::BAD_REQUEST, e))?
    {
        let data = field
            .bytes()
            .await
            .map_err(|e| err(StatusCode::BAD_REQUEST, e))?;
        // Validate by content, not by the client's filename/content-type.
        let ext = sniff_image_ext(&data).ok_or_else(|| {
            err(
                StatusCode::BAD_REQUEST,
                "unsupported file type — only PNG, JPEG, GIF, or WebP images are allowed",
            )
        })?;
        let filename = format!("{}.{}", uuid::Uuid::new_v4(), ext);
        let dest = st.uploads_dir.join(&filename);
        std::fs::write(&dest, &data).map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
        return Ok(Json(json!({ "path": format!("/uploads/{filename}") })));
    }
    Err(err(StatusCode::BAD_REQUEST, "no file field in upload"))
}
