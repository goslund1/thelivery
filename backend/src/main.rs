//! Card Catalog API — Axum + SQLite.
//!
//! Thin entry point: config from env, DB setup + seeding, router assembly.
//! The handlers live in the modules below, split along domain lines:
//!
//!   state       — AppState + error helpers
//!   auth        — JWT, login, users, `adduser` CLI
//!   cards       — card CRUD, history, seeding, body normalization
//!   images      — images table sync/inject, upload, naming, migration
//!   trash       — orphan scan, trash pipeline, stats
//!   identity    — cars registry, liveries, tunes, serials, AI color assess
//!   share       — OG share page, card PNG, compositor preview, OG presets
//!   suggestions — public tune suggestions + admin review
//!   presets     — tuning presets
//!   theme       — site theme
//!   compositor  — OG image rendering

mod audit;
mod auth;
mod cards;
mod compositor;
mod identity;
mod images;
mod presets;
mod share;
mod state;
mod suggestions;
#[cfg(test)]
mod testutil;
mod theme;
mod trash;

use std::path::PathBuf;

use axum::{
    extract::{DefaultBodyLimit, Request},
    http::{header::CACHE_CONTROL, HeaderName, HeaderValue},
    middleware::{self, Next},
    response::Response,
    routing::{delete, get, post, put},
    Router,
};
use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};
use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    services::{ServeDir, ServeFile},
    set_header::SetResponseHeaderLayer,
};

use state::AppState;

/// Cache-Control for the built SPA. Vite fingerprints everything under
/// /assets/, so those can be cached forever; everything else the SPA service
/// returns is index.html (either directly or as the fallback) and must be
/// revalidated on every load, otherwise browsers heuristic-cache it and keep
/// referencing bundles that no longer exist after the next deploy.
async fn spa_cache_control(req: Request, next: Next) -> Response {
    let immutable = req.uri().path().starts_with("/assets/");
    let mut res = next.run(req).await;
    let value = if immutable {
        HeaderValue::from_static("public, max-age=31536000, immutable")
    } else {
        HeaderValue::from_static("no-cache")
    };
    res.headers_mut().insert(CACHE_CONTROL, value);
    res
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok(); // load .env if present; ignore if missing
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

    // CLI: `livery-backend adduser <username> [admin|editor]` — create a user, then exit.
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("adduser") {
        let Some(username) = args.get(2) else {
            eprintln!("usage: livery-backend adduser <username> [admin|editor]");
            std::process::exit(2);
        };
        let role = args.get(3).map(String::as_str).unwrap_or("admin");
        auth::add_user(&pool, username, role).await?;
        return Ok(());
    }

    cards::seed_if_empty(&pool, &seed_path).await?;
    auth::seed_users_if_empty(&pool, "seed/users.json").await?;
    theme::seed_theme_if_empty(&pool).await?;
    identity::seed_cars_if_empty(&pool).await?;
    // Must run before normalize_bodies: seeded card images reference liveries
    // by id, and the images.livery_id FK needs those rows to exist.
    identity::seed_liveries_if_empty(&pool, "seed/liveries.json").await?;
    cards::normalize_bodies(&pool).await?;

    let state = AppState {
        pool,
        uploads_dir: uploads_dir.clone(),
        seed_path: PathBuf::from(&seed_path),
        db_path: PathBuf::from(&db_path),
        jwt_secret: auth::load_jwt_secret(),
    };

    // Serve the built SPA: real files (index.html at "/", hashed assets) are
    // served directly; any other path falls back to index.html. (This app has no
    // client-side router, so "/" is the only real entry point.)
    // Cache policy: index.html must always revalidate (no Cache-Control at all
    // lets browsers heuristic-cache it and keep loading old hashed bundles after
    // a deploy), while the content-hashed /assets/* files are safe to cache
    // forever.
    let spa = ServiceBuilder::new()
        .layer(middleware::from_fn(spa_cache_control))
        .service(
            ServeDir::new(&frontend_dir)
                .not_found_service(ServeFile::new(format!("{frontend_dir}/index.html"))),
        );

    let app = Router::new()
        .route("/api/health", get(|| async { "ok" }))
        .route("/api/login", post(auth::login))
        .route("/api/users", post(auth::create_user))
        .route("/api/me/password", put(auth::change_password))
        .route("/api/cards", get(cards::list_cards).post(cards::create_card))
        .route(
            "/api/cards/:id",
            get(cards::get_card).put(cards::put_card).delete(cards::delete_card),
        )
        .route("/api/cards/:id/history", get(cards::list_card_history))
        .route("/api/cards/:id/history/:version", get(cards::get_card_history_version))
        .route("/api/images", post(images::upload_image).delete(trash::delete_images))
        .route("/api/admin/stats", get(trash::admin_stats))
        .route("/api/admin/orphans", get(trash::admin_scan_orphans).delete(trash::admin_delete_orphans))
        .route("/api/admin/trash", get(trash::admin_list_trash).delete(trash::admin_delete_trash))
        .route("/api/admin/trash/restore", post(trash::admin_restore_trash))
        .route("/api/admin/export-seed", post(cards::admin_export_seed))
        .route("/api/admin/reload-seed", post(cards::admin_reload_seed))
        .route("/api/suggestions", post(suggestions::submit_suggestion))
        .route("/api/admin/suggestions", get(suggestions::admin_list_suggestions))
        .route("/api/admin/suggestions/:id", delete(suggestions::admin_dismiss_suggestion).patch(suggestions::admin_like_suggestion))
        .route("/api/admin/liveries/:id/assess-color", post(identity::admin_assess_livery_color))
        .route("/api/admin/images/migrate", post(images::admin_migrate_images))
        .route("/api/admin/repair-figure-paths", post(images::admin_repair_figure_paths))
        .route("/api/admin/deleted-cards", get(cards::admin_list_deleted_cards))
        .route("/api/admin/deleted-cards/:id/restore", post(cards::admin_restore_card))
        .route("/api/admin/deleted-cards/:id", delete(cards::admin_purge_card))
        .route("/api/admin/audit", get(audit::admin_list_audit))
        .route("/share/:id/card.png", get(share::share_card_png))
        .route("/share/preview", post(share::share_preview))
        .route("/share/:id", get(share::share_card))
        .route("/api/cars", get(identity::list_cars).post(identity::create_car))
        .route("/api/tune-types", get(identity::list_tune_types).post(identity::create_tune_type))
        .route("/api/liveries", get(identity::list_liveries).post(identity::create_livery))
        .route("/api/liveries/:id", put(identity::update_livery).delete(identity::delete_livery))
        .route("/api/tunes", get(identity::list_tunes).post(identity::create_tune))
        .route("/api/tunes/:id", put(identity::update_tune).delete(identity::delete_tune))
        .route("/api/theme", get(theme::get_theme).put(theme::put_theme))
        .route("/api/tuning-presets", get(presets::list_tuning_presets).post(presets::create_tuning_preset))
        .route("/api/tuning-presets/:id", delete(presets::delete_tuning_preset))
        .route("/api/og-presets", get(share::list_og_presets).post(share::create_og_preset))
        .route("/api/og-presets/:id", put(share::update_og_preset).delete(share::delete_og_preset))
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
