//! HTTP handlers for user creation and listing (`/api/v1/users`).

use std::sync::Arc;

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};

use crate::dtos::{ApiResponse, CreateUserRequest};
use crate::services::traits::UserService;
use crate::utils::AppError;

/// Handles `POST /api/v1/users` to register a new user in PostgreSQL and record ClickHouse analytics.
///
/// # Errors
/// Propagates [`AppError`] on validation failure, duplicate email, or persistence error.
pub async fn create_user(
    State(user_service): State<Arc<dyn UserService>>,
    Json(payload): Json<CreateUserRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = user_service.register_user(payload).await?;
    let res = ApiResponse::success("Pengguna berhasil dibuat!", user);
    Ok((StatusCode::CREATED, Json(res)))
}

/// Handles `GET /api/v1/users` to retrieve persisted user accounts.
///
/// # Errors
/// Propagates [`AppError`] if the underlying PostgreSQL query fails.
pub async fn get_users(
    State(user_service): State<Arc<dyn UserService>>,
) -> Result<impl IntoResponse, AppError> {
    let users = user_service.get_all_users().await?;
    let res = ApiResponse::success("Daftar pengguna berhasil diambil.", users);
    Ok((StatusCode::OK, Json(res)))
}
