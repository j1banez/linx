use axum::body::Body;
use axum::http::header::LOCATION;
use axum::http::Request;
use linx::{AppState, build_app};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower::ServiceExt;

#[tokio::test]
async fn redirect_returns_location_header() {
    let mut initial = HashMap::new();
    initial.insert("ex".to_string(), "https://example.com".to_string());
    let links = Arc::new(RwLock::new(initial));
    let state = AppState::new("http://localhost:3000".to_string(), links);
    let app = build_app(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/ex")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(response.status().is_redirection());
    assert_eq!(
        response.headers().get(LOCATION).unwrap(),
        "https://example.com"
    );
}
