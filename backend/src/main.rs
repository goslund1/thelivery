//! Card Catalog API — Axum + SQLite.
//!
//! Routes:
//!   GET    /api/cards                        -> [Card]   (ordered by catalogNumber)
//!   GET    /api/cards/:id                    -> Card
//!   PUT    /api/cards/:id                    -> Card     (whole-object upsert; auth required)
//!   POST   /api/cards                        -> Card     (create; body must include id; auth required)
//!   DELETE /api/cards/:id                    -> 204      (auth required)
//!   POST   /api/images                       -> { path, thumbPath, stagePath } (multipart upload; auth required)
//!   POST   /api/login                        -> { token, username }
//!   POST   /api/users                        -> { username }  (create user; auth required)
//!   PUT    /api/me/password                  -> { ok }        (change own password; auth required)
//!   GET    /api/admin/stats                  -> { cardCount, imageCount, fileCount, uploadsDirBytes, dbBytes }
//!   GET    /api/admin/orphans                -> { count, paths[] }  (dry-run scan; auth required)
//!   DELETE /api/admin/orphans                -> { deleted }          (auth required)
//!   POST   /api/admin/export-seed            -> { exported }         (auth required)
//!   GET    /uploads/*                        -> static files
//!   GET    /api/cars                          -> [Car]   (public; ?game=FH5|FH6 to filter)
//!   POST   /api/cars                         -> Car     (upsert; auth required)
//!   GET    /api/theme                        -> ThemeBody (public)
//!   PUT    /api/theme                        -> ThemeBody (auth required)
//!   GET    /api/health                       -> "ok"

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use argon2::password_hash::rand_core::{OsRng, RngCore};
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use axum::{
    async_trait,
    extract::{ConnectInfo, DefaultBodyLimit, FromRequestParts, Multipart, Path, Query, State},
    http::{header::AUTHORIZATION, request::Parts, HeaderName, HeaderValue, StatusCode},
    routing::{delete, get, post, put},
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
    seed_path: PathBuf,
    db_path: PathBuf,
    jwt_secret: Arc<Vec<u8>>,
    rate_limits: Arc<tokio::sync::Mutex<HashMap<String, Vec<Instant>>>>,
}

const SUGGESTION_RATE_LIMIT: usize = 3;
const SUGGESTION_RATE_WINDOW: Duration = Duration::from_secs(3600);
const SUGGESTION_TITLE_MAX: usize = 60;

// Patterns that indicate directed hostility or hate — not general profanity.
const BLOCKED_PATTERNS: &[&str] = &[
    "fuck you", "kill yourself", "kys", "go die", "you suck",
    "faggot", "nigger", "nigga", "chink", "spic", "wetback",
    "retard", "cunt", "whore", "bitch ass",
];

fn is_title_blocked(title: &str) -> bool {
    let lower = title.to_lowercase();
    BLOCKED_PATTERNS.iter().any(|p| lower.contains(p))
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

#[derive(Deserialize)]
struct CreateUserReq {
    username: String,
    password: String,
}

/// Create a new user. Requires a valid JWT so only existing users can invite others.
async fn create_user(
    _auth: AuthUser,
    State(st): State<AppState>,
    Json(req): Json<CreateUserReq>,
) -> Result<Json<Value>, ApiError> {
    if req.password.len() < 8 {
        return Err(err(StatusCode::BAD_REQUEST, "password must be at least 8 characters"));
    }
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(req.password.as_bytes(), &salt)
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?
        .to_string();
    sqlx::query("INSERT INTO users (username, password_hash) VALUES (?, ?)")
        .bind(&req.username)
        .bind(&hash)
        .execute(&st.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("UNIQUE") {
                err(StatusCode::CONFLICT, "username already exists")
            } else {
                err(StatusCode::INTERNAL_SERVER_ERROR, e)
            }
        })?;
    Ok(Json(json!({ "username": req.username })))
}

#[derive(Deserialize)]
struct ChangePasswordReq {
    current_password: String,
    new_password: String,
}

