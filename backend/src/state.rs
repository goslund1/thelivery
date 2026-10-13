//! Shared application state and error helpers.

use std::path::PathBuf;
use std::sync::Arc;

use axum::http::StatusCode;
use sqlx::SqlitePool;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub uploads_dir: PathBuf,
    pub seed_path: PathBuf,
    pub db_path: PathBuf,
    pub jwt_secret: Arc<Vec<u8>>,
    /// Canonical base URL e.g. "https://thelivery.silverleaf.services".
    /// Set via SITE_URL env var; used for sitemap, robots.txt, and attribution.
    /// Falls back to request Host header when empty.
    pub site_url: String,
    /// Shakedown's live channel: every change, send and run, to every page
    /// following (src/shakedown.rs).
    pub shakedown: crate::shakedown::Live,
}

pub type ApiError = (StatusCode, String);

pub fn err(code: StatusCode, msg: impl ToString) -> ApiError {
    (code, msg.to_string())
}
