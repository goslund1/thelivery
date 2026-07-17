//! Shared test helpers. Compiled only for `cargo test`.

use sqlx::sqlite::SqlitePoolOptions;
use sqlx::SqlitePool;

/// Fresh in-memory SQLite with all migrations applied.
///
/// max_connections(1) is load-bearing: each connection to `sqlite::memory:`
/// gets its *own* empty database, so a multi-connection pool would migrate one
/// connection and hand queries to another.
pub async fn test_pool() -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("in-memory sqlite");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrations");
    pool
}
