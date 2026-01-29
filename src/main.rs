use linx::{AppState, build_app};
use std::collections::HashMap;
use std::env;
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

    let base_url = env::var("LINX_URL").unwrap_or_else(|_| "http://127.0.0.1:3000".to_string());
    let state = AppState::new(base_url, links);
    let app = build_app(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
