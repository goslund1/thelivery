//! Shakedown's store: the hands-on checker (~/Projects/Shakedown), hosted at
//! /shakedown/ and kept here so a laptop and a phone share one run. The
//! agreement is Shakedown's `spec/hosted.md`; this serves it, admin only.
//!
//! - Sign-in: The Livery's own password check (auth::check_credentials),
//!   then a 90-day token marked `kind: "shakedown"`. The Livery's routes
//!   refuse it (auth::AuthUser), and these routes refuse The Livery's.
//! - Uploads: each install's checklist and map, with an upload key that
//!   is good for that route and nothing else (only its SHA-256 is kept).
//! - Runs and answers: every change is kept; each field of a row's answer
//!   takes the change given latest (by the giving page's clock), so an
//!   answer that waited on the phone doesn't overwrite a newer one.
//! - The live channel: server-sent events to every page following.

use std::convert::Infallible;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use argon2::password_hash::rand_core::{OsRng, RngCore};
use axum::{
    async_trait,
    extract::{ConnectInfo, FromRequestParts, Path, Query, State},
    http::{header::AUTHORIZATION, request::Parts, HeaderMap, StatusCode},
    response::sse::{Event, KeepAlive, Sse},
    routing::{delete, get, post, put},
    Json, Router,
};
use base64::Engine;
use futures_util::stream::{self, Stream, StreamExt};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use sqlx::Row;
use tokio::sync::broadcast;

use crate::state::{err, ApiError, AppState};

/// Shakedown's sign-in lasts about 90 days (D-P4). The Livery's own is
/// auth::TOKEN_TTL_SECS, 7 days, and stays so.
pub const TOKEN_TTL_SECS: u64 = 90 * 24 * 3600;
const KIND: &str = "shakedown";
/// A checklist or map as uploaded, or a run's texts.
const MAX_TEXT: usize = 2 * 1024 * 1024;
/// A note, a wish.
const MAX_FIELD: usize = 10_000;
const MAX_COUNTS: usize = 200_000;
/// A change "given" further ahead than this, by the server's clock, is taken
/// as given now: a page with a wrong clock can't win every merge after it.
const FUTURE_SLACK_MS: i64 = 5_000;
const MAX_CHANGES: usize = 500;

// --- The live channel -------------------------------------------------------

#[derive(Clone, Debug)]
pub struct LiveEvent {
    pub name: &'static str,
    pub data: Value,
}

/// Every page following gets every event; each picks out its own run.
pub type Live = broadcast::Sender<LiveEvent>;

pub fn live() -> Live {
    broadcast::channel(256).0
}

fn announce(st: &AppState, name: &'static str, data: Value) {
    // No page listening is fine.
    let _ = st.shakedown.send(LiveEvent { name, data });
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/shakedown/login", post(login))
        .route("/api/shakedown/me", get(me))
        .route("/api/shakedown/apps", get(apps))
        .route("/api/shakedown/apps/:name", put(upload))
        .route("/api/shakedown/apps/:name/run", get(app_run))
        .route("/api/shakedown/keys", get(list_keys).post(make_key))
        .route("/api/shakedown/keys/:id", delete(revoke_key))
        .route("/api/shakedown/runs", get(list_runs).post(create_run))
        .route("/api/shakedown/runs/:id", get(get_run).patch(rename_run))
        .route("/api/shakedown/runs/:id/changes", post(post_changes))
        .route("/api/shakedown/runs/:id/send", post(send_to_phone))
        .route("/api/shakedown/runs/:id/counts", put(put_counts))
        .route("/api/shakedown/events", get(events))
}

// --- Small helpers ----------------------------------------------------------

fn db(e: sqlx::Error) -> ApiError {
    err(StatusCode::INTERNAL_SERVER_ERROR, e)
}

fn bad(msg: impl ToString) -> ApiError {
    err(StatusCode::BAD_REQUEST, msg)
}

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

/// Unix seconds as an ISO time in UTC ("2026-10-12T18:20:00Z").
fn iso_utc(secs: u64) -> String {
    let (days, rem) = ((secs / 86_400) as i64, secs % 86_400);
    // Howard Hinnant's civil_from_days.
    let z = days + 719_468;
    let era = (if z >= 0 { z } else { z - 146_096 }) / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe as i64 + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", rem / 3_600, rem % 3_600 / 60, rem % 60)
}

fn hash_key(key: &str) -> String {
    Sha256::digest(key.as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
}

/// A checklist row's id, as the results file has it: "UP-012", "IO-226".
/// A checklist row's id, "UP-001", or one of its parts', "UP-001.2"
/// (Shakedown's B-18, nested parts).
fn row_id_ok(s: &str) -> bool {
    let Some((head, num)) = s.rsplit_once('-') else { return false };
    let digits = |x: &str| !x.is_empty() && x.chars().all(|c| c.is_ascii_digit());
    let num_ok = match num.split_once('.') {
        Some((row, part)) => digits(row) && digits(part),
        None => digits(num),
    };
    s.len() <= 32
        && head.starts_with(|c: char| c.is_ascii_uppercase())
        && head.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        && num_ok
}

/// The parts the checklist gives each row: an indented row-shaped line,
/// "  - [ ] **UP-001.1 · …". Row id → its part ids.
fn parts_of(checklist: &str) -> std::collections::HashMap<String, Vec<String>> {
    let mut out: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for line in checklist.lines() {
        if !line.starts_with(char::is_whitespace) || !line.trim_start().starts_with("- [") {
            continue;
        }
        let Some(a) = line.find("**") else { continue };
        let rest = &line[a + 2..];
        let Some(b) = rest.find(" · ") else { continue };
        let id = &rest[..b];
        if let Some((row, _)) = id.split_once('.') {
            if row_id_ok(id) {
                out.entry(row.to_string()).or_default().push(id.to_string());
            }
        }
    }
    out
}

/// An app card's counts (D, Jason 2026-10-13): a row is answered when it has
/// its own verdict, or every one of its parts has one; skipped when it was
/// skipped and isn't answered. Parts aren't counted on their own.
fn count_rows(answers: &std::collections::HashMap<String, Value>, checklist: &str) -> (i64, i64) {
    let has_verdict = |id: &str| {
        answers.get(id).and_then(|v| v.get("verdict")).and_then(Value::as_str).is_some_and(|x| !x.is_empty())
    };
    let parts = parts_of(checklist);
    let mut answered = std::collections::HashSet::new();
    for id in answers.keys().filter(|id| !id.contains('.')) {
        if has_verdict(id) {
            answered.insert(id.clone());
        }
    }
    for (row, ps) in &parts {
        if !ps.is_empty() && ps.iter().all(|p| has_verdict(p)) {
            answered.insert(row.clone());
        }
    }
    let skipped = answers
        .iter()
        .filter(|(id, v)| !id.contains('.') && !answered.contains(*id) && v.get("skipped").and_then(Value::as_bool) == Some(true))
        .count();
    (answered.len() as i64, skipped as i64)
}

/// A check map's pane id: "P-05".
fn pane_id_ok(s: &str) -> bool {
    s.len() <= 16 && s.strip_prefix("P-").is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
}

fn text_ok(s: &str, max: usize) -> bool {
    s.chars().count() <= max
}

/// A name or label: present, at most `max` characters, no control characters.
fn name_ok(s: &str, max: usize) -> bool {
    !s.trim().is_empty() && s.chars().count() <= max && !s.chars().any(char::is_control)
}

// --- Signing in ------------------------------------------------------------

#[derive(Serialize, Deserialize)]
struct ShakedownClaims {
    sub: String,
    exp: usize,
    /// Required: The Livery's own tokens have no kind, so they can't decode here.
    kind: String,
}

/// A signed-in admin, checked in the database on every request.
pub struct Admin(pub String);

#[async_trait]
impl FromRequestParts<AppState> for Admin {
    type Rejection = ApiError;
    async fn from_request_parts(parts: &mut Parts, st: &AppState) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "))
            .ok_or_else(|| err(StatusCode::UNAUTHORIZED, "sign in to Shakedown"))?;
        let data = decode::<ShakedownClaims>(token, &DecodingKey::from_secret(&st.jwt_secret), &Validation::default())
            .map_err(|_| err(StatusCode::UNAUTHORIZED, "sign in to Shakedown again"))?;
        if data.claims.kind != KIND {
            return Err(err(StatusCode::UNAUTHORIZED, "not a Shakedown sign-in"));
        }
        check_admin(st, &data.claims.sub).await?;
        Ok(Admin(data.claims.sub))
    }
}

/// Who answers: the admin signed in (a page), or a test build by its own key
/// (`sda_…`, Shakedown's stage 3, B-7), on behalf of the admin who made it.
/// A test-build key reads the runs and answers into them; an upload key
/// (`sdk_…`) never does.
pub struct Answerer(pub String);

