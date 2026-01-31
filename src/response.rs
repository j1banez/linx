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

#[derive(Debug)]
pub enum AppResponse {
    Shorten(String, String),
    Redirect(String),
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
            AppResponse::Health => (StatusCode::OK, "ok").into_response(),
        }
    }
}
