use crate::dtos::{AlarmActiveRequest, AlarmActiveResponse};
use crate::utils::AppError;
use async_trait::async_trait;

#[async_trait]
pub trait AlarmService: Send + Sync {
    async fn get_active_alarms(
        &self,
        req: AlarmActiveRequest,
    ) -> Result<AlarmActiveResponse, AppError>;
}
