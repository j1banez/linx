use crate::AppState;
use crate::error::AppError;
use crate::response::AppResponse;
use crate::sql;
use crate::value::{Code, ValidUrl};
use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
};
use serde::Deserialize;
use tracing::instrument;

#[derive(Debug, Deserialize)]
struct ShortenRequest {
    url: String,
    code: Option<String>,
}

pub fn api_routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/shorten", post(shorten))
        .route("/{code}/stats", get(stats))
}

async fn health() -> AppResponse {
    AppResponse::Health
}

#[instrument(skip(state))]
async fn shorten(
    State(state): State<AppState>,
    Json(payload): Json<ShortenRequest>,
) -> Result<AppResponse, AppError> {
    let url = ValidUrl::try_from(payload.url)?;

    let code = match payload.code {
        Some(code) => {
            let code = Code::try_from(code)?;
            sql::insert_link(&state.db, &code, &url).await?;
            code
        }
        None => sql::generate_and_insert(&state.db, &url, state.code_len).await?,
    };

    Ok(AppResponse::Shorten(
        format!("{}/{code}", state.base_url),
        code.to_string(),
    ))
}

#[instrument(skip(state))]
async fn stats(
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> Result<AppResponse, AppError> {
    let code = Code::try_from(code).map_err(|_| AppError::NotFound(Some("code".into())))?;
    let row = sql::fetch_link_stats(&state.db, &code).await?;

    match row {
        Some((url, clicks, created_at, last_accessed_at)) => Ok(AppResponse::new_stats(
            code.to_string(),
            url,
            clicks,
            created_at,
            last_accessed_at,
        )),
        None => Err(AppError::NotFound(Some("code".into()))),
    }
}
