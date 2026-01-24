use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::Redirect,
    routing::get,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

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
        .route("/{code}", get(redirect))
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
