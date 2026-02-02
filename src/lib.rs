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
use sqlx::SqlitePool;
use tower_http::services::ServeDir;
use tracing::instrument;

#[derive(Clone)]
pub struct AppState {
    base_url: String,
    db: SqlitePool,
    code_len: usize,
}

impl AppState {
    pub fn new(base_url: String, db: SqlitePool, code_len: usize) -> Self {
        Self {
            base_url,
            db,
            code_len,
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
    let url = sql::fetch_link_url(&state.db, &code).await?;

    match url {
        Some(to) => {
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
