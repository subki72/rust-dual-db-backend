//! HTTP handler for active alarm queries (`/alarm_list_active` and `/api/v1/alarms/active`).

use std::sync::Arc;

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};

use crate::dtos::AlarmActiveRequest;
use crate::services::traits::AlarmService;
use crate::utils::AppError;

/// Handles `POST /alarm_list_active` and `POST /api/v1/alarms/active`.
///
/// Accepts a JSON date range `{ "start_date": "YYYY-MM-DD", "end_date": "YYYY-MM-DD" }`
/// and returns active alarms (`cleartime = 0`).
///
/// # Errors
/// Propagates [`AppError`] on invalid date formats, inverted ranges, or ClickHouse query failure.
pub async fn get_alarm_list_active(
    State(alarm_service): State<Arc<dyn AlarmService>>,
    Json(payload): Json<AlarmActiveRequest>,
) -> Result<impl IntoResponse, AppError> {
    let response = alarm_service.get_active_alarms(payload).await?;
    Ok((StatusCode::OK, Json(response)))
}
