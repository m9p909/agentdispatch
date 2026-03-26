use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Internal server error: {0}")]
    Internal(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Request error: {0}")]
    Request(String),

    #[error("Response error: {0}")]
    Response(#[from] axum::http::Error),

}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::Database(e) => {
                tracing::error!("Database error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Database error occurred".to_string(),
                )
            }
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::Validation(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg),
            AppError::Request(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::Internal(msg) => {
                tracing::error!("Internal error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, msg)
            }
            AppError::Response(e) => {
                tracing::error!("Response error: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Response error".to_string())
            }
        };

        (status, Json(ErrorResponse { error: message })).into_response()
    }
}

pub type Result<T> = std::result::Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_not_found_error() {
        let err = AppError::NotFound("Agent not found".to_string());
        assert_eq!(err.to_string(), "Not found: Agent not found");
    }

    #[test]
    fn test_validation_error() {
        let err = AppError::Validation("Name is required".to_string());
        assert_eq!(err.to_string(), "Validation error: Name is required");
    }

    #[test]
    fn test_error_response() {
        let resp = ErrorResponse {
            error: "Test error".to_string(),
        };
        assert_eq!(resp.error, "Test error");
    }
}
