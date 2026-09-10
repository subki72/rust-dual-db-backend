use async_trait::async_trait;
use crate::dtos::{CreateUserRequest, UserResponse};
use crate::models::UserActivity;
use crate::utils::AppError;

#[async_trait]
pub trait UserService: Send + Sync {
    async fn register_user(&self, req: CreateUserRequest) -> Result<UserResponse, AppError>;
    async fn get_all_users(&self) -> Result<Vec<UserResponse>, AppError>;
    async fn get_recent_activities(&self) -> Result<Vec<UserActivity>, AppError>;
}
