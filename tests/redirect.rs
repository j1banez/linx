use axum::body::Body;
use axum::http::Request;
use axum::http::StatusCode;
use axum::http::header::LOCATION;
use linx::{AppState, DEFAULT_CODE_LEN, build_app};
use sqlx::sqlite::SqlitePoolOptions;
use tower::ServiceExt;

#[tokio::test]
async fn redirect_returns_location_header() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();

    sqlx::migrate!().run(&pool).await.unwrap();
    sqlx::query("INSERT INTO link (code, url) VALUES (?, ?)")
        .bind("ex")
        .bind("https://example.com")
        .execute(&pool)
        .await
        .unwrap();

    let state = AppState::new("http://localhost:3000".to_string(), pool, DEFAULT_CODE_LEN);
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

    assert_eq!(response.status(), StatusCode::MOVED_PERMANENTLY);
    assert_eq!(
        response.headers().get(LOCATION).unwrap(),
        "https://example.com"
    );
}