#[async_trait]
impl FromRequestParts<AppState> for Answerer {
    type Rejection = ApiError;
    async fn from_request_parts(parts: &mut Parts, st: &AppState) -> Result<Self, Self::Rejection> {
        let Some(key) = parts.headers.get("x-shakedown-key").and_then(|v| v.to_str().ok()).map(str::to_owned) else {
            return Admin::from_request_parts(parts, st).await.map(|Admin(u)| Answerer(u));
        };
        if !key.starts_with(TEST_KEY_PREFIX) {
            return Err(err(StatusCode::UNAUTHORIZED, "an upload key only uploads"));
        }
        let made_by: String = sqlx::query_scalar("SELECT created_by FROM shakedown_keys WHERE key_hash = ? AND revoked_at IS NULL")
            .bind(hash_key(&key))
            .fetch_optional(&st.pool)
            .await
            .map_err(db)?
            .ok_or_else(|| err(StatusCode::UNAUTHORIZED, "that test-build key isn't good"))?;
        check_admin(st, &made_by).await?;
        sqlx::query("UPDATE shakedown_keys SET last_used_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE key_hash = ?")
            .bind(hash_key(&key))
            .execute(&st.pool)
            .await
            .map_err(db)?;
        Ok(Answerer(made_by))
    }
}

const UPLOAD_KEY_PREFIX: &str = "sdk_";
const TEST_KEY_PREFIX: &str = "sda_";

async fn check_admin(st: &AppState, username: &str) -> Result<(), ApiError> {
    let row = sqlx::query("SELECT role, must_change_password FROM users WHERE username = ?")
        .bind(username)
        .fetch_optional(&st.pool)
        .await
        .map_err(db)?
        .ok_or_else(|| err(StatusCode::UNAUTHORIZED, "user no longer exists"))?;
    if row.get::<String, _>("role") != "admin" {
        return Err(err(StatusCode::FORBIDDEN, "Shakedown is for The Livery's admin"));
    }
    if row.get::<i64, _>("must_change_password") != 0 {
        return Err(err(StatusCode::FORBIDDEN, "change your password in The Livery first"));
    }
    Ok(())
}

#[derive(Deserialize)]
struct LoginReq {
    username: String,
    password: String,
}

async fn login(
    State(st): State<AppState>,
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<LoginReq>,
) -> Result<Json<Value>, ApiError> {
    let (role, must_change) = crate::auth::check_credentials(&st, &addr, &headers, &req.username, &req.password).await?;
    if role != "admin" {
        return Err(err(StatusCode::FORBIDDEN, "Shakedown is for The Livery's admin"));
    }
    if must_change {
        return Err(err(StatusCode::FORBIDDEN, "change your password in The Livery first"));
    }
    let exp = now_secs() + TOKEN_TTL_SECS;
    let claims = ShakedownClaims { sub: req.username.clone(), exp: exp as usize, kind: KIND.into() };
    let token = encode(&Header::default(), &claims, &EncodingKey::from_secret(&st.jwt_secret))
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(json!({ "token": token, "username": req.username, "expires": iso_utc(exp) })))
}

async fn me(Admin(username): Admin) -> Json<Value> {
    Json(json!({ "username": username }))
}

// --- Upload keys and uploads -----------------------------------------------

async fn list_keys(_: Admin, State(st): State<AppState>) -> Result<Json<Value>, ApiError> {
    let rows = sqlx::query(
        "SELECT id, label, created_at, last_used_at FROM shakedown_keys WHERE revoked_at IS NULL ORDER BY id",
    )
    .fetch_all(&st.pool)
    .await
    .map_err(db)?;
    Ok(Json(Value::Array(
        rows.iter()
            .map(|r| {
                json!({
                    "id": r.get::<i64, _>("id"),
                    "label": r.get::<String, _>("label"),
                    "createdAt": r.get::<String, _>("created_at"),
                    "lastUsedAt": r.get::<Option<String>, _>("last_used_at"),
                })
            })
            .collect(),
    )))
}

#[derive(Deserialize)]
struct KeyReq {
    label: String,
    /// "upload" (the default: an install's step) or "test" (a test build's
    /// key for answers and counts, B-7). The kind is the key's prefix.
    #[serde(default)]
    kind: Option<String>,
}

/// A new key, shown this once; only its SHA-256 is kept.
async fn make_key(Admin(username): Admin, State(st): State<AppState>, Json(req): Json<KeyReq>) -> Result<Json<Value>, ApiError> {
    let label = req.label.trim();
    if !name_ok(label, 80) {
        return Err(bad("a key needs a label of at most 80 characters"));
    }
    let (prefix, label) = match req.kind.as_deref() {
        None | Some("upload") => (UPLOAD_KEY_PREFIX, label.to_string()),
        Some("test") => (TEST_KEY_PREFIX, format!("Test build · {label}")),
        Some(_) => return Err(bad("a key is an upload key or a test-build key")),
    };
    let label = label.as_str();
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    let key = format!("{prefix}{}", base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes));
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO shakedown_keys (label, key_hash, created_by) VALUES (?, ?, ?) RETURNING id",
    )
    .bind(label)
    .bind(hash_key(&key))
    .bind(&username)
    .fetch_one(&st.pool)
    .await
    .map_err(db)?;
    Ok(Json(json!({ "id": id, "label": label, "key": key })))
}

async fn revoke_key(_: Admin, State(st): State<AppState>, Path(id): Path<i64>) -> Result<Json<Value>, ApiError> {
    let done = sqlx::query(
        "UPDATE shakedown_keys SET revoked_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ? AND revoked_at IS NULL",
    )
    .bind(id)
    .execute(&st.pool)
    .await
    .map_err(db)?;
    if done.rows_affected() == 0 {
        return Err(err(StatusCode::NOT_FOUND, "no such key"));
    }
    Ok(Json(json!({ "revoked": id })))
}

#[derive(Deserialize)]
struct UploadReq {
    #[serde(default)]
    commit: String,
    #[serde(default)]
    date: String,
    checklist: String,
    #[serde(default)]
    map: String,
}

/// An install's checklist and map (D-P5). The upload key is the only thing
/// that opens this route, and this is the only route it opens.
async fn upload(
    State(st): State<AppState>,
    Path(name): Path<String>,
    headers: HeaderMap,
    Json(req): Json<UploadReq>,
) -> Result<Json<Value>, ApiError> {
    let key = headers
        .get("x-shakedown-key")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| err(StatusCode::UNAUTHORIZED, "an upload key is needed"))?;
    if !key.starts_with(UPLOAD_KEY_PREFIX) {
        return Err(err(StatusCode::UNAUTHORIZED, "a test-build key doesn't upload"));
    }
    let key_id: i64 = sqlx::query_scalar("SELECT id FROM shakedown_keys WHERE key_hash = ? AND revoked_at IS NULL")
        .bind(hash_key(key))
        .fetch_optional(&st.pool)
        .await
        .map_err(db)?
        .ok_or_else(|| err(StatusCode::UNAUTHORIZED, "that upload key isn't good"))?;
    let name = name.trim();
    if !name_ok(name, 64) {
        return Err(bad("an app's name is 1 to 64 characters"));
    }
    if req.checklist.trim().is_empty() {
        return Err(bad("the checklist is empty"));
    }
    if req.checklist.len() > MAX_TEXT || req.map.len() > MAX_TEXT {
        return Err(err(StatusCode::PAYLOAD_TOO_LARGE, "a checklist or map is at most 2 MB"));
    }
    if !text_ok(&req.commit, 64) || !text_ok(&req.date, 32) {
        return Err(bad("the commit or date is too long"));
    }
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO shakedown_uploads (app, commit_id, date, checklist, map, key_id) VALUES (?, ?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(name)
    .bind(req.commit.trim())
    .bind(req.date.trim())
    .bind(&req.checklist)
    .bind(&req.map)
    .bind(key_id)
    .fetch_one(&st.pool)
    .await
    .map_err(db)?;
    sqlx::query("UPDATE shakedown_keys SET last_used_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?")
        .bind(key_id)
        .execute(&st.pool)
        .await
        .map_err(db)?;
    Ok(Json(json!({ "app": name, "id": id })))
}

