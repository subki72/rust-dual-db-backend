use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::sync::Arc;
use crate::dtos::{ApiResponse, CreateUserRequest};
use crate::services::traits::UserService;
use crate::utils::AppError;

pub async fn create_user(
    State(user_service): State<Arc<dyn UserService>>,
    Json(payload): Json<CreateUserRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = user_service.register_user(payload).await?;
    let res = ApiResponse::success("Pengguna berhasil dibuat!", user);
    Ok((StatusCode::CREATED, Json(res)))
}

pub async fn get_users(
    State(user_service): State<Arc<dyn UserService>>,
) -> Result<impl IntoResponse, AppError> {
    let users = user_service.get_all_users().await?;
    let res = ApiResponse::success("Daftar pengguna berhasil diambil.", users);
    Ok((StatusCode::OK, Json(res)))
}
