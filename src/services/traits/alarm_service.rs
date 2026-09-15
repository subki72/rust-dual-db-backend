use async_trait::async_trait;
use crate::dtos::{AlarmActiveRequest, AlarmActiveResponse};
use crate::utils::AppError;

#[async_trait]
pub trait AlarmService: Send + Sync {
    async fn get_active_alarms(&self, req: AlarmActiveRequest) -> Result<AlarmActiveResponse, AppError>;
}