/// Change the calling user's own password. Requires the current password to confirm.
async fn change_password(
    AuthUser(username): AuthUser,
    State(st): State<AppState>,
    Json(req): Json<ChangePasswordReq>,
) -> Result<Json<Value>, ApiError> {
    if req.new_password.len() < 8 {
        return Err(err(StatusCode::BAD_REQUEST, "new password must be at least 8 characters"));
    }
    let row = sqlx::query("SELECT password_hash FROM users WHERE username = ?")
        .bind(&username)
        .fetch_optional(&st.pool)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "user not found"))?;
    let stored_hash: String = row.get("password_hash");
    let ph = PasswordHash::new(&stored_hash)
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Argon2::default()
        .verify_password(req.current_password.as_bytes(), &ph)
        .map_err(|_| err(StatusCode::UNAUTHORIZED, "current password is incorrect"))?;
    let salt = SaltString::generate(&mut OsRng);
    let new_hash = Argon2::default()
        .hash_password(req.new_password.as_bytes(), &salt)
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?
        .to_string();
    sqlx::query("UPDATE users SET password_hash = ? WHERE username = ?")
        .bind(&new_hash)
        .bind(&username)
        .execute(&st.pool)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(json!({ "ok": true })))
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
    dotenvy::dotenv().ok();  // load .env if present; ignore if missing
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
    seed_users_if_empty(&pool, "seed/users.json").await?;
    seed_theme_if_empty(&pool).await?;
    seed_cars_if_empty(&pool).await?;
    normalize_bodies(&pool).await?;

    let state = AppState {
        pool,
        uploads_dir: uploads_dir.clone(),
        seed_path: PathBuf::from(&seed_path),
        db_path: PathBuf::from(&db_path),
        jwt_secret: load_jwt_secret(),
        rate_limits: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
    };

    // Serve the built SPA: real files (index.html at "/", hashed assets) are
    // served directly; any other path falls back to index.html. (This app has no
    // client-side router, so "/" is the only real entry point.)
    let spa = ServeDir::new(&frontend_dir)
        .not_found_service(ServeFile::new(format!("{frontend_dir}/index.html")));

    let app = Router::new()
        .route("/api/health", get(|| async { "ok" }))
        .route("/api/login", post(login))
        .route("/api/users", post(create_user))
        .route("/api/me/password", put(change_password))
        .route("/api/cards", get(list_cards).post(create_card))
        .route(
            "/api/cards/:id",
            get(get_card).put(put_card).delete(delete_card),
        )
        .route("/api/cards/:id/history", get(list_card_history))
        .route("/api/cards/:id/history/:version", get(get_card_history_version))
        .route("/api/images", post(upload_image).delete(delete_images))
        .route("/api/admin/stats", get(admin_stats))
        .route("/api/admin/orphans", get(admin_scan_orphans).delete(admin_delete_orphans))
        .route("/api/admin/trash", get(admin_list_trash).delete(admin_delete_trash))
        .route("/api/admin/trash/restore", post(admin_restore_trash))
        .route("/api/admin/export-seed", post(admin_export_seed))
        .route("/api/admin/reload-seed", post(admin_reload_seed))
        .route("/api/suggestions", post(submit_suggestion))
        .route("/api/admin/suggestions", get(admin_list_suggestions))
        .route("/api/admin/suggestions/:id", delete(admin_dismiss_suggestion).patch(admin_like_suggestion))
        .route("/api/admin/liveries/:id/assess-color", post(admin_assess_livery_color))
        .route("/api/admin/images/migrate", post(admin_migrate_images))
        .route("/api/admin/repair-figure-paths", post(admin_repair_figure_paths))
        .route("/api/cars", get(list_cars).post(create_car))
        .route("/api/tune-types", get(list_tune_types).post(create_tune_type))
        .route("/api/liveries", get(list_liveries).post(create_livery))
        .route("/api/liveries/:id", put(update_livery).delete(delete_livery))
        .route("/api/tunes", get(list_tunes).post(create_tune))
        .route("/api/tunes/:id", put(update_tune).delete(delete_tune))
        .route("/api/theme", get(get_theme).put(put_theme))
        .route("/api/tuning-presets", get(list_tuning_presets).post(create_tuning_preset))
        .route("/api/tuning-presets/:id", delete(delete_tuning_preset))
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
    axum::serve(listener, app.into_make_service_with_connect_info::<std::net::SocketAddr>()).await?;
    Ok(())
}

