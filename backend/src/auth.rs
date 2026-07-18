//! Authentication: JWT issue/verify, the AuthUser extractor, login,
//! user management, user seeding, and the `adduser` CLI.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use argon2::password_hash::rand_core::{OsRng, RngCore};
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use axum::{
    async_trait,
    extract::{ConnectInfo, FromRequestParts, State},
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
/// expired tokens. The role is read from the DB on every request (not from the
/// token), so demoting or deleting a user takes effect immediately instead of
/// waiting out the 7-day token TTL.
pub struct AuthUser {
    pub username: String,
    pub role: String,
}

impl AuthUser {
    pub fn is_admin(&self) -> bool {
        self.role == "admin"
    }
}

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
        let username = data.claims.sub;
        let row = sqlx::query("SELECT role, must_change_password FROM users WHERE username = ?")
            .bind(&username)
            .fetch_optional(&state.pool)
            .await
            .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?
            .ok_or_else(|| err(StatusCode::UNAUTHORIZED, "user no longer exists"))?;
        let role: String = row.get("role");
        let must_change: bool = row.get::<i64, _>("must_change_password") != 0;
        // A temp password only buys you the change-password endpoint — nothing
        // else works until the user sets a real one.
        if must_change && parts.uri.path() != "/api/me/password" {
            return Err(err(StatusCode::FORBIDDEN, "password change required"));
        }
        Ok(AuthUser { username, role })
    }
}

/// Extractor for admin-only endpoints: everything permanent (hard deletes,
/// trash purge, seed reload) and user management. Editors get 403.
pub struct AdminUser(pub AuthUser);

#[async_trait]
impl FromRequestParts<AppState> for AdminUser {
    type Rejection = ApiError;
    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let user = AuthUser::from_request_parts(parts, state).await?;
        if !user.is_admin() {
            return Err(err(StatusCode::FORBIDDEN, "admin access required"));
        }
        Ok(AdminUser(user))
    }
}

#[derive(Deserialize)]
pub struct LoginReq {
    username: String,
    password: String,
}

/// Failed login attempts allowed per anonymized IP before blocking.
pub const LOGIN_BURST_LIMIT: i64 = 5; // per 5 minutes
pub const LOGIN_HOURLY_LIMIT: i64 = 20; // per hour

/// Count recent failed logins for an anonymized IP within the given SQLite
/// datetime modifier window (e.g. "-5 minutes").
pub async fn failed_logins_since(pool: &SqlitePool, ip_hash: &str, window: &str) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM login_rate_log WHERE ip_hash = ? AND attempted_at > datetime('now', ?)",
    )
    .bind(ip_hash)
    .bind(window)
    .fetch_one(pool)
    .await
}

/// Verify credentials against the users table and issue a JWT. Uses a generic
/// "invalid credentials" error for both unknown users and bad passwords.
/// Failed attempts are rate-limited per anonymized IP (DB-backed, mirrors
/// suggestion_rate_log); successful logins don't count against the limit.
pub async fn login(
    State(st): State<AppState>,
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    headers: axum::http::HeaderMap,
    Json(req): Json<LoginReq>,
) -> Result<Json<Value>, ApiError> {
    let ip_hash = crate::suggestions::anonymize_ip(&crate::suggestions::client_ip(&headers, &addr));

    let burst = failed_logins_since(&st.pool, &ip_hash, "-5 minutes")
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let hourly = failed_logins_since(&st.pool, &ip_hash, "-1 hour")
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    if burst >= LOGIN_BURST_LIMIT || hourly >= LOGIN_HOURLY_LIMIT {
        return Err(err(StatusCode::TOO_MANY_REQUESTS, "too many failed login attempts — try again later"));
    }

    let row = sqlx::query("SELECT password_hash, role, must_change_password FROM users WHERE username = ?")
        .bind(&req.username)
        .fetch_optional(&st.pool)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let user_known = row.is_some();
    let verified = row.and_then(|r| {
        let role: String = r.get("role");
        let must_change: bool = r.get::<i64, _>("must_change_password") != 0;
        PasswordHash::new(&r.get::<String, _>("password_hash")).ok().and_then(|ph| {
            Argon2::default()
                .verify_password(req.password.as_bytes(), &ph)
                .is_ok()
                .then_some((role, must_change))
        })
    });
    let Some((role, must_change)) = verified else {
        if !user_known {
            // Burn the same Argon2 work an unknown username skipped, so
            // response timing can't distinguish "no such user" from
            // "wrong password" (username enumeration).
            let salt = SaltString::generate(&mut OsRng);
            let _ = Argon2::default().hash_password(req.password.as_bytes(), &salt);
        }
        let _ = sqlx::query("INSERT INTO login_rate_log (ip_hash) VALUES (?)")
            .bind(&ip_hash)
            .execute(&st.pool)
            .await;
        // Piggyback pruning on failures — rows older than the widest rate
        // window (1 hour) can never affect a limit check again.
        let _ = sqlx::query("DELETE FROM login_rate_log WHERE attempted_at < datetime('now', '-1 day')")
            .execute(&st.pool)
            .await;
        return Err(err(StatusCode::UNAUTHORIZED, "invalid credentials"));
    };
    let token = make_token(&req.username, &st.jwt_secret)
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(json!({
        "token": token,
        "username": req.username,
        "role": role,
        "mustChangePassword": must_change,
    })))
}