/// The newest upload of each app, in the shape of `apps.js`.
async fn apps(_: Admin, State(st): State<AppState>) -> Result<Json<Value>, ApiError> {
    let rows = sqlx::query(
        "SELECT u.app, u.commit_id, u.date, u.checklist, u.map, u.uploaded_at
         FROM shakedown_uploads u
         JOIN (SELECT app, MAX(id) AS id FROM shakedown_uploads GROUP BY app) newest ON u.id = newest.id
         ORDER BY u.app",
    )
    .fetch_all(&st.pool)
    .await
    .map_err(db)?;
    let apps: Vec<Value> = rows
        .iter()
        .map(|r| {
            json!({
                "name": r.get::<String, _>("app"),
                "commit": r.get::<String, _>("commit_id"),
                "date": r.get::<String, _>("date"),
                "checklist": r.get::<String, _>("checklist"),
                "map": r.get::<String, _>("map"),
                "uploaded": r.get::<String, _>("uploaded_at"),
            })
        })
        .collect();
    Ok(Json(json!({ "version": 1, "apps": apps })))
}

// --- Runs -------------------------------------------------------------------

/// A row's or pane's merged state, as the page keeps an answer: each field's value.
fn flatten(state: &Map<String, Value>) -> Value {
    Value::Object(state.iter().map(|(k, f)| (k.clone(), f.get("v").cloned().unwrap_or(Value::Null))).collect())
}

async fn run_seq(st: &AppState, run_id: &str) -> Result<i64, ApiError> {
    sqlx::query_scalar("SELECT COALESCE(MAX(seq), 0) FROM shakedown_changes WHERE run_id = ?")
        .bind(run_id)
        .fetch_one(&st.pool)
        .await
        .map_err(db)
}

async fn summary(st: &AppState, run_id: &str) -> Result<Value, ApiError> {
    let r = sqlx::query("SELECT id, app, build, started, created_at, closed_at, send_json, checklist_text FROM shakedown_runs WHERE id = ?")
        .bind(run_id)
        .fetch_optional(&st.pool)
        .await
        .map_err(db)?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such run"))?;
    let rows: Vec<(String, String)> = sqlx::query_as("SELECT target, state FROM shakedown_answers WHERE run_id = ? AND kind = 'row'")
        .bind(run_id)
        .fetch_all(&st.pool)
        .await
        .map_err(db)?;
    let answers = rows
        .into_iter()
        .map(|(id, s)| (id, flatten(&serde_json::from_str::<Map<String, Value>>(&s).unwrap_or_default())))
        .collect();
    let (answered, skipped) = count_rows(&answers, &r.get::<String, _>("checklist_text"));
    let send = r
        .get::<Option<String>, _>("send_json")
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .unwrap_or(Value::Null);
    Ok(json!({
        "id": r.get::<String, _>("id"),
        "app": r.get::<String, _>("app"),
        "build": r.get::<String, _>("build"),
        "started": r.get::<String, _>("started"),
        "createdAt": r.get::<String, _>("created_at"),
        "closed": r.get::<Option<String>, _>("closed_at"),
        "seq": run_seq(st, run_id).await?,
        "answered": answered,
        "skipped": skipped,
        "send": send,
    }))
}

async fn whole(st: &AppState, run_id: &str) -> Result<Value, ApiError> {
    let mut out = summary(st, run_id).await?;
    let r = sqlx::query("SELECT checklist_name, map_name, checklist_text, map_text FROM shakedown_runs WHERE id = ?")
        .bind(run_id)
        .fetch_one(&st.pool)
        .await
        .map_err(db)?;
    let rows = sqlx::query("SELECT kind, target, state FROM shakedown_answers WHERE run_id = ?")
        .bind(run_id)
        .fetch_all(&st.pool)
        .await
        .map_err(db)?;
    let (mut answers, mut panes) = (Map::new(), Map::new());
    for a in &rows {
        let state = serde_json::from_str::<Map<String, Value>>(&a.get::<String, _>("state")).unwrap_or_default();
        let into = if a.get::<String, _>("kind") == "pane" { &mut panes } else { &mut answers };
        into.insert(a.get("target"), flatten(&state));
    }
    let o = out.as_object_mut().expect("a summary is an object");
    o.insert("checklistName".into(), Value::String(r.get("checklist_name")));
    o.insert("mapName".into(), Value::String(r.get("map_name")));
    o.insert("checklistText".into(), Value::String(r.get("checklist_text")));
    o.insert("mapText".into(), Value::String(r.get("map_text")));
    o.insert("answers".into(), Value::Object(answers));
    o.insert("paneNotes".into(), Value::Object(panes));
    // The latest counts a test build put beside the run (B-7), if any.
    let counts: Option<String> = sqlx::query_scalar("SELECT text FROM shakedown_counts WHERE run_id = ? ORDER BY seq DESC LIMIT 1")
        .bind(run_id)
        .fetch_optional(&st.pool)
        .await
        .map_err(db)?;
    o.insert("countsText".into(), counts.map(Value::String).unwrap_or(Value::Null));
    Ok(out)
}

#[derive(Deserialize)]
struct RunsQuery {
    app: Option<String>,
    all: Option<String>,
}

/// The open runs; or, with `?app=…&all=1`, every run of that app, newest first.
async fn list_runs(_: Admin, State(st): State<AppState>, Query(q): Query<RunsQuery>) -> Result<Json<Value>, ApiError> {
    let ids: Vec<String> = match (&q.app, q.all.as_deref()) {
        (Some(app), Some("1")) => sqlx::query_scalar("SELECT id FROM shakedown_runs WHERE app = ? ORDER BY rowid DESC")
            .bind(app)
            .fetch_all(&st.pool)
            .await,
        (Some(app), _) => sqlx::query_scalar("SELECT id FROM shakedown_runs WHERE app = ? AND closed_at IS NULL")
            .bind(app)
            .fetch_all(&st.pool)
            .await,
        (None, _) => sqlx::query_scalar("SELECT id FROM shakedown_runs WHERE closed_at IS NULL ORDER BY app")
            .fetch_all(&st.pool)
            .await,
    }
    .map_err(db)?;
    let mut out = Vec::with_capacity(ids.len());
    for id in &ids {
        out.push(summary(&st, id).await?);
    }
    Ok(Json(Value::Array(out)))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NewRun {
    app: String,
    build: String,
    #[serde(default)]
    started: String,
    #[serde(default)]
    checklist_name: String,
    #[serde(default)]
    map_name: String,
    checklist_text: String,
    #[serde(default)]
    map_text: String,
}

/// A new run of an app. Its open run, if any, closes (and is kept).
async fn create_run(Admin(username): Admin, State(st): State<AppState>, Json(r): Json<NewRun>) -> Result<Json<Value>, ApiError> {
    let app = r.app.trim();
    let build = r.build.trim();
    if !name_ok(app, 200) || !name_ok(build, 200) {
        return Err(bad("a run needs an app and a build, each at most 200 characters"));
    }
    if r.checklist_text.trim().is_empty() {
        return Err(bad("the checklist is empty"));
    }
    if r.checklist_text.len() > MAX_TEXT || r.map_text.len() > MAX_TEXT {
        return Err(err(StatusCode::PAYLOAD_TOO_LARGE, "a checklist or map is at most 2 MB"));
    }
    if !text_ok(&r.checklist_name, 200) || !text_ok(&r.map_name, 200) || !text_ok(&r.started, 32) {
        return Err(bad("a name or date is too long"));
    }
    let started = if r.started.trim().is_empty() { iso_utc(now_secs())[..10].to_string() } else { r.started.trim().to_string() };
    let id = uuid::Uuid::new_v4().to_string();
    let mut tx = st.pool.begin().await.map_err(db)?;
    let closed: Vec<String> = sqlx::query_scalar("SELECT id FROM shakedown_runs WHERE app = ? AND closed_at IS NULL")
        .bind(app)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
    sqlx::query("UPDATE shakedown_runs SET closed_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE app = ? AND closed_at IS NULL")
        .bind(app)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
    sqlx::query(
        "INSERT INTO shakedown_runs (id, app, build, started, checklist_name, map_name, checklist_text, map_text, created_by)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(app)
    .bind(build)
    .bind(&started)
    .bind(&r.checklist_name)
    .bind(&r.map_name)
    .bind(&r.checklist_text)
    .bind(&r.map_text)
    .bind(&username)
    .execute(&mut *tx)
    .await
    .map_err(db)?;
    tx.commit().await.map_err(db)?;
    for c in &closed {
        announce(&st, "run", summary(&st, c).await?);
    }
    announce(&st, "run", summary(&st, &id).await?);
    Ok(Json(whole(&st, &id).await?))
}

async fn get_run(_: Answerer, State(st): State<AppState>, Path(id): Path<String>) -> Result<Json<Value>, ApiError> {
    Ok(Json(whole(&st, &id).await?))
}

/// An app's open run, whole (a test build asks for its own app's), or 404.
async fn app_run(_: Answerer, State(st): State<AppState>, Path(name): Path<String>) -> Result<Json<Value>, ApiError> {
    let id: String = sqlx::query_scalar("SELECT id FROM shakedown_runs WHERE app = ? AND closed_at IS NULL ORDER BY rowid DESC LIMIT 1")
        .bind(name.trim())
        .fetch_optional(&st.pool)
        .await
        .map_err(db)?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "no run of that app is open: start one on the page"))?;
    Ok(Json(whole(&st, &id).await?))
}

