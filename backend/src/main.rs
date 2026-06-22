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

use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde_json::{json, Value};
use sqlx::{sqlite::SqliteConnectOptions, Row, SqlitePool};
use tower_http::{
    cors::CorsLayer,
    services::{ServeDir, ServeFile},
};

#[derive(Clone)]
struct AppState {
    pool: SqlitePool,
    uploads_dir: PathBuf,
}

type ApiError = (StatusCode, String);

fn err(code: StatusCode, msg: impl ToString) -> ApiError {
    (code, msg.to_string())
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
    seed_if_empty(&pool, &seed_path).await?;
    normalize_bodies(&pool).await?;

    let state = AppState { pool, uploads_dir: uploads_dir.clone() };

    // Serve the built SPA: real files (index.html at "/", hashed assets) are
    // served directly; any other path falls back to index.html. (This app has no
    // client-side router, so "/" is the only real entry point.)
    let spa = ServeDir::new(&frontend_dir)
        .not_found_service(ServeFile::new(format!("{frontend_dir}/index.html")));

    let app = Router::new()
        .route("/api/health", get(|| async { "ok" }))
        .route("/api/cards", get(list_cards).post(create_card))
        .route(
            "/api/cards/:id",
            get(get_card).put(put_card).delete(delete_card),
        )
        .route("/api/images", post(upload_image))
        .nest_service("/uploads", ServeDir::new(uploads_dir))
        .fallback_service(spa)
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
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    sqlx::query("DELETE FROM cards WHERE id = ?")
        .bind(&id)
        .execute(&st.pool)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(StatusCode::NO_CONTENT)
}

/// Accept a single multipart file field, store it under uploads/, return its URL path.
async fn upload_image(
    State(st): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<Value>, ApiError> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| err(StatusCode::BAD_REQUEST, e))?
    {
        let orig = field.file_name().unwrap_or("upload").to_string();
        let ext = std::path::Path::new(&orig)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("png")
            .to_lowercase();
        let data = field
            .bytes()
            .await
            .map_err(|e| err(StatusCode::BAD_REQUEST, e))?;
        let filename = format!("{}.{}", uuid::Uuid::new_v4(), ext);
        let dest = st.uploads_dir.join(&filename);
        std::fs::write(&dest, &data).map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
        return Ok(Json(json!({ "path": format!("/uploads/{filename}") })));
    }
    Err(err(StatusCode::BAD_REQUEST, "no file field in upload"))
}
