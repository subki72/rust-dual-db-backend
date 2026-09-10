use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::sync::Arc;
use crate::dtos::ApiResponse;
use crate::services::traits::UserService;
use crate::utils::AppError;

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