// --- Suggestions ------------------------------------------------------------

#[derive(Deserialize)]
struct SubmitSuggestionReq {
    card_id: String,
    title: String,
    credit: Option<String>,
    adjustments: Value,
}

async fn submit_suggestion(
    State(st): State<AppState>,
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    Json(req): Json<SubmitSuggestionReq>,
) -> Result<Json<Value>, ApiError> {
    let ip = addr.ip().to_string();

    // Rate limit: 3 per hour per IP.
    // Sweep the whole map on each request so IPs that never return don't accumulate forever.
    {
        let mut limits = st.rate_limits.lock().await;
        let now = Instant::now();
        limits.retain(|_, entries| {
            entries.retain(|t| now.duration_since(*t) < SUGGESTION_RATE_WINDOW);
            !entries.is_empty()
        });
        let entries = limits.entry(ip.clone()).or_default();
        if entries.len() >= SUGGESTION_RATE_LIMIT {
            return Err(err(StatusCode::TOO_MANY_REQUESTS, "Too many suggestions — try again later"));
        }
        entries.push(now);
    }

    // Validate title
    let title = req.title.trim().to_string();
    if title.is_empty() || title.chars().count() > SUGGESTION_TITLE_MAX {
        return Err(err(StatusCode::UNPROCESSABLE_ENTITY, format!("Title must be 1–{SUGGESTION_TITLE_MAX} characters")));
    }
    if is_title_blocked(&title) {
        return Err(err(StatusCode::UNPROCESSABLE_ENTITY, "Title contains prohibited content"));
    }

    let credit = req.credit.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    let adjustments = req.adjustments.to_string();
    if adjustments.len() > 65_536 {
        return Err(err(StatusCode::UNPROCESSABLE_ENTITY, "adjustments payload too large"));
    }

    sqlx::query(
        "INSERT INTO suggestions (card_id, title, credit, adjustments, ip) VALUES (?, ?, ?, ?, ?)"
    )
    .bind(&req.card_id)
    .bind(&title)
    .bind(&credit)
    .bind(&adjustments)
    .bind(&ip)
    .execute(&st.pool)
    .await
    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    Ok(Json(json!({ "ok": true })))
}

async fn admin_list_suggestions(
    State(st): State<AppState>,
    _auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let rows = sqlx::query(
        "SELECT id, card_id, title, credit, adjustments, submitted_at, ip, status FROM suggestions ORDER BY submitted_at DESC"
    )
    .fetch_all(&st.pool)
    .await
    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let list: Vec<Value> = rows.iter().map(|r| json!({
        "id":           r.get::<i64, _>("id"),
        "cardId":       r.get::<String, _>("card_id"),
        "title":        r.get::<String, _>("title"),
        "credit":       r.get::<Option<String>, _>("credit"),
        "adjustments":  serde_json::from_str::<Value>(&r.get::<String, _>("adjustments")).unwrap_or(Value::Null),
        "submittedAt":  r.get::<String, _>("submitted_at"),
        "ip":           r.get::<String, _>("ip"),
        "status":       r.get::<String, _>("status"),
    })).collect();

    Ok(Json(json!(list)))
}

async fn admin_dismiss_suggestion(
    State(st): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    sqlx::query("DELETE FROM suggestions WHERE id = ?")
        .bind(id)
        .execute(&st.pool)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(json!({ "ok": true })))
}

async fn admin_like_suggestion(
    State(st): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    sqlx::query("UPDATE suggestions SET status = CASE WHEN status = 'liked' THEN 'pending' ELSE 'liked' END WHERE id = ?")
        .bind(id)
        .execute(&st.pool)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(json!({ "ok": true })))
}

// --- Tuning Presets ---------------------------------------------------------

#[derive(Deserialize)]
struct CreatePresetReq {
    name: String,
    values: Value,
}

