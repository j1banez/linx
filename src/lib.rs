pub mod api;
pub mod error;
pub mod redirect;
pub mod response;
pub mod sql;
pub mod ui;
pub mod value;

use axum::{Router, routing::get};
use lru::LruCache;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tower_http::services::ServeDir;

pub const DEFAULT_REDIRECT_CACHE_CAPACITY: usize = 10_000;
const DEFAULT_STATS_BATCH_SIZE: i64 = 100;
const DEFAULT_STATS_FLUSH_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Clone)]
pub struct AppState {
    pub(crate) base_url: String,
    pub(crate) db: SqlitePool,
    pub(crate) code_len: usize,
    pub(crate) redirect_cache: Arc<Mutex<LruCache<String, String>>>,
    pub(crate) stats_buffer: Arc<Mutex<HashMap<String, redirect::StatsBuffer>>>,
    pub(crate) stats_batch_size: i64,
    pub(crate) stats_flush_interval: Duration,
}

impl AppState {
    #[must_use]
    pub fn new(base_url: String, db: SqlitePool, code_len: usize, cache_capacity: usize) -> Self {
        let cache_capacity = NonZeroUsize::new(cache_capacity)
            .or(NonZeroUsize::new(DEFAULT_REDIRECT_CACHE_CAPACITY))
            .unwrap_or(NonZeroUsize::MIN);
        let redirect_cache = Arc::new(Mutex::new(LruCache::new(cache_capacity)));

        Self {
            base_url,
            db,
            code_len,
            redirect_cache,
            stats_buffer: Arc::new(Mutex::new(HashMap::new())),
            stats_batch_size: DEFAULT_STATS_BATCH_SIZE,
            stats_flush_interval: DEFAULT_STATS_FLUSH_INTERVAL,
        }
    }
}

pub fn build_app(state: AppState) -> Router {
    Router::new()
        // Static assets
        .nest_service("/static", ServeDir::new("public"))
        .route_service(
            "/robots.txt",
            ServeDir::new("public").append_index_html_on_directories(false),
        )
        // API
        .nest("/api", api::api_routes())
        // UI, no prefix
        .merge(ui::ui_routes())
        // Redirect, keep this a the end.
        .route("/{code}", get(redirect::redirect))
        .with_state(state)
}
