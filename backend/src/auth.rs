//! Authentication: JWT issue/verify, the AuthUser extractor, login,
//! user management, user seeding, and the `adduser` CLI.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use argon2::password_hash::rand_core::{OsRng, RngCore};
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use axum::{
    async_trait,
    extract::{FromRequestParts, State},
    http::{header::AUTHORIZATION, request::Parts, StatusCode},
    Json,
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{Row, SqlitePool};

use crate::state::{err, ApiError, AppState};

// --- Authentication ---------------------------------------------------------

#[derive(Serialize, Deserialize)]
pub struct Claims {
    sub: String, // username
    exp: usize,  // expiry (unix seconds)
}

pub const TOKEN_TTL_SECS: u64 = 7 * 24 * 3600; // 7 days

pub fn make_token(username: &str, secret: &[u8]) -> anyhow::Result<String> {
    let exp = (SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + TOKEN_TTL_SECS) as usize;
    let claims = Claims { sub: username.to_string(), exp };
    Ok(encode(&Header::default(), &claims, &EncodingKey::from_secret(secret))?)
}

// JWT signing secret: from JWT_SECRET in prod; a random ephemeral secret in dev
// (so we never ship a known-weak default). Ephemeral secrets reset tokens on
// restart, which is fine for dev — set JWT_SECRET in production.
pub fn load_jwt_secret() -> Arc<Vec<u8>> {
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
pub struct AuthUser(#[allow(dead_code)] String);

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
pub struct LoginReq {
    username: String,
    password: String,
}

/// Verify credentials against the users table and issue a JWT. Uses a generic
/// "invalid credentials" error for both unknown users and bad passwords.
pub async fn login(State(st): State<AppState>, Json(req): Json<LoginReq>) -> Result<Json<Value>, ApiError> {
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
pub struct CreateUserReq {
    username: String,
    password: String,
}

/// Create a new user. Requires a valid JWT so only existing users can invite others.
pub async fn create_user(
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
pub struct ChangePasswordReq {
    current_password: String,
    new_password: String,
}

/// Change the calling user's own password. Requires the current password to confirm.
pub async fn change_password(
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
pub async fn add_user(pool: &SqlitePool, username: &str) -> anyhow::Result<()> {
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

/// Seed users from seed/users.json when the users table is empty (e.g. after a DB reset).
/// Skips silently if the file doesn't exist. Never overwrites existing users.
pub async fn seed_users_if_empty(pool: &SqlitePool, seed_path: &str) -> anyhow::Result<()> {
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
