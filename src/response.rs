use axum::{
    Json,
    http::StatusCode,
    http::header::LOCATION,
    response::{IntoResponse, Response},
};
use serde::Serialize;

#[derive(Serialize)]
struct ShortenResponse {
    short_url: String,
    code: String,
}

#[derive(Debug, Serialize)]
pub struct StatsResponse {
    code: String,
    url: String,
    clicks: i64,
    created_at: i64,
    last_accessed_at: Option<i64>,
}

#[derive(Debug)]
pub enum AppResponse {
    Shorten(String, String),
    Redirect(String),
    Stats(StatsResponse),
    Health,
}

impl IntoResponse for AppResponse {
    fn into_response(self) -> Response {
        match self {
            AppResponse::Shorten(url, code) => (
                StatusCode::OK,
                Json(ShortenResponse {
                    short_url: url,
                    code,
                }),
            )
                .into_response(),
            AppResponse::Redirect(location) => {
                (StatusCode::MOVED_PERMANENTLY, [(LOCATION, location)]).into_response()
            }
            AppResponse::Stats(stats) => (StatusCode::OK, Json(stats)).into_response(),
            AppResponse::Health => (StatusCode::OK, "ok").into_response(),
        }
    }
}

impl AppResponse {
    #[must_use]
    pub fn new_stats(
        code: String,
        url: String,
        clicks: i64,
        created_at: i64,
        last_accessed_at: Option<i64>,
    ) -> Self {
        AppResponse::Stats(StatsResponse {
            code,
            url,
            clicks,
            created_at,
            last_accessed_at,
        })
    }
}