#[derive(Deserialize)]
pub struct CreateUserReq {
    username: String,
    password: String,
    role: Option<String>,
    #[serde(rename = "mustChangePassword")]
    must_change_password: Option<bool>,
}

/// List all users. Admin-only, read-only — lets an admin see what accounts
/// exist; `mustChangePassword` doubles as a "hasn't signed in yet" indicator
/// for temp-password accounts.
pub async fn list_users(
    _admin: AdminUser,
    State(st): State<AppState>,
) -> Result<Json<Value>, ApiError> {
    let rows = sqlx::query(
        "SELECT username, role, must_change_password, created_at FROM users ORDER BY created_at",
    )
    .fetch_all(&st.pool)
    .await
    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let users: Vec<Value> = rows
        .iter()
        .map(|r| {
            json!({
                "username": r.get::<String, _>("username"),
                "role": r.get::<String, _>("role"),
                "mustChangePassword": r.get::<i64, _>("must_change_password") != 0,
                "createdAt": r.get::<String, _>("created_at"),
            })
        })
        .collect();
    Ok(Json(json!(users)))
}

/// Create a new user. Admin-only — otherwise an editor could mint themselves a
/// fresh admin account. New users default to 'editor' unless a role is given.
pub async fn create_user(
    admin: AdminUser,
    State(st): State<AppState>,
    Json(req): Json<CreateUserReq>,
) -> Result<Json<Value>, ApiError> {
    let username = req.username.trim();
    if username.is_empty() || username.len() > 64 {
        return Err(err(StatusCode::BAD_REQUEST, "username must be 1-64 characters"));
    }
    if req.password.len() < 8 {
        return Err(err(StatusCode::BAD_REQUEST, "password must be at least 8 characters"));
    }
    let role = req.role.as_deref().unwrap_or("editor");
    if !matches!(role, "admin" | "editor") {
        return Err(err(StatusCode::BAD_REQUEST, "role must be 'admin' or 'editor'"));
    }
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(req.password.as_bytes(), &salt)
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?
        .to_string();
    sqlx::query("INSERT INTO users (username, password_hash, role, must_change_password) VALUES (?, ?, ?, ?)")
        .bind(username)
        .bind(&hash)
        .bind(role)
        .bind(req.must_change_password.unwrap_or(false))
        .execute(&st.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("UNIQUE") {
                err(StatusCode::CONFLICT, "username already exists")
            } else {
                err(StatusCode::INTERNAL_SERVER_ERROR, e)
            }
        })?;
    crate::audit::record(
        &st.pool, &admin.0.username, "user.create", "user", Some(username),
        Some(json!({ "role": role, "mustChangePassword": req.must_change_password.unwrap_or(false) })),
    ).await;
    Ok(Json(json!({ "username": username, "role": role })))
}

#[derive(Deserialize)]
pub struct ChangePasswordReq {
    current_password: String,
    new_password: String,
}

/// Change the calling user's own password. Requires the current password to confirm.
pub async fn change_password(
    AuthUser { username, .. }: AuthUser,
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
    sqlx::query("UPDATE users SET password_hash = ?, must_change_password = 0 WHERE username = ?")
        .bind(&new_hash)
        .bind(&username)
        .execute(&st.pool)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(json!({ "ok": true })))
}

