pub mod api;
pub mod error;
pub mod response;
pub mod sql;
pub mod ui;
pub mod validate;

use crate::error::AppError;
use crate::response::AppResponse;
use axum::{
    Router,
    extract::{Path, State},
    routing::get,
};
use lru::LruCache;
use sqlx::SqlitePool;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};
use tower_http::services::ServeDir;
use tracing::instrument;

pub const DEFAULT_REDIRECT_CACHE_CAPACITY: usize = 10_000;

#[derive(Clone)]
pub struct AppState {
    base_url: String,
    db: SqlitePool,
    code_len: usize,
    redirect_cache: Arc<Mutex<LruCache<String, String>>>,
}

impl AppState {
    pub fn new(base_url: String, db: SqlitePool, code_len: usize, cache_capacity: usize) -> Self {
        let redirect_cache = Arc::new(Mutex::new(LruCache::new(
            NonZeroUsize::new(cache_capacity).expect("cache capacity must be non-zero"),
        )));

        Self {
            base_url,
            db,
            code_len,
            redirect_cache,
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
    // Try to get the redirect from the cache
    if let Some(to) = state
        .redirect_cache
        .lock()
        .expect("redirect cache lock poisoned")
        .get(&code)
        .cloned()
    {
        let db = state.db.clone();
        let code = code.clone();

        tokio::spawn(async move {
            if let Err(err) = sql::bump_link_stats(&db, &code).await {
                tracing::error!(%code, error = ?err, "failed to bump link stats");
            }
        });

        return Ok(AppResponse::Redirect(to));
    }

    tracing::debug!(%code, "redirect cache miss");

    // Try to get the redirect from the database
    let url = sql::fetch_link_url(&state.db, &code).await?;

    match url {
        Some(to) => {
            state
                .redirect_cache
                .lock()
                .expect("redirect cache lock poisoned")
                .put(code.clone(), to.clone());
            let db = state.db.clone();
            let code = code.clone();

            tokio::spawn(async move {
                if let Err(err) = sql::bump_link_stats(&db, &code).await {
                    tracing::error!(%code, error = ?err, "failed to bump link stats");
                }
            });

            Ok(AppResponse::Redirect(to))
        }
        None => Err(AppError::NotFound(None)),
    }
}