#[derive(Deserialize)]
struct CountsReq {
    text: String,
    #[serde(default)]
    device: String,
}

/// A test build's counts beside its run (B-7): the counts file, totals only;
/// the raw record never leaves the Mac (D-H6). Each is kept
/// (`shakedown_counts`); the latest is the run's, and announced, so a page
/// shows Tried without a file.
async fn put_counts(
    Answerer(username): Answerer,
    State(st): State<AppState>,
    Path(run_id): Path<String>,
    Json(c): Json<CountsReq>,
) -> Result<Json<Value>, ApiError> {
    if !c.text.starts_with("# Shakedown counts, version ") {
        return Err(bad("that isn't a counts file"));
    }
    if c.text.len() > MAX_COUNTS || !text_ok(&c.device, 64) {
        return Err(err(StatusCode::PAYLOAD_TOO_LARGE, "a counts file is at most 200 KB"));
    }
    summary(&st, &run_id).await?;
    let seq: i64 = sqlx::query_scalar(
        "INSERT INTO shakedown_counts (run_id, text, device, username) VALUES (?, ?, ?, ?) RETURNING seq",
    )
    .bind(&run_id)
    .bind(&c.text)
    .bind(&c.device)
    .bind(&username)
    .fetch_one(&st.pool)
    .await
    .map_err(db)?;
    announce(&st, "counts", json!({ "runId": run_id, "seq": seq, "device": c.device }));
    Ok(Json(json!({ "seq": seq })))
}

#[derive(Deserialize)]
struct Rename {
    build: String,
}

