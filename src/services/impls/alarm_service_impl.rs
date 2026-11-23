//! Concrete implementation of [`AlarmService`] for querying active network alarms.

use std::sync::Arc;

use async_trait::async_trait;

use crate::dtos::{AlarmActiveRequest, AlarmActiveResponse};
use crate::repositories::traits::AlarmRepository;
use crate::services::traits::AlarmService;
use crate::utils::validation::validate_date_range;
use crate::utils::AppError;

/// Domain service enforcing date-range validation before querying ClickHouse active alarms.
pub struct AlarmServiceImpl {
    alarm_repo: Arc<dyn AlarmRepository>,
}

impl AlarmServiceImpl {
    /// Creates a new [`AlarmServiceImpl`] with an injected [`AlarmRepository`].
    pub fn new(alarm_repo: Arc<dyn AlarmRepository>) -> Self {
        Self { alarm_repo }
    }
}

#[async_trait]
impl AlarmService for AlarmServiceImpl {
    async fn get_active_alarms(
        &self,
        req: AlarmActiveRequest,
    ) -> Result<AlarmActiveResponse, AppError> {
        validate_date_range(&req.start_date, &req.end_date)?;

        let alarms = self
            .alarm_repo
            .find_active_by_date_range(req.start_date.trim(), req.end_date.trim())
            .await?;
        let total = alarms.len();

        tracing::info!(
            start_date = %req.start_date,
            end_date = %req.end_date,
            total,
            "Query alarm aktif selesai"
        );

        Ok(AlarmActiveResponse {
            status_code: "200".to_string(),
            result: alarms,
            total,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dtos::AlarmItemDto;

    struct MockAlarmRepository {
        items: Vec<AlarmItemDto>,
    }

    #[async_trait]
    impl AlarmRepository for MockAlarmRepository {
        async fn find_active_by_date_range(
            &self,
            _start_date: &str,
            _end_date: &str,
        ) -> Result<Vec<AlarmItemDto>, AppError> {
            Ok(self.items.clone())
        }
    }

    #[tokio::test]
    async fn test_get_active_alarms_success() {
        let mock_repo = Arc::new(MockAlarmRepository {
            items: vec![AlarmItemDto {
                identifier: "ABC123".to_string(),
                alarmname: "Multiple ONT Down".to_string(),
                severity: 2,
                sitecode: "RAP".to_string(),
                node: "GPON00-D1-1".to_string(),
                firstoccurrence: 1757308934,
                cleartime: 0,
                acknowledged: "0".to_string(),
            }],
        });

        let service = AlarmServiceImpl::new(mock_repo);
        let req = AlarmActiveRequest {
            start_date: "2026-09-08".to_string(),
            end_date: "2026-09-14".to_string(),
        };

        let res = service.get_active_alarms(req).await.unwrap();
        assert_eq!(res.status_code, "200");
        assert_eq!(res.total, 1);
        assert_eq!(res.result[0].identifier, "ABC123");
        assert_eq!(res.result[0].cleartime, 0);
    }

    #[tokio::test]
    async fn test_get_active_alarms_invalid_date_format() {
        let mock_repo = Arc::new(MockAlarmRepository { items: vec![] });
        let service = AlarmServiceImpl::new(mock_repo);
        let req = AlarmActiveRequest {
            start_date: "invalid-date".to_string(),
            end_date: "2026-09-14".to_string(),
        };

        let err = service.get_active_alarms(req).await.unwrap_err();
        match err {
            AppError::ValidationError(msg) => {
                assert!(msg.contains("Format start_date tidak valid"));
            }
            _ => panic!("Expected ValidationError"),
        }
    }

    #[tokio::test]
    async fn test_get_active_alarms_start_greater_than_end() {
        let mock_repo = Arc::new(MockAlarmRepository { items: vec![] });
        let service = AlarmServiceImpl::new(mock_repo);
        let req = AlarmActiveRequest {
            start_date: "2026-09-20".to_string(),
            end_date: "2026-09-10".to_string(),
        };

        let err = service.get_active_alarms(req).await.unwrap_err();
        match err {
            AppError::ValidationError(msg) => {
                assert!(msg.contains("tidak boleh lebih besar"));
            }
            _ => panic!("Expected ValidationError"),
        }
    }
}
