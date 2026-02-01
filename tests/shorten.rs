use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

mod common;

#[tokio::test]
async fn shorten_returns_code_and_short() {
    let pool = common::new_test_db().await;
    let app = common::new_test_app(pool);

    let body = serde_json::json!({
        "url": "https://example.com",
        "code": "ex"
    })
    .to_string();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/shorten")
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
    assert_eq!(payload["short_url"], "http://localhost:3000/ex");
}
