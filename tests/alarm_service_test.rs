//! Offline unit tests for [`AlarmServiceImpl`] using mock [`AlarmRepository`] implementations.

use std::sync::Arc;

use async_trait::async_trait;
use braincode_be::dtos::{AlarmActiveRequest, AlarmItemDto};
use braincode_be::repositories::traits::AlarmRepository;
use braincode_be::services::{AlarmService, AlarmServiceImpl};
use braincode_be::utils::AppError;

/// Configurable mock repository returning preset alarms or a simulated backend failure.
struct MockAlarmRepo {
    items: Vec<AlarmItemDto>,
    fail_with: Option<String>,
}

#[async_trait]
impl AlarmRepository for MockAlarmRepo {
    async fn find_active_by_date_range(
        &self,
        _start_date: &str,
        _end_date: &str,
    ) -> Result<Vec<AlarmItemDto>, AppError> {
        if let Some(msg) = &self.fail_with {
            return Err(AppError::InternalError(msg.clone()));
        }
        Ok(self.items.clone())
    }
}

/// Helper building a deterministic active alarm fixture.
fn sample_alarm(identifier: &str, severity: u8) -> AlarmItemDto {
    AlarmItemDto {
        identifier: identifier.to_string(),
        alarmname: "ONU is Disconnected".to_string(),
        severity,
        sitecode: "RAP".to_string(),
        node: "GPON00-D1-1".to_string(),
        firstoccurrence: 1_757_308_934_000,
        cleartime: 0,
        acknowledged: "0".to_string(),
    }
}

#[tokio::test]
async fn alarm_service_returns_multiple_active_alarms_and_accurate_total() {
    // Arrange
    let repo = Arc::new(MockAlarmRepo {
        items: vec![sample_alarm("ALM-001", 5), sample_alarm("ALM-002", 4)],
        fail_with: None,
    });
    let service = AlarmServiceImpl::new(repo);
    let req = AlarmActiveRequest {
        start_date: "2026-09-08".to_string(),
        end_date: "2026-09-14".to_string(),
    };

    // Act
    let response = service.get_active_alarms(req).await.expect("valid query");

    // Assert
    assert_eq!(response.status_code, "200");
    assert_eq!(response.total, 2);
    assert_eq!(response.result[0].identifier, "ALM-001");
    assert_eq!(response.result[1].severity, 4);
}

#[tokio::test]
async fn alarm_service_allows_single_day_boundary_range() {
    // Arrange
    let repo = Arc::new(MockAlarmRepo {
        items: vec![],
        fail_with: None,
    });
    let service = AlarmServiceImpl::new(repo);
    let req = AlarmActiveRequest {
        start_date: "2026-09-14".to_string(),
        end_date: "2026-09-14".to_string(),
    };

    // Act
    let response = service
        .get_active_alarms(req)
        .await
        .expect("single-day range allowed");

    // Assert
    assert_eq!(response.total, 0);
    assert!(response.result.is_empty());
}

#[tokio::test]
async fn alarm_service_rejects_inverted_or_malformed_dates() {
    // Arrange
    let repo = Arc::new(MockAlarmRepo {
        items: vec![],
        fail_with: None,
    });
    let service = AlarmServiceImpl::new(repo);

    // Act
    let bad_end = service
        .get_active_alarms(AlarmActiveRequest {
            start_date: "2026-09-08".to_string(),
            end_date: "14-09-2026".to_string(),
        })
        .await;
    let inverted = service
        .get_active_alarms(AlarmActiveRequest {
            start_date: "2026-09-15".to_string(),
            end_date: "2026-09-10".to_string(),
        })
        .await;

    // Assert
    assert!(matches!(bad_end, Err(AppError::ValidationError(_))));
    assert!(matches!(inverted, Err(AppError::ValidationError(_))));
}

#[tokio::test]
async fn alarm_service_propagates_repository_errors() {
    // Arrange
    let repo = Arc::new(MockAlarmRepo {
        items: vec![],
        fail_with: Some("simulated clickhouse timeout".to_string()),
    });
    let service = AlarmServiceImpl::new(repo);

    // Act
    let result = service
        .get_active_alarms(AlarmActiveRequest {
            start_date: "2026-09-08".to_string(),
            end_date: "2026-09-14".to_string(),
        })
        .await;

    // Assert
    assert!(matches!(result, Err(AppError::InternalError(_))));
}
