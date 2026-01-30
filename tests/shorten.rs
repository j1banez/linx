use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use linx::{AppState, DEFAULT_CODE_LEN, build_app};
use serde_json::Value;
use sqlx::sqlite::SqlitePoolOptions;
use tower::ServiceExt;

#[tokio::test]
async fn shorten_returns_code_and_short() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();

    sqlx::migrate!().run(&pool).await.unwrap();

    let state = AppState::new("http://localhost:3000".to_string(), pool, DEFAULT_CODE_LEN);
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
    assert_eq!(payload["short_url"], "http://localhost:3000/ex");
}