async fn list_tuning_presets(
    State(st): State<AppState>,
) -> Result<Json<Value>, ApiError> {
    let rows = sqlx::query("SELECT id, name, body, created_at FROM tuning_presets ORDER BY created_at ASC")
        .fetch_all(&st.pool)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let list: Vec<Value> = rows.iter().map(|r| json!({
        "id":        r.get::<i64, _>("id"),
        "name":      r.get::<String, _>("name"),
        "values":    serde_json::from_str::<Value>(&r.get::<String, _>("body")).unwrap_or(json!({})),
        "createdAt": r.get::<String, _>("created_at"),
    })).collect();

    Ok(Json(json!(list)))
}

async fn create_tuning_preset(
    State(st): State<AppState>,
    _auth: AuthUser,
    Json(req): Json<CreatePresetReq>,
) -> Result<Json<Value>, ApiError> {
    let name = req.name.trim().to_string();
    if name.is_empty() {
        return Err(err(StatusCode::UNPROCESSABLE_ENTITY, "Name is required"));
    }
    let body = req.values.to_string();

    let result = sqlx::query("INSERT INTO tuning_presets (name, body) VALUES (?, ?)")
        .bind(&name)
        .bind(&body)
        .execute(&st.pool)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let id = result.last_insert_rowid();
    let created_at: String = sqlx::query_scalar("SELECT created_at FROM tuning_presets WHERE id = ?")
        .bind(id)
        .fetch_one(&st.pool)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(json!({ "id": id, "name": name, "values": req.values, "createdAt": created_at })))
}

async fn delete_tuning_preset(
    State(st): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    sqlx::query("DELETE FROM tuning_presets WHERE id = ?")
        .bind(id)
        .execute(&st.pool)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(json!({ "ok": true })))
}

// --- Theme ------------------------------------------------------------------

const THEME_DEFAULT: &str = r##"{
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

async fn seed_theme_if_empty(pool: &SqlitePool) -> anyhow::Result<()> {
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

async fn get_theme(State(st): State<AppState>) -> Result<Json<Value>, ApiError> {
    let body: String = sqlx::query_scalar("SELECT body FROM theme WHERE id = 1")
        .fetch_optional(&st.pool)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?
        .unwrap_or_else(|| THEME_DEFAULT.to_string());
    let v: Value = serde_json::from_str(&body)
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(v))
}

async fn put_theme(
    _auth: AuthUser,
    State(st): State<AppState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let body_str = body.to_string();
    sqlx::query(
        "INSERT INTO theme (id, body) VALUES (1, ?)
         ON CONFLICT(id) DO UPDATE SET body = excluded.body"
    )
    .bind(&body_str)
    .execute(&st.pool)
    .await
    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(body))
}

// --- Cars -------------------------------------------------------------------

