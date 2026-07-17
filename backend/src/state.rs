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
}

pub type ApiError = (StatusCode, String);

pub fn err(code: StatusCode, msg: impl ToString) -> ApiError {
    (code, msg.to_string())
}
