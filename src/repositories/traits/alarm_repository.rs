use async_trait::async_trait;
use crate::dtos::AlarmItemDto;
use crate::utils::AppError;

#[async_trait]
pub trait AlarmRepository: Send + Sync {
    async fn find_active_by_date_range(
        &self,
        start_date: &str,
        end_date: &str,
    ) -> Result<Vec<AlarmItemDto>, AppError>;
}
