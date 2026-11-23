//! Health probe endpoint for liveness monitoring and container healthchecks.

use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;

/// Returns `200 OK` with service status metadata for `GET /health`.
pub async fn health_check() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(json!({
            "status": "ok",
            "service": "braincode-be",
            "message": "Server berjalan dengan normal."
        })),
    )
}
