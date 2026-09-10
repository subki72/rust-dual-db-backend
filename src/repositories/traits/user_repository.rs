use async_trait::async_trait;
use crate::models::User;
use crate::utils::AppError;

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn create(&self, name: &str, email: &str) -> Result<User, AppError>;
    async fn find_all(&self) -> Result<Vec<User>, AppError>;
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError>;
}
