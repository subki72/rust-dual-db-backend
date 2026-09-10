use async_trait::async_trait;
use crate::models::UserActivity;
use crate::utils::AppError;

#[async_trait]
pub trait ActivityRepository: Send + Sync {
    async fn record(&self, activity: &UserActivity) -> Result<(), AppError>;
    async fn get_recent(&self, limit: u64) -> Result<Vec<UserActivity>, AppError>;
}
