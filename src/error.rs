use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Debug)]
pub enum AppError {
    BadRequest(String),
    Conflict(String),
    NotFound(Option<String>),
    Internal,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::BadRequest(msg) => {
                (StatusCode::BAD_REQUEST, error_response(msg)).into_response()
            }
            AppError::Conflict(msg) => (StatusCode::CONFLICT, error_response(msg)).into_response(),
            AppError::NotFound(entity_name) => {
                let msg = entity_name
                    .map(|name| format!("{name} not found"))
                    .unwrap_or_else(|| "not found".to_string());

                (StatusCode::NOT_FOUND, error_response(msg)).into_response()
            }
            AppError::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                error_response("internal error"),
            )
                .into_response(),
        }
    }
}

fn error_response(msg: impl Into<String>) -> Json<ErrorResponse> {
    Json(ErrorResponse { error: msg.into() })
}
