use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use linx::{AppState, build_app};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower::ServiceExt;

#[tokio::test]
async fn shorten_returns_code_and_short() {
    let links = Arc::new(RwLock::new(HashMap::new()));
    let state = AppState::new("http://localhost:3000".to_string(), links);
    let app = build_app(state);

    let body = serde_json::json!({
        "url": "https://example.com",
        "code": "ex"
    })
    .to_string();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/shorten")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let payload: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(payload["code"], "ex");
    assert_eq!(payload["short"], "http://localhost:3000/ex");
}
