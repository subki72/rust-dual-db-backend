use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::sync::Arc;
use crate::dtos::AlarmActiveRequest;
use crate::services::traits::AlarmService;
use crate::utils::AppError;

/// Handler untuk endpoint POST /alarm_list_active
/// Menerima parameter rentang tanggal: { "start_date": "YYYY-MM-DD", "end_date": "YYYY-MM-DD" }
/// Mengembalikan daftar alarm aktif (cleartime = 0)
pub async fn get_alarm_list_active(
    State(alarm_service): State<Arc<dyn AlarmService>>,
    Json(payload): Json<AlarmActiveRequest>,
) -> Result<impl IntoResponse, AppError> {
    let response = alarm_service.get_active_alarms(payload).await?;
    Ok((StatusCode::OK, Json(response)))
}