/// `adduser` CLI: prompt for a password and insert an Argon2-hashed user.
/// Role defaults to 'admin' (matching historical behavior of CLI-created users).
pub async fn add_user(pool: &SqlitePool, username: &str, role: &str) -> anyhow::Result<()> {
    if !matches!(role, "admin" | "editor") {
        anyhow::bail!("role must be 'admin' or 'editor'");
    }
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
    match sqlx::query("INSERT INTO users (username, password_hash, role) VALUES (?, ?, ?)")
        .bind(username)
        .bind(&hash)
        .bind(role)
        .execute(pool)
        .await
    {
        Ok(_) => {
            println!("✓ user '{username}' created ({role})");
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
        let role = u.get("role").and_then(Value::as_str).unwrap_or("admin");
        if username.is_empty() || hash.is_empty() { continue; }
        sqlx::query("INSERT OR IGNORE INTO users (username, password_hash, role) VALUES (?, ?, ?)")
            .bind(username)
            .bind(hash)
            .bind(role)
            .execute(pool)
            .await?;
    }
    tracing::info!("seeded {} user(s) from {seed_path}", users.len());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn seed_users_reads_role_and_defaults_to_admin() {
        let pool = crate::testutil::test_pool().await;
        let seed = serde_json::json!([
            { "username": "jason", "password_hash": "$argon2$fake" },
            { "username": "geoff", "password_hash": "$argon2$fake", "role": "editor" }
        ]);
        let path = std::env::temp_dir().join("seed_users_roles_test.json");
        std::fs::write(&path, seed.to_string()).unwrap();
        seed_users_if_empty(&pool, path.to_str().unwrap()).await.unwrap();

        let role_of = |u: &'static str| {
            let pool = pool.clone();
            async move {
                sqlx::query_scalar::<_, String>("SELECT role FROM users WHERE username = ?")
                    .bind(u)
                    .fetch_one(&pool)
                    .await
                    .unwrap()
            }
        };
        assert_eq!(role_of("jason").await, "admin", "missing role defaults to admin");
        assert_eq!(role_of("geoff").await, "editor");

        // Legacy INSERTs without a role column also land as admin (schema default).
        sqlx::query("INSERT INTO users (username, password_hash) VALUES ('legacy', 'x')")
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(role_of("legacy").await, "admin");
    }

    #[tokio::test]
    async fn change_password_clears_must_change_flag() {
        let pool = crate::testutil::test_pool().await;
        let salt = SaltString::generate(&mut OsRng);
        let hash = Argon2::default()
            .hash_password(b"temppass123", &salt)
            .unwrap()
            .to_string();
        sqlx::query(
            "INSERT INTO users (username, password_hash, role, must_change_password) VALUES ('geoff', ?, 'editor', 1)",
        )
        .bind(&hash)
        .execute(&pool)
        .await
        .unwrap();

        let st = AppState {
            pool: pool.clone(),
            uploads_dir: std::env::temp_dir(),
            seed_path: std::env::temp_dir().join("unused.json"),
            db_path: std::env::temp_dir().join("unused.db"),
            jwt_secret: Arc::new(b"test-secret".to_vec()),
        };
        let auth = AuthUser { username: "geoff".into(), role: "editor".into() };
        change_password(
            auth,
            State(st),
            Json(ChangePasswordReq {
                current_password: "temppass123".into(),
                new_password: "a-real-password".into(),
            }),
        )
        .await
        .expect("change succeeds with correct current password");

        let flag: i64 = sqlx::query_scalar(
            "SELECT must_change_password FROM users WHERE username = 'geoff'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(flag, 0, "flag cleared after password change");
    }

    #[tokio::test]
    async fn role_check_rejects_editor_for_admin_gate() {
        let editor = AuthUser { username: "geoff".into(), role: "editor".into() };
        let admin = AuthUser { username: "jason".into(), role: "admin".into() };
        assert!(!editor.is_admin());
        assert!(admin.is_admin());
    }

    #[tokio::test]
    async fn failed_login_counting_hits_burst_limit() {
        let pool = crate::testutil::test_pool().await;
        let ip = "1.2.3.0";

        assert_eq!(failed_logins_since(&pool, ip, "-5 minutes").await.unwrap(), 0);

        for _ in 0..LOGIN_BURST_LIMIT {
            sqlx::query("INSERT INTO login_rate_log (ip_hash) VALUES (?)")
                .bind(ip)
                .execute(&pool)
                .await
                .unwrap();
        }

        let burst = failed_logins_since(&pool, ip, "-5 minutes").await.unwrap();
        assert!(burst >= LOGIN_BURST_LIMIT, "burst window counts all recent failures");

        // A different subnet is unaffected.
        assert_eq!(failed_logins_since(&pool, "9.9.9.0", "-5 minutes").await.unwrap(), 0);

        // Attempts outside the window don't count.
        sqlx::query("UPDATE login_rate_log SET attempted_at = datetime('now', '-2 hours')")
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(failed_logins_since(&pool, ip, "-1 hour").await.unwrap(), 0);
    }
}