/// The build renamed (the laptop's Build box). Takes a seq like any change.
async fn rename_run(
    Admin(username): Admin,
    State(st): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<Rename>,
) -> Result<Json<Value>, ApiError> {
    let build = req.build.trim();
    if !name_ok(build, 200) {
        return Err(bad("a build is 1 to 200 characters"));
    }
    summary(&st, &id).await?;
    let mut tx = st.pool.begin().await.map_err(db)?;
    sqlx::query("INSERT INTO shakedown_changes (run_id, cid, kind, fields, username) VALUES (?, ?, 'build', ?, ?)")
        .bind(&id)
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(json!({ "build": { "v": build } }).to_string())
        .bind(&username)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
    sqlx::query("UPDATE shakedown_runs SET build = ? WHERE id = ?")
        .bind(build)
        .bind(&id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
    tx.commit().await.map_err(db)?;
    let s = summary(&st, &id).await?;
    announce(&st, "run", s.clone());
    Ok(Json(s))
}

// --- Answers and the merge ---------------------------------------------------

#[derive(Deserialize)]
struct ChangesReq {
    changes: Vec<ChangeIn>,
}

#[derive(Deserialize)]
struct ChangeIn {
    cid: String,
    kind: String,
    id: String,
    fields: Map<String, Value>,
    #[serde(default)]
    device: String,
}

/// One field of a change, checked: its value and when it was given (ms).
struct FieldIn {
    name: String,
    v: Value,
    at: i64,
}

fn check_change(c: &ChangeIn, now: i64) -> Result<Vec<FieldIn>, ApiError> {
    if !name_ok(&c.cid, 64) || !text_ok(&c.device, 64) {
        return Err(bad("a change needs a cid (and a device) of at most 64 characters"));
    }
    let allowed: &[&str] = match c.kind.as_str() {
        "row" if row_id_ok(&c.id) => &["verdict", "skipped", "addressed", "note", "wish", "date"],
        "pane" if pane_id_ok(&c.id) => &["note", "date"],
        "row" | "pane" => return Err(bad(format!("\"{}\" isn't a {} id", c.id, c.kind))),
        _ => return Err(bad("a change is to a row or a pane")),
    };
    if c.fields.is_empty() {
        return Err(bad("a change changes something"));
    }
    let mut out = Vec::with_capacity(c.fields.len());
    for (name, f) in &c.fields {
        if !allowed.contains(&name.as_str()) {
            return Err(bad(format!("a {} has no field \"{name}\"", c.kind)));
        }
        let v = f.get("v").cloned().ok_or_else(|| bad(format!("{name} has no value")))?;
        let at = f.get("at").and_then(Value::as_i64).filter(|a| *a > 0).ok_or_else(|| bad(format!("{name} has no time")))?;
        let ok = match name.as_str() {
            "verdict" => matches!(v.as_str(), Some("" | "passed" | "failed" | "partly")),
            "skipped" | "addressed" => v.is_boolean(),
            "note" | "wish" => v.as_str().is_some_and(|s| text_ok(s, MAX_FIELD)),
            "date" => v.as_str().is_some_and(|s| text_ok(s, 32)),
            _ => false,
        };
        if !ok {
            return Err(bad(format!("{name}: {v} isn't allowed")));
        }
        out.push(FieldIn { name: name.clone(), v, at: at.min(now + FUTURE_SLACK_MS) });
    }
    Ok(out)
}

/// Answers from a page (D-P7). Each field takes the change given latest; on
/// the same time, the later to arrive. A cid seen before is ignored.
async fn post_changes(
    Answerer(username): Answerer,
    State(st): State<AppState>,
    Path(run_id): Path<String>,
    Json(req): Json<ChangesReq>,
) -> Result<Json<Value>, ApiError> {
    if req.changes.len() > MAX_CHANGES {
        return Err(bad(format!("at most {MAX_CHANGES} changes at once")));
    }
    summary(&st, &run_id).await?;
    let now = now_ms();
    let checked = req
        .changes
        .iter()
        .map(|c| check_change(c, now).map(|f| (c, f)))
        .collect::<Result<Vec<_>, _>>()?;

    let mut tx = st.pool.begin().await.map_err(db)?;
    let (mut applied, mut ignored, mut events) = (Vec::new(), Vec::new(), Vec::new());
    for (c, fields) in checked {
        let seen: Option<i64> = sqlx::query_scalar("SELECT seq FROM shakedown_changes WHERE cid = ?")
            .bind(&c.cid)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
        if seen.is_some() {
            ignored.push(c.cid.clone());
            continue;
        }
        let sent: Map<String, Value> =
            fields.iter().map(|f| (f.name.clone(), json!({ "v": f.v, "at": f.at }))).collect();
        let seq: i64 = sqlx::query_scalar(
            "INSERT INTO shakedown_changes (run_id, cid, kind, target, fields, device, username)
             VALUES (?, ?, ?, ?, ?, ?, ?) RETURNING seq",
        )
        .bind(&run_id)
        .bind(&c.cid)
        .bind(&c.kind)
        .bind(&c.id)
        .bind(Value::Object(sent).to_string())
        .bind(&c.device)
        .bind(&username)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let before: Option<String> =
            sqlx::query_scalar("SELECT state FROM shakedown_answers WHERE run_id = ? AND kind = ? AND target = ?")
                .bind(&run_id)
                .bind(&c.kind)
                .bind(&c.id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?;
        let mut state: Map<String, Value> = before.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        for f in fields {
            let older = state.get(&f.name).and_then(|o| o.get("at")).and_then(Value::as_i64);
            // Later given wins; the same time, the later to arrive (this one).
            if older.map_or(true, |o| f.at >= o) {
                state.insert(f.name, json!({ "v": f.v, "at": f.at, "seq": seq }));
            }
        }
        sqlx::query(
            "INSERT INTO shakedown_answers (run_id, kind, target, state, updated_seq, server_at)
             VALUES (?, ?, ?, ?, ?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
             ON CONFLICT (run_id, kind, target) DO UPDATE SET
               state = excluded.state, updated_seq = excluded.updated_seq, server_at = excluded.server_at",
        )
        .bind(&run_id)
        .bind(&c.kind)
        .bind(&c.id)
        .bind(Value::Object(state.clone()).to_string())
        .bind(seq)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        events.push(json!({
            "runId": run_id, "seq": seq, "kind": c.kind, "id": c.id,
            "value": flatten(&state), "device": c.device,
        }));
        applied.push(c.cid.clone());
    }
    tx.commit().await.map_err(db)?;
    // Several changes in one post go as one event: WebKit hands a page only
    // the first of several events that arrive together, and holds the rest
    // until the next ping, 15 s on (measured 2026-10-13, Shakedown's
    // e2e-hosted, a batch of two). One change goes as it always has.
    if events.len() > 1 {
        let device = events[0]["device"].clone();
        announce(&st, "changes", json!({ "runId": run_id, "device": device, "changes": events }));
    } else {
        for e in events {
            announce(&st, "change", e);
        }
    }
    Ok(Json(json!({ "seq": run_seq(&st, &run_id).await?, "applied": applied, "ignored": ignored })))
}

// --- Send to Phone ------------------------------------------------------------

#[derive(Deserialize)]
struct SendReq {
    mode: String,
    group: String,
    #[serde(default)]
    current: Option<String>,
    #[serde(default)]
    label: String,
    #[serde(default)]
    device: String,
    /// Which screen it's for: "phone" (Send to Phone) or "laptop" (the
    /// phone's Show on Laptop). Missing: the phone, as before.
    #[serde(default)]
    to: Option<String>,
    /// The filter the sending screen had on, for the other to take.
    #[serde(default)]
    show: Option<String>,
}

/// One screen sends a section, pane, window or row to the other: the laptop
/// to the phone, or the phone to the laptop. The run keeps the last send to
/// the phone, so a phone that wasn't open goes there next time; a send to the
/// laptop is only announced (the laptop goes there if it's open).
async fn send_to_phone(
    Admin(username): Admin,
    State(st): State<AppState>,
    Path(run_id): Path<String>,
    Json(s): Json<SendReq>,
) -> Result<Json<Value>, ApiError> {
    if !matches!(s.mode.as_str(), "sections" | "panes" | "windows")
        || !name_ok(&s.group, 300)
        || !text_ok(&s.label, 300)
        || !text_ok(&s.device, 64)
        || s.current.as_deref().is_some_and(|c| !row_id_ok(c))
        || !matches!(s.to.as_deref(), None | Some("phone" | "laptop"))
        || s.show.as_deref().is_some_and(|f| !name_ok(f, 32))
    {
        return Err(bad("a send names a way through, a group, and maybe a row"));
    }
    summary(&st, &run_id).await?;
    let mut tx = st.pool.begin().await.map_err(db)?;
    let to = s.to.unwrap_or_else(|| "phone".into());
    let body = json!({ "mode": s.mode, "group": s.group, "current": s.current, "label": s.label, "device": s.device,
                       "to": to, "show": s.show });
    let seq: i64 = sqlx::query_scalar(
        "INSERT INTO shakedown_changes (run_id, cid, kind, fields, device, username) VALUES (?, ?, 'send', ?, ?, ?) RETURNING seq",
    )
    .bind(&run_id)
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(body.to_string())
    .bind(&s.device)
    .bind(&username)
    .fetch_one(&mut *tx)
    .await
    .map_err(db)?;
    if to == "phone" {
        let mut kept = body.clone();
        kept["seq"] = json!(seq);
        kept["at"] = json!(iso_utc(now_secs()));
        sqlx::query("UPDATE shakedown_runs SET send_json = ? WHERE id = ?")
            .bind(kept.to_string())
            .bind(&run_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
    }
    tx.commit().await.map_err(db)?;
    let mut event = body;
    event["runId"] = json!(run_id);
    event["seq"] = json!(seq);
    announce(&st, "send", event);
    Ok(Json(json!({ "seq": seq })))
}

// --- The live channel -----------------------------------------------------------

/// Server-sent events for every open run: hello, change, run, send (and
/// lagged, when a slow page missed some: it fetches its run again).
async fn events(
    _: Answerer,
    State(st): State<AppState>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, ApiError> {
    // Subscribed before reading seq, so nothing falls between the two.
    let rx = st.shakedown.subscribe();
    let seq: i64 = sqlx::query_scalar("SELECT COALESCE(MAX(seq), 0) FROM shakedown_changes")
        .fetch_one(&st.pool)
        .await
        .map_err(db)?;
    let hello = stream::once(async move { Ok(Event::default().event("hello").data(json!({ "seq": seq }).to_string())) });
    let live = stream::unfold(rx, |mut rx| async move {
        match rx.recv().await {
            Ok(ev) => Some((Ok(Event::default().event(ev.name).data(ev.data.to_string())), rx)),
            Err(broadcast::error::RecvError::Lagged(_)) => Some((Ok(Event::default().event("lagged").data("{}")), rx)),
            Err(broadcast::error::RecvError::Closed) => None,
        }
    });
    Ok(Sse::new(hello.chain(live)).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)).text("ping")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use argon2::password_hash::{PasswordHasher, SaltString};
    use argon2::Argon2;
    use axum::body::Body;
    use axum::extract::connect_info::MockConnectInfo;
    use axum::http::Request;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    async fn setup() -> (Router, AppState) {
        let st = AppState {
            pool: crate::testutil::test_pool().await,
            uploads_dir: std::env::temp_dir(),
            seed_path: std::env::temp_dir().join("unused.json"),
            db_path: std::env::temp_dir().join("unused.db"),
            jwt_secret: Arc::new(b"test-secret".to_vec()),
            site_url: String::new(),
            shakedown: live(),
        };
        let app = routes()
            .with_state(st.clone())
            .layer(MockConnectInfo(std::net::SocketAddr::from(([127, 0, 0, 1], 9))));
        (app, st)
    }

    async fn add_user(st: &AppState, name: &str, role: &str, password: Option<&str>, must_change: bool) {
        let hash = match password {
            Some(p) => Argon2::default().hash_password(p.as_bytes(), &SaltString::generate(&mut OsRng)).unwrap().to_string(),
            None => "$argon2$fake".into(),
        };
        sqlx::query("INSERT INTO users (username, password_hash, role, must_change_password) VALUES (?, ?, ?, ?)")
            .bind(name)
            .bind(hash)
            .bind(role)
            .bind(i64::from(must_change))
            .execute(&st.pool)
            .await
            .unwrap();
    }

    fn token(name: &str) -> String {
        let claims = ShakedownClaims { sub: name.into(), exp: (now_secs() + 3600) as usize, kind: KIND.into() };
        encode(&Header::default(), &claims, &EncodingKey::from_secret(b"test-secret")).unwrap()
    }

    fn bearer(t: &str) -> (&'static str, String) {
        ("authorization", format!("Bearer {t}"))
    }

    async fn call(app: &Router, method: &str, uri: &str, headers: &[(&str, String)], body: Option<Value>) -> (StatusCode, Value) {
        let mut b = Request::builder().method(method).uri(uri);
        for (k, v) in headers {
            b = b.header(*k, v);
        }
        let req = match body {
            Some(j) => b.header("content-type", "application/json").body(Body::from(j.to_string())),
            None => b.body(Body::empty()),
        }
        .unwrap();
        let res = app.clone().oneshot(req).await.unwrap();
        let status = res.status();
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        let v = serde_json::from_slice(&bytes).unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into()));
        (status, v)
    }

    const LIST: &str = "# UP scaler — shakedown\n\n## Viewer\n\n- [ ] **UP-001 · Opens** — it opens · Jason: — · Claude: —\n";

    async fn new_run(app: &Router, t: &str, app_name: &str) -> Value {
        let (s, run) = call(app, "POST", "/api/shakedown/runs", &[bearer(t)],
            Some(json!({ "app": app_name, "build": "abc1234", "checklistText": LIST }))).await;
        assert_eq!(s, StatusCode::OK, "{run}");
        run
    }

    fn change(cid: &str, kind: &str, id: &str, fields: Value, device: &str) -> Value {
        json!({ "cid": cid, "kind": kind, "id": id, "fields": fields, "device": device })
    }

    #[tokio::test]
    async fn a_test_build_key_answers_and_puts_counts_and_nothing_more() {
        let (app, st) = setup().await;
        add_user(&st, "jason", "admin", None, false).await;
        let t = token("jason");
        let (s, made) = call(&app, "POST", "/api/shakedown/keys", &[bearer(&t)], Some(json!({ "label": "UP's test build", "kind": "test" }))).await;
        assert_eq!(s, StatusCode::OK, "{made}");
        let key = made["key"].as_str().unwrap().to_string();
        assert!(key.starts_with("sda_") && key.len() == 47, "{key}");
        assert_eq!(made["label"], "Test build · UP's test build");
        let (_, upload_key) = call(&app, "POST", "/api/shakedown/keys", &[bearer(&t)], Some(json!({ "label": "Mac" }))).await;
        let upload_key = upload_key["key"].as_str().unwrap().to_string();
        assert!(upload_key.starts_with("sdk_"));
        let k = |v: &str| ("x-shakedown-key", v.to_string());

        // No open run yet: said so; then the page starts one and the test build finds it.
        let (s, _) = call(&app, "GET", "/api/shakedown/apps/UP%20scaler/run", &[k(&key)], None).await;
        assert_eq!(s, StatusCode::NOT_FOUND);
        let run = new_run(&app, &t, "UP scaler").await;
        let id = run["id"].as_str().unwrap();
        let (s, found) = call(&app, "GET", "/api/shakedown/apps/UP%20scaler/run", &[k(&key)], None).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(found["id"], id);
        assert_eq!(found["countsText"], Value::Null);

        // An answer from the test build merges as the phone's do, on the maker's name.
        let (s, r) = call(&app, "POST", &format!("/api/shakedown/runs/{id}/changes"), &[k(&key)],
            Some(json!({ "changes": [change("tb1", "row", "UP-001", json!({ "verdict": { "v": "partly", "at": 9 }, "addressed": { "v": true, "at": 9 } }), "test build")] }))).await;
        assert_eq!(s, StatusCode::OK, "{r}");
        let who: String = sqlx::query_scalar("SELECT username FROM shakedown_changes WHERE cid = 'tb1'").fetch_one(&st.pool).await.unwrap();
        assert_eq!(who, "jason");

        // Counts beside the run, the latest kept; anything that isn't a counts file refused.
        let counts = "# Shakedown counts, version 1\n# app\tUP scaler\nkind\tkey\tcount\tms\tdetail\nelement\tupPlay\t2\t0\tAXButton\n";
        let (s, _) = call(&app, "PUT", &format!("/api/shakedown/runs/{id}/counts"), &[k(&key)], Some(json!({ "text": counts, "device": "test build" }))).await;
        assert_eq!(s, StatusCode::OK);
        let (s, _) = call(&app, "PUT", &format!("/api/shakedown/runs/{id}/counts"), &[k(&key)], Some(json!({ "text": "a raw record" }))).await;
        assert_eq!(s, StatusCode::BAD_REQUEST);
        let (_, whole) = call(&app, "GET", &format!("/api/shakedown/runs/{id}"), &[k(&key)], None).await;
        assert_eq!(whole["countsText"], counts);
        assert_eq!(whole["answers"]["UP-001"]["verdict"], "partly");

        // An upload key never answers; a test-build key never uploads, never lists keys or starts runs.
        let (s, _) = call(&app, "POST", &format!("/api/shakedown/runs/{id}/changes"), &[k(&upload_key)],
            Some(json!({ "changes": [change("u1", "row", "UP-001", json!({ "verdict": { "v": "failed", "at": 10 } }), "")] }))).await;
        assert_eq!(s, StatusCode::UNAUTHORIZED);
        let (s, _) = call(&app, "PUT", "/api/shakedown/apps/UP%20scaler", &[k(&key)],
            Some(json!({ "checklist": LIST, "map": "", "commit": "abc", "date": "2026-10-13" }))).await;
        assert_eq!(s, StatusCode::UNAUTHORIZED);
        assert_eq!(call(&app, "GET", "/api/shakedown/keys", &[k(&key)], None).await.0, StatusCode::UNAUTHORIZED);
        assert_eq!(call(&app, "POST", "/api/shakedown/runs", &[k(&key)], Some(json!({ "app": "UP scaler", "build": "x", "checklistText": LIST }))).await.0, StatusCode::UNAUTHORIZED);

        // Revoked: refused.
        sqlx::query("UPDATE shakedown_keys SET revoked_at = 'now' WHERE label LIKE 'Test build%'").execute(&st.pool).await.unwrap();
        assert_eq!(call(&app, "GET", &format!("/api/shakedown/runs/{id}"), &[k(&key)], None).await.0, StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn part_ids_and_the_cards_count() {
        assert!(row_id_ok("UP-001") && row_id_ok("UP-001.2") && row_id_ok("IO-120.10"));
        for bad in ["UP-001.", "UP-.1", "UP-001.2.3", "UP-001.x", "up-001.1"] {
            assert!(!row_id_ok(bad), "{bad}");
        }
        let list = "## Viewer\n\n- [ ] **UP-001 · Adding** — four · Jason: —\n  - [ ] **UP-001.1 · Window** — w · Jason: —\n  - [ ] **UP-001.2 · Dock** — d · Jason: —\n- [ ] **UP-002 · Import** — i · Jason: —\n- [ ] **UP-003 · Large** — l · Jason: —\n  - [ ] **UP-003.1 · A** — a · Jason: —\n  - [ ] **UP-003.2 · B** — b · Jason: —\n";
        assert_eq!(parts_of(list)["UP-001"], vec!["UP-001.1", "UP-001.2"]);
        let a = |pairs: &[(&str, Value)]| pairs.iter().map(|(k, v)| (k.to_string(), v.clone())).collect();
        // UP-001 by both parts; UP-002 by its own verdict; UP-003 has one part of two: not yet.
        let answers = a(&[
            ("UP-001.1", json!({ "verdict": "passed" })), ("UP-001.2", json!({ "verdict": "failed" })),
            ("UP-002", json!({ "verdict": "partly" })),
            ("UP-003.1", json!({ "verdict": "passed" })),
        ]);
        assert_eq!(count_rows(&answers, list), (2, 0));
        // A row's own answer counts as before, parts or not; a skip only when not answered.
        let answers = a(&[("UP-003", json!({ "verdict": "passed" })), ("UP-002", json!({ "skipped": true })), ("UP-001.1", json!({ "skipped": true }))]);
        assert_eq!(count_rows(&answers, list), (1, 1));
    }

    #[test]
    fn iso_utc_writes_utc() {
        assert_eq!(iso_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(iso_utc(1_760_000_000), "2025-10-09T08:53:20Z");
        assert_eq!(iso_utc(951_782_400), "2000-02-29T00:00:00Z");
    }

    #[tokio::test]
    async fn login_gives_only_an_admin_a_90_day_shakedown_sign_in() {
        let (app, st) = setup().await;
        add_user(&st, "jason", "admin", Some("right-password"), false).await;
        add_user(&st, "geoff", "editor", Some("editor-password"), false).await;
        add_user(&st, "temp", "admin", Some("temp-password"), true).await;

        let (s, v) = call(&app, "POST", "/api/shakedown/login", &[],
            Some(json!({ "username": "jason", "password": "right-password" }))).await;
        assert_eq!(s, StatusCode::OK, "{v}");
        let claims = decode::<ShakedownClaims>(v["token"].as_str().unwrap(),
            &DecodingKey::from_secret(b"test-secret"), &Validation::default()).unwrap().claims;
        assert_eq!(claims.kind, "shakedown");
        let ttl = claims.exp as i64 - now_secs() as i64;
        assert!((TOKEN_TTL_SECS as i64 - 60..=TOKEN_TTL_SECS as i64).contains(&ttl), "about 90 days: {ttl}");
        assert_eq!(v["expires"].as_str().unwrap().len(), 20);

        let wrong = call(&app, "POST", "/api/shakedown/login", &[],
            Some(json!({ "username": "jason", "password": "wrong" }))).await;
        assert_eq!(wrong.0, StatusCode::UNAUTHORIZED);
        let editor = call(&app, "POST", "/api/shakedown/login", &[],
            Some(json!({ "username": "geoff", "password": "editor-password" }))).await;
        assert_eq!(editor.0, StatusCode::FORBIDDEN, "an editor gets no Shakedown sign-in");
        let owed = call(&app, "POST", "/api/shakedown/login", &[],
            Some(json!({ "username": "temp", "password": "temp-password" }))).await;
        assert_eq!(owed.0, StatusCode::FORBIDDEN, "a password still to change gets none");

        assert_eq!(crate::auth::TOKEN_TTL_SECS, 7 * 24 * 3600, "The Livery's own sign-in is unchanged");
    }

    #[tokio::test]
    async fn the_two_sign_ins_never_cross() {
        let (app, st) = setup().await;
        add_user(&st, "jason", "admin", None, false).await;
        let livery = crate::auth::make_token("jason", b"test-secret").unwrap();
        let shakedown = token("jason");

        assert_eq!(call(&app, "GET", "/api/shakedown/apps", &[bearer(&livery)], None).await.0, StatusCode::UNAUTHORIZED);
        assert_eq!(call(&app, "GET", "/api/shakedown/apps", &[bearer(&shakedown)], None).await.0, StatusCode::OK);
        assert_eq!(call(&app, "GET", "/api/shakedown/apps", &[], None).await.0, StatusCode::UNAUTHORIZED);

        // The Livery's own extractor, as its routes use it.
        let livery_app = Router::new()
            .route("/livery", get(|_: crate::auth::AuthUser| async { "ok" }))
            .with_state(st.clone());
        assert_eq!(call(&livery_app, "GET", "/livery", &[bearer(&shakedown)], None).await.0, StatusCode::UNAUTHORIZED,
            "a Shakedown sign-in never edits The Livery");
        assert_eq!(call(&livery_app, "GET", "/livery", &[bearer(&livery)], None).await.0, StatusCode::OK,
            "The Livery's own sign-in works as before");
    }

    #[tokio::test]
    async fn the_role_is_read_from_the_database_each_time() {
        let (app, st) = setup().await;
        add_user(&st, "jason", "admin", None, false).await;
        let t = token("jason");
        assert_eq!(call(&app, "GET", "/api/shakedown/me", &[bearer(&t)], None).await.0, StatusCode::OK);
        sqlx::query("UPDATE users SET role = 'editor' WHERE username = 'jason'").execute(&st.pool).await.unwrap();
        assert_eq!(call(&app, "GET", "/api/shakedown/me", &[bearer(&t)], None).await.0, StatusCode::FORBIDDEN);
        sqlx::query("DELETE FROM users WHERE username = 'jason'").execute(&st.pool).await.unwrap();
        assert_eq!(call(&app, "GET", "/api/shakedown/me", &[bearer(&t)], None).await.0, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn an_upload_key_uploads_and_does_nothing_else() {
        let (app, st) = setup().await;
        add_user(&st, "jason", "admin", None, false).await;
        let t = token("jason");
        let (s, made) = call(&app, "POST", "/api/shakedown/keys", &[bearer(&t)], Some(json!({ "label": "Jason's Mac" }))).await;
        assert_eq!(s, StatusCode::OK, "{made}");
        let key = made["key"].as_str().unwrap().to_string();
        assert!(key.starts_with("sdk_") && key.len() == 47, "{key}");
        let kept: String = sqlx::query_scalar("SELECT key_hash FROM shakedown_keys").fetch_one(&st.pool).await.unwrap();
        assert_eq!(kept, hash_key(&key));
        assert!(!kept.contains(&key[4..]), "only the hash is kept");

        let up = |commit: &'static str| json!({ "commit": commit, "date": "2026-10-12", "checklist": LIST, "map": "" });
        let k = ("x-shakedown-key", key.clone());
        assert_eq!(call(&app, "PUT", "/api/shakedown/apps/UP%20scaler", &[k.clone()], Some(up("aaa1111"))).await.0, StatusCode::OK);
        assert_eq!(call(&app, "PUT", "/api/shakedown/apps/UP%20scaler", &[k.clone()], Some(up("bbb2222"))).await.0, StatusCode::OK);
        let (_, apps) = call(&app, "GET", "/api/shakedown/apps", &[bearer(&t)], None).await;
        assert_eq!(apps["apps"].as_array().unwrap().len(), 1);
        assert_eq!(apps["apps"][0]["name"], "UP scaler");
        assert_eq!(apps["apps"][0]["commit"], "bbb2222", "the newest upload is the one shown");

        assert_eq!(call(&app, "GET", "/api/shakedown/apps", &[bearer(&key)], None).await.0, StatusCode::UNAUTHORIZED,
            "the key isn't a sign-in");
        assert_eq!(call(&app, "GET", "/api/shakedown/runs", &[k.clone()], None).await.0, StatusCode::UNAUTHORIZED,
            "and opens no other route");
        assert_eq!(call(&app, "PUT", "/api/shakedown/apps/UP%20scaler", &[("x-shakedown-key", "sdk_wrong".into())], Some(up("c"))).await.0,
            StatusCode::UNAUTHORIZED);
        assert_eq!(call(&app, "PUT", "/api/shakedown/apps/UP%20scaler", &[], Some(up("c"))).await.0, StatusCode::UNAUTHORIZED);
        assert_eq!(call(&app, "PUT", "/api/shakedown/apps/UP%20scaler", &[bearer(&t)], Some(up("c"))).await.0, StatusCode::UNAUTHORIZED,
            "a sign-in doesn't upload: installs use the key");

        let (_, keys) = call(&app, "GET", "/api/shakedown/keys", &[bearer(&t)], None).await;
        assert!(keys[0].get("key").is_none() && keys[0].get("keyHash").is_none(), "{keys}");
        assert!(keys[0]["lastUsedAt"].is_string());
        let id = made["id"].as_i64().unwrap();
        assert_eq!(call(&app, "DELETE", &format!("/api/shakedown/keys/{id}"), &[bearer(&t)], None).await.0, StatusCode::OK);
        assert_eq!(call(&app, "PUT", "/api/shakedown/apps/UP%20scaler", &[k], Some(up("d"))).await.0, StatusCode::UNAUTHORIZED,
            "a revoked key is refused");
    }

    #[tokio::test]
    async fn the_later_answer_wins_field_by_field() {
        let (app, st) = setup().await;
        add_user(&st, "jason", "admin", None, false).await;
        let t = token("jason");
        let run = new_run(&app, &t, "UP scaler").await;
        let url = format!("/api/shakedown/runs/{}/changes", run["id"].as_str().unwrap());
        let auth = [bearer(&t)];
        let post = |changes: Vec<Value>| call(&app, "POST", &url, &auth, Some(json!({ "changes": changes })));

        // The phone passes UP-001 at 2000; the laptop's Failed, given at 1000,
        // arrives after it (it waited offline): the phone's stays.
        post(vec![change("c1", "row", "UP-001", json!({ "verdict": { "v": "passed", "at": 2000 } }), "phone")]).await;
        let (s, r) = post(vec![change("c2", "row", "UP-001",
            json!({ "verdict": { "v": "failed", "at": 1000 }, "note": { "v": "laptop note", "at": 1000 } }), "laptop")]).await;
        assert_eq!(s, StatusCode::OK, "{r}");
        // A note given at 1500 is later than the laptop's note: it takes the note only.
        post(vec![change("c3", "row", "UP-001", json!({ "note": { "v": "phone note", "at": 1500 } }), "phone")]).await;
        // The same change sent again (a retry) is ignored.
        let (_, again) = post(vec![change("c1", "row", "UP-001", json!({ "verdict": { "v": "passed", "at": 2000 } }), "phone")]).await;
        assert_eq!(again["ignored"], json!(["c1"]));
        assert_eq!(again["applied"], json!([]));

        let (_, whole) = call(&app, "GET", &format!("/api/shakedown/runs/{}", run["id"].as_str().unwrap()), &[bearer(&t)], None).await;
        assert_eq!(whole["answers"]["UP-001"], json!({ "verdict": "passed", "note": "phone note" }));
        assert_eq!(whole["answered"], 1);

        // The same time: the later to arrive wins.
        post(vec![change("c4", "row", "UP-001", json!({ "verdict": { "v": "partly", "at": 2000 } }), "laptop")]).await;
        // A clock far ahead is taken as now (plus a little).
        let far = now_ms() + 3_600_000;
        post(vec![change("c5", "row", "UP-001", json!({ "wish": { "v": "bigger", "at": far } }), "phone")]).await;
        let state: String = sqlx::query_scalar("SELECT state FROM shakedown_answers WHERE target = 'UP-001'")
            .fetch_one(&st.pool).await.unwrap();
        let state: Value = serde_json::from_str(&state).unwrap();
        assert_eq!(state["verdict"]["v"], "partly");
        assert!(state["wish"]["at"].as_i64().unwrap() <= now_ms() + FUTURE_SLACK_MS);

        // A pane note, and the change log keeps every applied change.
        post(vec![change("c6", "pane", "P-05", json!({ "note": { "v": "wrong name", "at": 3000 } }), "laptop")]).await;
        let (_, whole) = call(&app, "GET", &format!("/api/shakedown/runs/{}", run["id"].as_str().unwrap()), &[bearer(&t)], None).await;
        assert_eq!(whole["paneNotes"]["P-05"]["note"], "wrong name");
        let logged: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM shakedown_changes").fetch_one(&st.pool).await.unwrap();
        assert_eq!(logged, 6, "c1 to c6, the retry not twice");
    }

    #[tokio::test]
    async fn changes_are_checked() {
        let (app, st) = setup().await;
        add_user(&st, "jason", "admin", None, false).await;
        let t = token("jason");
        let run = new_run(&app, &t, "UP scaler").await;
        let url = format!("/api/shakedown/runs/{}/changes", run["id"].as_str().unwrap());
        for (why, c) in [
            ("an unknown field", change("x1", "row", "UP-001", json!({ "colour": { "v": "red", "at": 1 } }), "")),
            ("a verdict that isn't one", change("x2", "row", "UP-001", json!({ "verdict": { "v": "great", "at": 1 } }), "")),
            ("a row id that isn't one", change("x3", "row", "../etc", json!({ "note": { "v": "n", "at": 1 } }), "")),
            ("a verdict on a pane", change("x4", "pane", "P-01", json!({ "verdict": { "v": "passed", "at": 1 } }), "")),
            ("no time", change("x5", "row", "UP-001", json!({ "note": { "v": "n" } }), "")),
            ("a note too long", change("x6", "row", "UP-001", json!({ "note": { "v": "n".repeat(MAX_FIELD + 1), "at": 1 } }), "")),
            ("addressed that isn't true or false", change("x8", "row", "UP-001", json!({ "addressed": { "v": "yes", "at": 1 } }), "")),
            ("addressed on a pane", change("x9", "pane", "P-01", json!({ "addressed": { "v": true, "at": 1 } }), "")),
        ] {
            let (s, _) = call(&app, "POST", &url, &[bearer(&t)], Some(json!({ "changes": [c] }))).await;
            assert_eq!(s, StatusCode::BAD_REQUEST, "{why}");
        }
        let (s, _) = call(&app, "POST", "/api/shakedown/runs/no-such-run/changes", &[bearer(&t)],
            Some(json!({ "changes": [change("x7", "row", "UP-001", json!({ "note": { "v": "n", "at": 1 } }), "")] }))).await;
        assert_eq!(s, StatusCode::NOT_FOUND);
        // Addressed (the box, beside the dot) is kept with the verdict.
        let (s, r) = call(&app, "POST", &url, &[bearer(&t)], Some(json!({ "changes": [change("ok1", "row", "UP-001",
            json!({ "verdict": { "v": "failed", "at": 5 }, "addressed": { "v": true, "at": 5 } }), "phone")] }))).await;
        assert_eq!(s, StatusCode::OK, "{r}");
        let (_, whole) = call(&app, "GET", &format!("/api/shakedown/runs/{}", run["id"].as_str().unwrap()), &[bearer(&t)], None).await;
        assert_eq!(whole["answers"]["UP-001"], json!({ "verdict": "failed", "addressed": true }));
        // A part's answer (B-18) is taken like a row's.
        let (s, r) = call(&app, "POST", &url, &[bearer(&t)], Some(json!({ "changes": [change("ok2", "row", "UP-001.1",
            json!({ "verdict": { "v": "passed", "at": 6 } }), "phone")] }))).await;
        assert_eq!(s, StatusCode::OK, "{r}");
        let (_, whole) = call(&app, "GET", &format!("/api/shakedown/runs/{}", run["id"].as_str().unwrap()), &[bearer(&t)], None).await;
        assert_eq!(whole["answers"]["UP-001.1"]["verdict"], "passed");
        assert_eq!(whole["answered"], 1, "UP-001 once, by its own answer; the part isn't counted on its own");
    }

    #[tokio::test]
    async fn a_new_run_closes_the_open_one_and_sends_are_kept() {
        let (app, st) = setup().await;
        add_user(&st, "jason", "admin", None, false).await;
        let t = token("jason");
        let first = new_run(&app, &t, "UP scaler").await;
        let second = new_run(&app, &t, "UP scaler").await;
        new_run(&app, &t, "RhythmIO").await;
        let (_, open) = call(&app, "GET", "/api/shakedown/runs", &[bearer(&t)], None).await;
        let open: Vec<&str> = open.as_array().unwrap().iter().map(|r| r["id"].as_str().unwrap()).collect();
        assert_eq!(open.len(), 2, "one open run per app");
        assert!(open.contains(&second["id"].as_str().unwrap()) && !open.contains(&first["id"].as_str().unwrap()));
        let (_, all) = call(&app, "GET", "/api/shakedown/runs?app=UP%20scaler&all=1", &[bearer(&t)], None).await;
        assert_eq!(all.as_array().unwrap().len(), 2, "the closed run is kept");
        assert!(all[1]["closed"].is_string());

        let id = second["id"].as_str().unwrap();
        let (s, renamed) = call(&app, "PATCH", &format!("/api/shakedown/runs/{id}"), &[bearer(&t)], Some(json!({ "build": "def5678" }))).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(renamed["build"], "def5678");
        let (s, sent) = call(&app, "POST", &format!("/api/shakedown/runs/{id}/send"), &[bearer(&t)],
            Some(json!({ "mode": "sections", "group": "s:Viewer", "current": "UP-001", "label": "Viewer", "device": "laptop" }))).await;
        assert_eq!(s, StatusCode::OK, "{sent}");
        let (_, run) = call(&app, "GET", &format!("/api/shakedown/runs/{id}"), &[bearer(&t)], None).await;
        assert_eq!(run["send"]["group"], "s:Viewer");
        assert_eq!(run["send"]["seq"], sent["seq"]);
        assert_eq!(run["seq"], sent["seq"], "a send takes the run's next seq");
        assert_eq!(run["checklistText"], LIST);
        let bad = call(&app, "POST", &format!("/api/shakedown/runs/{id}/send"), &[bearer(&t)],
            Some(json!({ "mode": "everything", "group": "s:Viewer" }))).await;
        assert_eq!(bad.0, StatusCode::BAD_REQUEST);
        assert_eq!(run["send"]["to"], "phone", "a send with no direction is the phone's, as before");

        // The phone's Show on Laptop: announced, never kept as the phone's next place.
        let (s, back) = call(&app, "POST", &format!("/api/shakedown/runs/{id}/send"), &[bearer(&t)],
            Some(json!({ "mode": "sections", "group": "s:Viewer", "current": "UP-001", "label": "UP-001",
                         "device": "phone", "to": "laptop", "show": "red" }))).await;
        assert_eq!(s, StatusCode::OK, "{back}");
        let (_, run) = call(&app, "GET", &format!("/api/shakedown/runs/{id}"), &[bearer(&t)], None).await;
        assert_eq!(run["send"]["seq"], sent["seq"], "the phone's next place is still the laptop's last send");
        assert_eq!(run["seq"], back["seq"]);
        for wrong in [json!({ "mode": "sections", "group": "s:Viewer", "to": "tablet" }),
                      json!({ "mode": "sections", "group": "s:Viewer", "show": "x".repeat(33) })] {
            let r = call(&app, "POST", &format!("/api/shakedown/runs/{id}/send"), &[bearer(&t)], Some(wrong)).await;
            assert_eq!(r.0, StatusCode::BAD_REQUEST);
        }
    }

    #[tokio::test]
    async fn the_live_channel_carries_each_change() {
        let (app, st) = setup().await;
        add_user(&st, "jason", "admin", None, false).await;
        let t = token("jason");
        let run = new_run(&app, &t, "UP scaler").await;
        let id = run["id"].as_str().unwrap().to_string();

        assert_eq!(call(&app, "GET", "/api/shakedown/events", &[], None).await.0, StatusCode::UNAUTHORIZED);
        let req = Request::builder().uri("/api/shakedown/events").header("authorization", format!("Bearer {t}")).body(Body::empty()).unwrap();
        let res = app.clone().oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.headers()["content-type"], "text/event-stream");
        let mut body = res.into_body();
        let mut seen = String::new();
        async fn more(body: &mut Body, seen: &mut String, want: &str) {
            tokio::time::timeout(Duration::from_secs(5), async {
                while !seen.contains(want) {
                    let frame = body.frame().await.expect("the stream stays open").unwrap();
                    if let Ok(data) = frame.into_data() {
                        seen.push_str(&String::from_utf8_lossy(&data));
                    }
                }
            })
            .await
            .unwrap_or_else(|_| panic!("no {want:?} within 5 s; saw {seen:?}"));
        }
        more(&mut body, &mut seen, "event: hello").await;

        let (s, _) = call(&app, "POST", &format!("/api/shakedown/runs/{id}/changes"), &[bearer(&t)],
            Some(json!({ "changes": [change("e1", "row", "UP-001", json!({ "verdict": { "v": "passed", "at": 10 } }), "phone")] }))).await;
        assert_eq!(s, StatusCode::OK);
        more(&mut body, &mut seen, "event: change").await;
        more(&mut body, &mut seen, "\"verdict\":\"passed\"").await;
        assert!(seen.contains("\"device\":\"phone\""), "{seen}");

        call(&app, "POST", &format!("/api/shakedown/runs/{id}/send"), &[bearer(&t)],
            Some(json!({ "mode": "panes", "group": "p:P-05", "label": "P-05" }))).await;
        more(&mut body, &mut seen, "event: send").await;

        // Several in one post: one event holding them all.
        call(&app, "POST", &format!("/api/shakedown/runs/{id}/changes"), &[bearer(&t)],
            Some(json!({ "changes": [
                change("e2", "row", "UP-001", json!({ "addressed": { "v": true, "at": 20 } }), "phone"),
                change("e3", "row", "UP-002", json!({ "verdict": { "v": "failed", "at": 20 } }), "phone"),
            ] }))).await;
        more(&mut body, &mut seen, "event: changes").await;
        more(&mut body, &mut seen, "\"UP-002\"").await;
        let tail = &seen[seen.find("event: changes").unwrap()..];
        assert!(tail.contains("\"UP-001\"") && !tail.contains("event: change\n"), "{tail}");
    }
}
