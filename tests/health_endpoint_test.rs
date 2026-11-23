//! Offline HTTP integration tests for the `/health` liveness probe and CORS policy.

use std::sync::Arc;

use async_trait::async_trait;
use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use braincode_be::build_app_router;
use braincode_be::dtos::{
    AlarmActiveRequest, AlarmActiveResponse, CreateUserRequest, UserResponse,
};
use braincode_be::models::UserActivity;
use braincode_be::services::{AlarmService, UserService};
use braincode_be::utils::AppError;
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

/// Minimal no-op user service stub for health endpoint isolation tests.
struct StubUserService;

#[async_trait]
impl UserService for StubUserService {
    async fn register_user(&self, _req: CreateUserRequest) -> Result<UserResponse, AppError> {
        Err(AppError::InternalError("unused stub".to_string()))
    }

    async fn get_all_users(&self) -> Result<Vec<UserResponse>, AppError> {
        Ok(vec![])
    }

    async fn get_recent_activities(&self) -> Result<Vec<UserActivity>, AppError> {
        Ok(vec![])
    }
}

/// Minimal no-op alarm service stub for health endpoint isolation tests.
struct StubAlarmService;

#[async_trait]
impl AlarmService for StubAlarmService {
    async fn get_active_alarms(
        &self,
        _req: AlarmActiveRequest,
    ) -> Result<AlarmActiveResponse, AppError> {
        Ok(AlarmActiveResponse {
            status_code: "200".to_string(),
            result: vec![],
            total: 0,
        })
    }
}

/// Builds a test router configured with explicit CORS origins.
fn create_router(origins: &[&str]) -> axum::Router {
    let origin_list: Vec<String> = origins.iter().map(|s| (*s).to_string()).collect();
    build_app_router(
        Arc::new(StubUserService),
        Arc::new(StubAlarmService),
        &origin_list,
    )
}

#[tokio::test]
async fn health_endpoint_returns_ok_json_payload() {
    // Arrange
    let app = create_router(&["http://localhost:3000"]);
    let request = Request::builder()
        .uri("/health")
        .body(Body::empty())
        .expect("valid request");

    // Act
    let response = app.oneshot(request).await.expect("router response");
    let status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body bytes")
        .to_bytes();
    let payload: Value = serde_json::from_slice(&bytes).expect("valid json");

    // Assert
    assert_eq!(status, StatusCode::OK);
    assert_eq!(payload["status"], "ok");
    assert_eq!(payload["service"], "braincode-be");
}

#[tokio::test]
async fn health_endpoint_enforces_configured_cors_allowlist() {
    // Arrange
    let app = create_router(&["https://dashboard.example.com"]);
    let allowed_req = Request::builder()
        .uri("/health")
        .header(header::ORIGIN, "https://dashboard.example.com")
        .body(Body::empty())
        .expect("valid request");
    let disallowed_req = Request::builder()
        .uri("/health")
        .header(header::ORIGIN, "https://untrusted.example.net")
        .body(Body::empty())
        .expect("valid request");

    // Act
    let allowed_res = app
        .clone()
        .oneshot(allowed_req)
        .await
        .expect("allowed response");
    let disallowed_res = app
        .oneshot(disallowed_req)
        .await
        .expect("disallowed response");

    // Assert
    assert_eq!(
        allowed_res
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .and_then(|v| v.to_str().ok()),
        Some("https://dashboard.example.com")
    );
    assert!(disallowed_res
        .headers()
        .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
        .is_none());
}
