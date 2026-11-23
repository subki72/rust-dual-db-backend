//! HTTP handler for ClickHouse user activity analytics (`/api/v1/analytics/activities`).

use std::sync::Arc;

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};

use crate::dtos::ApiResponse;
use crate::services::traits::UserService;
use crate::utils::AppError;

/// Handles `GET /api/v1/analytics/activities` to fetch recent user activity events.
///
/// # Errors
/// Propagates [`AppError`] if the ClickHouse analytics query fails.
pub async fn get_activities(
    State(user_service): State<Arc<dyn UserService>>,
) -> Result<impl IntoResponse, AppError> {
    let activities = user_service.get_recent_activities().await?;
    let res = ApiResponse::success(
        "Data analitik aktivitas terbaru dari ClickHouse berhasil diambil.",
        activities,
    );
    Ok((StatusCode::OK, Json(res)))
}
