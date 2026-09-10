use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum AppError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("ClickHouse error: {0}")]
    ClickHouseError(#[from] clickhouse::error::Error),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Internal server error: {0}")]
    InternalError(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::ValidationError(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            AppError::DatabaseError(err) => {
                tracing::error!("Database error: {:?}", err);
                (StatusCode::INTERNAL_SERVER_ERROR, "Terjadi kesalahan pada database.".to_string())
            }
            AppError::ClickHouseError(err) => {
                tracing::error!("ClickHouse error: {:?}", err);
                (StatusCode::INTERNAL_SERVER_ERROR, "Terjadi kesalahan pada analitik ClickHouse.".to_string())
            }
            AppError::InternalError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
        };

        let body = Json(json!({
            "status": "error",
            "message": message
        }));

        (status, body).into_response()
    }
}
