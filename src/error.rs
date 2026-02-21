use crate::value::{CodeError, ValidUrlError};
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use thiserror::Error;

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    Conflict(String),
    #[error(
        "{}",
        .0.as_deref()
        .map_or_else(
            || "not found".to_string(),
            |entity| format!("{entity} not found")
        )
    )]
    NotFound(Option<String>),
    #[error("internal error")]
    Internal,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        tracing::error!(error=?self, "request failure");

        let status = self.status_code();

        match self {
            // Simple HTTP 404 response
            Self::NotFound(None) => status.into_response(),
            err => (
                status,
                Json(ErrorResponse {
                    error: err.to_string(),
                }),
            )
                .into_response(),
        }
    }
}

impl AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => AppError::NotFound(None),
            _ => AppError::Internal,
        }
    }
}

impl From<CodeError> for AppError {
    fn from(err: CodeError) -> Self {
        AppError::BadRequest(err.to_string())
    }
}

impl From<ValidUrlError> for AppError {
    fn from(err: ValidUrlError) -> Self {
        AppError::BadRequest(err.to_string())
    }
}
