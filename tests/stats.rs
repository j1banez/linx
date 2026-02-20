mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

#[tokio::test]
async fn stats_returns_link_details() {
    let pool = common::new_test_db().await;

    sqlx::query(
        "INSERT INTO link (code, url, clicks, created_at, last_accessed_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind("ex")
    .bind("https://example.com")
    .bind(12i64)
    .bind(1_700_000_000i64)
    .bind(1_700_000_100i64)
    .execute(&pool)
    .await
    .expect("seed stats row should be inserted");

    let app = common::new_test_app(pool);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/ex/stats")
                .body(Body::empty())
                .expect("request should be built"),
        )
        .await
        .expect("stats request should succeed");

    assert_eq!(response.status(), StatusCode::OK);

    let body = response
        .into_body()
        .collect()
        .await
        .expect("response body should be readable")
        .to_bytes();
    let payload: Value = serde_json::from_slice(&body).expect("response body should be valid json");

    assert_eq!(payload["code"], "ex");
    assert_eq!(payload["url"], "https://example.com");
    assert_eq!(payload["clicks"], 12);
    assert_eq!(payload["created_at"], 1_700_000_000);
    assert_eq!(payload["last_accessed_at"], 1_700_000_100);
}

#[tokio::test]
async fn stats_returns_404_for_unknown_code() {
    let pool = common::new_test_db().await;
    let app = common::new_test_app(pool);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/doesnotexist/stats")
                .body(Body::empty())
                .expect("request should be built"),
        )
        .await
        .expect("stats request should succeed");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
