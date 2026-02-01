use axum::Router;
use linx::{AppState, DEFAULT_CODE_LEN, build_app};
use sqlx::SqlitePool;
use sqlx::sqlite::SqlitePoolOptions;

pub async fn new_test_db() -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:?cache=shared")
        .await
        .unwrap();

    sqlx::migrate!().run(&pool).await.unwrap();

    pool
}

pub fn new_test_app(pool: SqlitePool) -> Router {
    let state = AppState::new("http://localhost:3000".to_string(), pool, DEFAULT_CODE_LEN);
    build_app(state)
}
