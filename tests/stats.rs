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
    .unwrap();

    let app = common::new_test_app(pool);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/ex/stats")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let payload: Value = serde_json::from_slice(&body).unwrap();

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
                .uri("/doesnotexist/stats")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
