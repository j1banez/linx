pub mod api;
pub mod error;
pub mod response;
pub mod sql;
pub mod ui;
pub mod value;

use crate::error::AppError;
use crate::response::AppResponse;
use crate::value::Code;
use axum::{
    Router,
    extract::{Path, State},
    routing::get,
};
use lru::LruCache;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tower_http::services::ServeDir;
use tracing::instrument;

pub const DEFAULT_REDIRECT_CACHE_CAPACITY: usize = 10_000;
const DEFAULT_STATS_BATCH_SIZE: i64 = 100;
const DEFAULT_STATS_FLUSH_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Clone)]
pub struct AppState {
    base_url: String,
    db: SqlitePool,
    code_len: usize,
    redirect_cache: Arc<Mutex<LruCache<String, String>>>,
    stats_buffer: Arc<Mutex<HashMap<String, StatsBuffer>>>,
    stats_batch_size: i64,
    stats_flush_interval: Duration,
}

#[derive(Debug, Clone, Copy)]
struct StatsBuffer {
    pending: i64,
    last_flush: Instant,
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
        .route("/{code}", get(redirect))
        .with_state(state)
}

#[instrument(skip(state))]
async fn redirect(
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> Result<AppResponse, AppError> {
    let code = Code::try_from(code).map_err(|_| AppError::NotFound(None))?;

    // Try to get the redirect from the cache
    let cached = state
        .redirect_cache
        .lock()
        .map_err(|_| {
            tracing::error!("redirect cache lock poisoned");
            AppError::Internal
        })?
        .get(code.as_str())
        .cloned();

    if let Some(to) = cached {
        maybe_bump_link_stats(&state, &code);

        return Ok(AppResponse::Redirect(to));
    }

    tracing::debug!(%code, "redirect cache miss");

    // Try to get the redirect from the database
    let Some(to) = sql::fetch_link_url(&state.db, &code).await? else {
        return Err(AppError::NotFound(None));
    };

    state
        .redirect_cache
        .lock()
        .map_err(|_| {
            tracing::error!("redirect cache lock poisoned");
            AppError::Internal
        })?
        .put(code.to_string(), to.clone());

    maybe_bump_link_stats(&state, &code);

    Ok(AppResponse::Redirect(to))
}

// Batch stats updates to reduce SQLite write contention while keeping stats fresh.
// Flush when either a count threshold is reached or a time window elapses.
fn maybe_bump_link_stats(state: &AppState, code: &Code) {
    let now = Instant::now();
    let mut flush_count = None;

    {
        let Ok(mut buffer) = state.stats_buffer.lock() else {
            tracing::error!("stats buffer lock poisoned");
            return;
        };
        let entry = buffer
            .entry(code.to_string())
            .or_insert_with(|| StatsBuffer {
                pending: 0,
                // Ensure first hit flushes immediately to keep stats responsive.
                last_flush: now.checked_sub(state.stats_flush_interval).unwrap_or(now),
            });

        entry.pending += 1;

        if entry.pending >= state.stats_batch_size
            || now.duration_since(entry.last_flush) >= state.stats_flush_interval
        {
            flush_count = Some(entry.pending);
            entry.pending = 0;
            entry.last_flush = now;
        }
    }

    if let Some(count) = flush_count {
        spawn_bump_link_stats(state.db.clone(), code.clone(), count);
    }
}

fn spawn_bump_link_stats(db: SqlitePool, code: Code, count: i64) {
    tokio::spawn(async move {
        if let Err(err) = sql::bump_link_stats_by(&db, &code, count).await {
            tracing::error!(%code, error = ?err, "failed to bump link stats");
        }
    });
}
