use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

/// Errors returned by handlers. Each becomes a JSON body `{"error": "..."}`.
#[derive(Debug)]
pub enum ApiError {
    BadRequest(String),
    Unauthorized,
    Forbidden(String),
    NotFound(String),
    Conflict(String),
    TooManyRequests(String),
    Internal,
}

pub type ApiResult<T> = Result<T, ApiError>;

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::BadRequest(message) => (StatusCode::BAD_REQUEST, message),
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, "missing or invalid token".to_string()),
            ApiError::Forbidden(message) => (StatusCode::FORBIDDEN, message),
            ApiError::NotFound(message) => (StatusCode::NOT_FOUND, message),
            ApiError::Conflict(message) => (StatusCode::CONFLICT, message),
            ApiError::TooManyRequests(message) => (StatusCode::TOO_MANY_REQUESTS, message),
            ApiError::Internal => (StatusCode::INTERNAL_SERVER_ERROR, "internal error".to_string()),
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(error: sqlx::Error) -> Self {
        tracing::error!(%error, "database error");
        ApiError::Internal
    }
}

/// Maps any internal failure (hashing, token signing, a panicked task) to a 500 and logs it.
pub fn internal(error: impl std::fmt::Display) -> ApiError {
    tracing::error!(%error, "internal error");
    ApiError::Internal
}
