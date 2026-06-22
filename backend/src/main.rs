//! Livery Catalog API — Axum + SQLite (single-user, no auth).
//!
//! Routes:
//!   GET    /api/liveries        -> [Livery]   (ordered by catalogNumber)
//!   GET    /api/liveries/:id    -> Livery
//!   PUT    /api/liveries/:id    -> Livery     (whole-object upsert)
//!   POST   /api/liveries        -> Livery     (create; body must include id)
//!   DELETE /api/liveries/:id    -> 204
//!   POST   /api/images          -> { path }   (multipart upload to uploads/)
//!   GET    /uploads/*           -> static files
//!   GET    /api/health          -> "ok"

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
    let seed_path = std::env::var("SEED_PATH").unwrap_or_else(|_| "seed/liveries.json".into());
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

    let state = AppState { pool, uploads_dir: uploads_dir.clone() };

    // Serve the built SPA: real files (index.html at "/", hashed assets) are
    // served directly; any other path falls back to index.html. (This app has no
    // client-side router, so "/" is the only real entry point.)
    let spa = ServeDir::new(&frontend_dir)
        .not_found_service(ServeFile::new(format!("{frontend_dir}/index.html")));

    let app = Router::new()
        .route("/api/health", get(|| async { "ok" }))
        .route("/api/liveries", get(list_liveries).post(create_livery))
        .route(
            "/api/liveries/:id",
            get(get_livery).put(put_livery).delete(delete_livery),
        )
        .route("/api/images", post(upload_image))
        .nest_service("/uploads", ServeDir::new(uploads_dir))
        .fallback_service(spa)
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("livery-backend listening on http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}

/// Import seed/liveries.json on first run (when the table is empty).
async fn seed_if_empty(pool: &SqlitePool, seed_path: &str) -> anyhow::Result<()> {
    let count: i64 = sqlx::query("SELECT COUNT(*) AS c FROM liveries")
        .fetch_one(pool)
        .await?
        .get("c");
    if count > 0 {
        tracing::info!("db already has {count} liveries; skipping seed");
        return Ok(());
    }
    let Ok(raw) = std::fs::read_to_string(seed_path) else {
        tracing::warn!("no seed file at {seed_path}; starting empty");
        return Ok(());
    };
    let liveries: Vec<Value> = serde_json::from_str(&raw)?;
    for l in &liveries {
        upsert(pool, l).await?;
    }
    tracing::info!("seeded {} liveries from {seed_path}", liveries.len());
    Ok(())
}

/// Insert-or-replace a livery row from its JSON object.
async fn upsert(pool: &SqlitePool, body: &Value) -> Result<(), sqlx::Error> {
    let id = body.get("id").and_then(Value::as_str).unwrap_or_default().to_string();
    let catalog_number = body.get("catalogNumber").and_then(Value::as_i64).unwrap_or(0);
    let body_str = body.to_string();
    sqlx::query(
        "INSERT INTO liveries (id, catalog_number, body) VALUES (?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET catalog_number = excluded.catalog_number, body = excluded.body",
    )
    .bind(&id)
    .bind(catalog_number)
    .bind(&body_str)
    .execute(pool)
    .await?;
    Ok(())
}

async fn list_liveries(State(st): State<AppState>) -> Result<Json<Vec<Value>>, ApiError> {
    let rows = sqlx::query("SELECT body FROM liveries ORDER BY catalog_number")
        .fetch_all(&st.pool)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let out = rows
        .iter()
        .filter_map(|r| serde_json::from_str::<Value>(r.get::<String, _>("body").as_str()).ok())
        .collect();
    Ok(Json(out))
}

async fn get_livery(
    State(st): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let row = sqlx::query("SELECT body FROM liveries WHERE id = ?")
        .bind(&id)
        .fetch_optional(&st.pool)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "livery not found"))?;
    let body: Value = serde_json::from_str(row.get::<String, _>("body").as_str())
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(body))
}

async fn put_livery(
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

async fn create_livery(
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

async fn delete_livery(
    State(st): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    sqlx::query("DELETE FROM liveries WHERE id = ?")
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
