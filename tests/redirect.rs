mod common;

use axum::body::Body;
use axum::http::Request;
use axum::http::StatusCode;
use axum::http::header::LOCATION;
use tokio::time::{Duration, sleep};
use tower::ServiceExt;

#[tokio::test]
async fn redirect_returns_location_header() {
    let pool = common::new_test_db().await;

    sqlx::query("INSERT INTO link (code, url) VALUES (?, ?)")
        .bind("ex")
        .bind("https://example.com")
        .execute(&pool)
        .await
        .unwrap();

    let app = common::new_test_app(pool);

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

    assert_eq!(response.status(), StatusCode::MOVED_PERMANENTLY);
    assert_eq!(
        response.headers().get(LOCATION).unwrap(),
        "https://example.com"
    );
}

#[tokio::test]
async fn redirect_bumps_stats() {
    let pool = common::new_test_db().await;

    sqlx::query(
        "INSERT INTO link (code, url, clicks, created_at)
         VALUES (?, ?, 0, unixepoch())",
    )
    .bind("ex")
    .bind("https://example.com")
    .execute(&pool)
    .await
    .unwrap();

    let app = common::new_test_app(pool.clone());

    let _ = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/ex")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Stats are updated async, give the spawned task time to run
    sleep(Duration::from_millis(10)).await;

    let (clicks, last_accessed_at) = sqlx::query_as::<_, (i64, Option<i64>)>(
        "SELECT clicks, last_accessed_at FROM link WHERE code = ?",
    )
    .bind("ex")
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(clicks, 1);
    assert!(last_accessed_at.is_some());
}
