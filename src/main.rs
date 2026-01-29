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

#[tokio::main]
async fn main() {
    let links = Arc::new(RwLock::new(HashMap::from([
        ("g".to_string(), "http://google.com".to_string()),
        ("e".to_string(), "http://example.com".to_string()),
        ("n".to_string(), "http://netfilx.com".to_string()),
        ("x".to_string(), "http://x.com".to_string()),
    ])));

    let app = Router::new()
        .route("/health", get(health))
        .route("/shorten", post(shorten))
        .route("/r/{code}", get(redirect))
        .with_state(links);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health() -> &'static str {
    "ok"
}

async fn redirect(
    State(links): State<Arc<RwLock<HashMap<String, String>>>>,
    Path(code): Path<String>,
) -> Result<Redirect, StatusCode> {
    let links = links.read().await;

    if let Some(url) = links.get(&code) {
        Ok(Redirect::to(url))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

async fn shorten(
    State(links): State<Arc<RwLock<HashMap<String, String>>>>,
    Json(payload): Json<ShortenRequest>,
) -> Json<ShortenResponse> {
    let mut links = links.write().await;
    let code = payload
        .code
        .unwrap_or_else(|| format!("l{}", links.len() + 1));

    links.insert(code.clone(), payload.url);

    Json(ShortenResponse {
        short: format!("http://127.0.0.1:3000/{code}"),
        code,
    })
}