async fn upsert_car(pool: &SqlitePool, c: &Value) -> Result<(), sqlx::Error> {
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

fn car_row_to_json(r: &sqlx::sqlite::SqliteRow) -> Value {
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

async fn list_cars(
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

async fn create_car(
    _auth: AuthUser,
    State(st): State<AppState>,
    Json(body): Json<Value>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    upsert_car(&st.pool, &body)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok((StatusCode::CREATED, Json(body)))
}

async fn seed_cars_if_empty(pool: &SqlitePool) -> anyhow::Result<()> {
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

// --- Seed -------------------------------------------------------------------

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

/// Seed users from seed/users.json when the users table is empty (e.g. after a DB reset).
/// Skips silently if the file doesn't exist. Never overwrites existing users.
async fn seed_users_if_empty(pool: &SqlitePool, seed_path: &str) -> anyhow::Result<()> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await?;
    if count > 0 {
        return Ok(());
    }
    let Ok(raw) = std::fs::read_to_string(seed_path) else {
        tracing::warn!("no user seed at {seed_path}; starting with no users (run `adduser` to create one)");
        return Ok(());
    };
    let users: Vec<Value> = serde_json::from_str(&raw)?;
    for u in &users {
        let username = u.get("username").and_then(Value::as_str).unwrap_or_default();
        let hash = u.get("password_hash").and_then(Value::as_str).unwrap_or_default();
        if username.is_empty() || hash.is_empty() { continue; }
        sqlx::query("INSERT OR IGNORE INTO users (username, password_hash) VALUES (?, ?)")
            .bind(username)
            .bind(hash)
            .execute(pool)
            .await?;
    }
    tracing::info!("seeded {} user(s) from {seed_path}", users.len());
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
/// Fetch images for a card from the authoritative images table.
async fn fetch_images_for_card(pool: &SqlitePool, card_id: &str) -> Vec<Value> {
    let rows = sqlx::query(
        "SELECT id, path, thumb_path, stage_path, alt_text, sort_order, car_id, livery_id \
         FROM images WHERE card_id = ? ORDER BY sort_order ASC",
    )
    .bind(card_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    rows.iter().map(|r| json!({
        "id":        r.get::<i64, _>("id"),
        "path":      r.get::<String, _>("path"),
        "thumbPath": r.get::<Option<String>, _>("thumb_path"),
        "stagePath": r.get::<Option<String>, _>("stage_path"),
        "alt":       r.get::<Option<String>, _>("alt_text").unwrap_or_default(),
        "order":     r.get::<i64, _>("sort_order"),
        "carId":     r.get::<Option<String>, _>("car_id"),
        "liveryId":  r.get::<Option<i64>, _>("livery_id"),
    })).collect()
}

/// Replace body["images"] with rows from the images table.
/// Falls back to body["images"] as-is when no DB rows exist (unmigrated card).
async fn inject_images(pool: &SqlitePool, body: &mut Value) {
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
async fn sync_card_images(pool: &SqlitePool, card_id: &str, body: &mut Value) -> Result<(), sqlx::Error> {
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

        let final_id: i64 = if let Some(id) = db_id {
            sqlx::query(
                "UPDATE images SET alt_text = ?, sort_order = ?, car_id = ?, livery_id = COALESCE(?, livery_id) WHERE id = ? AND card_id = ?",
            )
            .bind(&alt).bind(order).bind(&car_id).bind(livery_id).bind(id).bind(card_id)
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
                    "UPDATE images SET alt_text = ?, sort_order = ?, car_id = ? WHERE id = ?",
                )
                .bind(&alt).bind(order).bind(&car_id).bind(existing_id)
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

        synced.push(json!({ "id": final_id, "alt": alt, "order": order, "carId": car_id }));
    }

    body["images"] = json!(synced);
    Ok(())
}

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
        // Step 3: sync body images to images table, strip paths from body.
        let card_id = v.get("id").and_then(Value::as_str).map(String::from);
        if let Some(card_id) = card_id {
            let has_path_images = v.get("images")
                .and_then(Value::as_array)
                .map(|arr| arr.iter().any(|img| img.get("path").is_some()))
                .unwrap_or(false);
            if has_path_images {
                if sync_card_images(pool, &card_id, &mut v).await.is_ok() {
                    changed = true;
                }
            }
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

// --- Admin ------------------------------------------------------------------

/// Recursively collect all file paths under a directory.
fn walk_files(dir: &std::path::Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() { walk_files(&path, out); } else { out.push(path); }
    }
}

/// Collect paths referenced in the images table (relative to uploads_dir, forward-slash separated).
async fn referenced_paths(pool: &SqlitePool) -> Result<std::collections::HashSet<String>, ApiError> {
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

async fn admin_stats(
    _auth: AuthUser,
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

async fn admin_scan_orphans(
    _auth: AuthUser,
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

    Ok(Json(json!({ "count": orphan_paths.len(), "paths": orphan_paths })))
}

async fn admin_delete_orphans(
    _auth: AuthUser,
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

    Ok(Json(json!({ "moved": moved })))
}

async fn admin_reload_seed(
    _auth: AuthUser,
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

async fn admin_export_seed(
    _auth: AuthUser,
    State(st): State<AppState>,
) -> Result<Json<Value>, ApiError> {
    let rows = sqlx::query("SELECT body FROM cards ORDER BY catalog_number")
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

async fn list_cards(State(st): State<AppState>) -> Result<Json<Vec<Value>>, ApiError> {
    let rows = sqlx::query("SELECT body FROM cards ORDER BY catalog_number")
        .fetch_all(&st.pool)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let mut out = Vec::new();
    for row in &rows {
        if let Ok(mut v) = serde_json::from_str::<Value>(row.get::<String, _>("body").as_str()) {
            inject_images(&st.pool, &mut v).await;
            out.push(v);
        }
    }
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
    let mut body: Value = serde_json::from_str(row.get::<String, _>("body").as_str())
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    inject_images(&st.pool, &mut body).await;
    Ok(Json(body))
}

async fn put_card(
    State(st): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<String>,
    Json(mut body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
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

    sync_card_images(&st.pool, &id, &mut body)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    upsert(&st.pool, &body)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    inject_images(&st.pool, &mut body).await;
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
    Json(mut body): Json<Value>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let card_id = body.get("id").and_then(Value::as_str).unwrap_or_default().to_string();
    if card_id.is_empty() {
        return Err(err(StatusCode::BAD_REQUEST, "body.id is required"));
    }
    sync_card_images(&st.pool, &card_id, &mut body)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    upsert(&st.pool, &body)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    inject_images(&st.pool, &mut body).await;
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

/// Card folder: {card_name_slug}_{card_id}  e.g. smokin_abc123
/// Falls back to "misc" when either field is absent.
fn card_folder(name: &str, card_id: &str) -> String {
    let slug = slugify(name);
    match (slug.is_empty(), card_id.is_empty()) {
        (false, false) => format!("{slug}_{card_id}"),
        (false, true)  => slug,
        (true,  false) => format!("card_{card_id}"),
        (true,  true)  => "misc".into(),
    }
}

/// Move a single file to the trash directory. Returns the new basename in trash.
fn move_to_trash(src: &std::path::Path, trash_dir: &std::path::Path) -> Option<String> {
    if !src.exists() { return None; }
    let original_name = src.file_name()?.to_str()?;
    let uuid = uuid::Uuid::new_v4().to_string().replace('-', "");
    let trash_name = format!("{}_{original_name}", &uuid[..8]);
    let dest = trash_dir.join(&trash_name);
    std::fs::rename(src, dest).ok()?;
    Some(trash_name)
}

/// Move an image (and its thumb/stage variants) to trash, log the event, and
/// remove the images table row. Called when an image is explicitly removed from
/// a card (reason = 'user_delete') or auto-detected as orphaned (reason = 'orphan').
async fn trash_image(
    pool: &SqlitePool,
    uploads_dir: &std::path::Path,
    trash_dir: &std::path::Path,
    path_str: &str,
    reason: &str,
) -> bool {
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
async fn delete_images(
    State(st): State<AppState>,
    _auth: AuthUser,
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
        trash_image(&st.pool, &st.uploads_dir, &trash_dir, path_str, "user_delete").await;
    }
    StatusCode::NO_CONTENT
}

// --- Admin: trash management ------------------------------------------------

async fn admin_list_trash(
    _auth: AuthUser,
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
struct DeleteTrashReq {
    ids: Option<Vec<i64>>,
    all: Option<bool>,
    unknown: Option<bool>, // also wipe unlogged files
}

async fn admin_delete_trash(
    _auth: AuthUser,
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
struct RestoreTrashReq {
    ids: Vec<i64>,
}

async fn admin_restore_trash(
    _auth: AuthUser,
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

/// Build a structured image stem from car + livery DB data.
/// Pattern: {GAME}_{make}_{model}_{year}_{livery}_{NNN}_{YYYYMMDD}_{uuid6}
/// Falls back to a UUID stem when car data is absent.
async fn build_image_stem(
    pool: &sqlx::SqlitePool,
    card_id: &str,
    car_id: &Option<String>,
    livery_id: Option<i64>,
    file_index: Option<u32>,
) -> String {
    let uuid_full = uuid::Uuid::new_v4().to_string();
    let uuid6 = uuid_full.replace('-', "");
    let uuid6 = &uuid6[..6];

    // Date from SQLite so we don't need chrono.
    let date: String = sqlx::query_scalar("SELECT strftime('%Y%m%d', 'now')")
        .fetch_one(pool).await.unwrap_or_else(|_| "00000000".into());

    let Some(cid) = car_id else {
        // No car — simple fallback.
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
async fn upload_image(
    State(st): State<AppState>,
    _auth: AuthUser,
    mut multipart: Multipart,
) -> Result<Json<Value>, ApiError> {
    let mut card_name = String::new();
    let mut card_id = String::new();
    let mut car_id: Option<String> = None;
    let mut livery_id: Option<i64> = None;
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
        let stem = build_image_stem(&st.pool, &card_id, &car_id, livery_id, file_index).await;

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
        let db_id: Option<i64> = if !card_id.is_empty() {
            let result = sqlx::query(
                "INSERT INTO images (card_id, path, thumb_path, stage_path, car_id, livery_id) VALUES (?, ?, ?, ?, ?, ?)",
            )
            .bind(&card_id)
            .bind(&path)
            .bind(&thumb_path)
            .bind(&stage_path)
            .bind(&car_id)
            .bind(&livery_id)
            .execute(&st.pool)
            .await;
            result.ok().map(|r| r.last_insert_rowid())
        } else {
            None
        };

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

// --- AI color assessment ----------------------------------------------------

const COLOR_TAXONOMY: &[&str] = &[
    "Red", "Blue", "Green", "Yellow", "Orange", "Purple", "Pink",
    "White", "Black", "Silver", "Grey", "Gold", "Bronze", "Teal", "Multi",
];

async fn admin_assess_livery_color(
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
async fn admin_repair_figure_paths(
    _auth: AuthUser,
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

            // Stale — find the card's lead image from the images table.
            let img = sqlx::query(
                "SELECT stage_path, path FROM images WHERE card_id = ? ORDER BY sort_order ASC LIMIT 1",
            )
            .bind(&card_id)
            .fetch_optional(&st.pool)
            .await
            .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;

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

async fn admin_migrate_images(
    _auth: AuthUser,
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

        let stem = build_image_stem(&st.pool, &card_id, &car_id, livery_id, None).await;

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

// --- Serial generator -------------------------------------------------------

/// Returns the next L### serial for a car, e.g. "FH6-NISRVGTSP99-L003".
/// Reads MAX of existing serials for that car rather than COUNT so deletions
/// don't cause collisions.
async fn next_livery_serial(pool: &SqlitePool, car_id: &str) -> Result<String, sqlx::Error> {
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
async fn next_tune_serial(pool: &SqlitePool, livery_id: i64) -> Result<String, sqlx::Error> {
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

async fn list_tune_types(
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
struct CreateTuneTypeReq {
    name: String,
    #[serde(rename = "sortOrder")]
    sort_order: Option<i64>,
}

async fn create_tune_type(
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

fn livery_row_to_json(r: &sqlx::sqlite::SqliteRow) -> Value {
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

async fn list_liveries(
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
struct CreateLiveryReq {
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

async fn create_livery(
    _auth: AuthUser,
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

    Ok((StatusCode::CREATED, Json(json!({ "id": id, "serial": serial }))))
}

#[derive(Deserialize)]
struct UpdateLiveryReq {
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

async fn update_livery(
    _auth: AuthUser,
    State(st): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateLiveryReq>,
) -> Result<Json<Value>, ApiError> {
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
    Ok(Json(json!({ "id": id })))
}

async fn delete_livery(
    _auth: AuthUser,
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

fn tune_row_to_json(r: &sqlx::sqlite::SqliteRow) -> Value {
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

async fn list_tunes(
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
struct CreateTuneReq {
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

async fn create_tune(
    _auth: AuthUser,
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

    Ok((StatusCode::CREATED, Json(json!({ "id": id, "serial": serial }))))
}

#[derive(Deserialize)]
struct UpdateTuneReq {
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

async fn update_tune(
    _auth: AuthUser,
    State(st): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateTuneReq>,
) -> Result<Json<Value>, ApiError> {
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
    Ok(Json(json!({ "id": id })))
}

async fn delete_tune(
    _auth: AuthUser,
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
