//! Centralized application error types and HTTP response mapping.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

/// Domain and infrastructure errors mapped to safe HTTP JSON responses.
#[derive(Error, Debug)]
pub enum AppError {
    /// Error propagated from PostgreSQL via SQLx.
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    /// Error propagated from ClickHouse OLAP client.
    #[error("ClickHouse error: {0}")]
    ClickHouseError(#[from] clickhouse::error::Error),

    /// Client input failed domain validation rules (HTTP 400).
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// Requested endpoint or entity was not found (HTTP 404).
    #[error("Resource not found: {0}")]
    NotFound(String),

    /// Unexpected server-side fault (HTTP 500).
    #[error("Internal server error: {0}")]
    InternalError(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::ValidationError(msg) => {
                tracing::info!(error = %msg, "Request rejected by validation guard");
                (StatusCode::BAD_REQUEST, msg.clone())
            }
            AppError::NotFound(msg) => {
                tracing::info!(error = %msg, "Resource or route not found");
                (StatusCode::NOT_FOUND, msg.clone())
            }
            AppError::DatabaseError(err) => {
                tracing::error!(error = ?err, "PostgreSQL database operation failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Terjadi kesalahan pada database.".to_string(),
                )
            }
            AppError::ClickHouseError(err) => {
                tracing::error!(error = ?err, "ClickHouse analytical query failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Terjadi kesalahan pada analitik ClickHouse.".to_string(),
                )
            }
            AppError::InternalError(msg) => {
                tracing::error!(error = %msg, "Internal server error encountered");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Terjadi kesalahan internal pada server.".to_string(),
                )
            }
        };

        let body = Json(json!({
            "status": "error",
            "message": message
        }));

        (status, body).into_response()
    }
}
