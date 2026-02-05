use linx::{
    AppState, DEFAULT_REDIRECT_CACHE_CAPACITY, build_app,
    validate::{DEFAULT_CODE_LEN, MAX_CODE_LEN, MIN_CODE_LEN},
};
use sqlx::SqlitePool;
use sqlx::sqlite::SqliteConnectOptions;
use std::env;
use std::str::FromStr;
use tracing_subscriber::{EnvFilter, fmt};

#[tokio::main]
async fn main() {
    fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,linx=info")),
        )
        .init();

    let base_url = env::var("LINX_URL").unwrap_or_else(|_| "http://127.0.0.1:3000".to_string());
    let database_url =
        env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://./linx.db".to_string());
    let code_len = env::var("CODE_LEN")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_CODE_LEN);
    let cache_capacity = env::var("REDIRECT_CACHE_CAPACITY")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_REDIRECT_CACHE_CAPACITY);

    let options = SqliteConnectOptions::from_str(&database_url)
        .expect("Invalid DATABASE_URL format")
        .create_if_missing(true)
        .pragma("journal_mode", "WAL")
        .pragma("synchronous", "NORMAL")
        .pragma("busy_timeout", "2000")
        .pragma("foreign_keys", "ON");

    let pool = SqlitePool::connect_with(options)
        .await
        .expect("Failed to open SQLite database");

    if code_len < MIN_CODE_LEN || code_len > MAX_CODE_LEN {
        eprintln!("invalid CODE_LEN={code_len} (expected {MIN_CODE_LEN}-{MAX_CODE_LEN})");
        std::process::exit(2);
    }

    sqlx::migrate!().run(&pool).await.unwrap();

    let state = AppState::new(base_url, pool, code_len, cache_capacity);
    let app = build_app(state);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    tracing::info!("listening on http://{}", listener.local_addr().unwrap());

    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
            // Just so the next shell prompt goes to the next line after a ^C
            eprintln!();
        })
        .await
        .unwrap();
}
