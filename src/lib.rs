use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::Redirect,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct AppState {
    base_url: String,
    links: Arc<RwLock<HashMap<String, String>>>,
}

impl AppState {
    pub fn new(base_url: String, links: Arc<RwLock<HashMap<String, String>>>) -> Self {
        Self { base_url, links }
    }
}

#[derive(Deserialize)]
struct ShortenRequest {
    url: String,
    code: Option<String>,
}

#[derive(Serialize)]
struct ShortenResponse {
    short: String,
    code: String,
}

pub fn build_app(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/shorten", post(shorten))
        .route("/{code}", get(redirect))
        .with_state(state)
}

async fn health() -> &'static str {
    "ok"
}

async fn shorten(
    State(state): State<AppState>,
    Json(payload): Json<ShortenRequest>,
) -> Json<ShortenResponse> {
    let mut links = state.links.write().await;
    let code = payload
        .code
        .unwrap_or_else(|| format!("l{}", links.len() + 1));

    links.insert(code.clone(), payload.url);

    Json(ShortenResponse {
        short: format!("{}/{code}", state.base_url),
        code,
    })
}

async fn redirect(
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> Result<Redirect, StatusCode> {
    let links = state.links.read().await;

    if let Some(url) = links.get(&code) {
        Ok(Redirect::to(url))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}
