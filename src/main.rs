use linx::{AppState, build_app};
use sqlx::SqlitePool;
use sqlx::sqlite::SqliteConnectOptions;
use std::env;
use std::str::FromStr;

#[tokio::main]
async fn main() {
    let base_url = env::var("LINX_URL").unwrap_or_else(|_| "http://127.0.0.1:3000".to_string());
    let database_url =
        env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://./linx.db".to_string());
    let code_len = env::var("CODE_LEN")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(linx::DEFAULT_CODE_LEN);
    let options = SqliteConnectOptions::from_str(&database_url)
        .expect("Invalid DATABASE_URL format")
        .create_if_missing(true);
    let pool = SqlitePool::connect_with(options)
        .await
        .expect("Failed to open SQLite database");

    sqlx::migrate!().run(&pool).await.unwrap();

    let state = AppState::new(base_url, pool, code_len);
    let app = build_app(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();

    axum::serve(listener, app).await.unwrap();
}
