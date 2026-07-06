use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("configuration error: {0}")]
    Config(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
    #[error("forbidden")]
    Forbidden,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("not found")]
    NotFound,
    #[error("unauthorized")]
    Unauthorized,
    #[error("too many requests: {0}")]
    TooManyRequests(String),
    #[error("internal error: {0}")]
    Internal(String),
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ErrorBody {
    error: &'static str,
    message: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            AppError::BadRequest(message) => (StatusCode::BAD_REQUEST, "bad_request", message),
            AppError::Config(message) => (StatusCode::INTERNAL_SERVER_ERROR, "config", message),
            AppError::Conflict(message) => (StatusCode::CONFLICT, "conflict", message),
            AppError::Database(err) => {
                tracing::error!(%err, "database request failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "database",
                    "database request failed".to_string(),
                )
            }
            AppError::Migration(err) => {
                tracing::error!(%err, "database migration failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "migration",
                    "database migration failed".to_string(),
                )
            }
            AppError::Forbidden => (StatusCode::FORBIDDEN, "forbidden", "forbidden".to_string()),
            AppError::Io(err) => {
                tracing::error!(%err, "io failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "io",
                    "io failed".to_string(),
                )
            }
            AppError::NotFound => (StatusCode::NOT_FOUND, "not_found", "not found".to_string()),
            AppError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "unauthorized".to_string(),
            ),
            AppError::TooManyRequests(message) => {
                (StatusCode::TOO_MANY_REQUESTS, "too_many_requests", message)
            }
            AppError::Internal(message) => {
                tracing::error!(%message, "internal request failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal",
                    "internal request failed".to_string(),
                )
            }
        };
        (
            status,
            Json(ErrorBody {
                error: code,
                message,
            }),
        )
            .into_response()
    }
}

impl From<std::net::AddrParseError> for AppError {
    fn from(err: std::net::AddrParseError) -> Self {
        AppError::Config(err.to_string())
    }
}

impl From<std::num::ParseIntError> for AppError {
    fn from(err: std::num::ParseIntError) -> Self {
        AppError::Config(err.to_string())
    }
}

impl From<axum::Error> for AppError {
    fn from(err: axum::Error) -> Self {
        AppError::Internal(err.to_string())
    }
}
