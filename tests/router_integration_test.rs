//! End-to-end offline HTTP router integration tests across all REST endpoints.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use braincode_be::build_app_router;
use braincode_be::dtos::AlarmItemDto;
use braincode_be::models::{User, UserActivity};
use braincode_be::repositories::traits::{ActivityRepository, AlarmRepository, UserRepository};
use braincode_be::services::{AlarmService, AlarmServiceImpl, UserService, UserServiceImpl};
use braincode_be::utils::AppError;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

/// In-memory user store for router-level integration tests.
#[derive(Default)]
struct InMemoryUserRepo {
    users: Mutex<Vec<User>>,
}

#[async_trait]
impl UserRepository for InMemoryUserRepo {
    async fn create(&self, name: &str, email: &str) -> Result<User, AppError> {
        let mut guard = self.users.lock().expect("lock poisoned");
        let id = i32::try_from(guard.len() + 1).unwrap_or(1);
        let user = User {
            id,
            name: name.to_string(),
            email: email.to_string(),
        };
        guard.push(user.clone());
        Ok(user)
    }

    async fn find_all(&self) -> Result<Vec<User>, AppError> {
        Ok(self.users.lock().expect("lock poisoned").clone())
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError> {
        Ok(self
            .users
            .lock()
            .expect("lock poisoned")
            .iter()
            .find(|u| u.email == email)
            .cloned())
    }
}

/// In-memory ClickHouse activity store for router-level integration tests.
#[derive(Default)]
struct InMemoryActivityRepo {
    events: Mutex<Vec<UserActivity>>,
}

#[async_trait]
impl ActivityRepository for InMemoryActivityRepo {
    async fn record(&self, activity: &UserActivity) -> Result<(), AppError> {
        self.events
            .lock()
            .expect("lock poisoned")
            .push(activity.clone());
        Ok(())
    }

    async fn get_recent(&self, limit: u64) -> Result<Vec<UserActivity>, AppError> {
        let count = usize::try_from(limit).unwrap_or(20);
        Ok(self
            .events
            .lock()
            .expect("lock poisoned")
            .iter()
            .take(count)
            .cloned()
            .collect())
    }
}

/// Static ClickHouse alarm repository stub returning active alarm fixtures.
struct StaticAlarmRepo {
    alarms: Vec<AlarmItemDto>,
}

#[async_trait]
impl AlarmRepository for StaticAlarmRepo {
    async fn find_active_by_date_range(
        &self,
        _start_date: &str,
        _end_date: &str,
    ) -> Result<Vec<AlarmItemDto>, AppError> {
        Ok(self.alarms.clone())
    }
}

/// Constructs a fully wired [`axum::Router`] backed by in-memory repositories.
fn create_test_app() -> axum::Router {
    let user_repo = Arc::new(InMemoryUserRepo::default());
    let activity_repo = Arc::new(InMemoryActivityRepo::default());
    let alarm_repo = Arc::new(StaticAlarmRepo {
        alarms: vec![AlarmItemDto {
            identifier: "INT-ALARM-01".to_string(),
            alarmname: "Multiple ONT Down".to_string(),
            severity: 4,
            sitecode: "RAP".to_string(),
            node: "GPON00-D1-1".to_string(),
            firstoccurrence: 1_757_308_934_000,
            cleartime: 0,
            acknowledged: "0".to_string(),
        }],
    });

    let user_service: Arc<dyn UserService> =
        Arc::new(UserServiceImpl::new(user_repo, activity_repo));
    let alarm_service: Arc<dyn AlarmService> = Arc::new(AlarmServiceImpl::new(alarm_repo));
    let origins = vec!["http://localhost:3000".to_string()];

    build_app_router(user_service, alarm_service, &origins)
}

/// Executes a JSON HTTP request against the provided router and parses the JSON response.
async fn send_json_request(
    app: &axum::Router,
    method: Method,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    let payload = match body {
        Some(json_val) => {
            builder = builder.header(header::CONTENT_TYPE, "application/json");
            Body::from(json_val.to_string())
        }
        None => Body::empty(),
    };

    let response = app
        .clone()
        .oneshot(builder.body(payload).expect("valid request"))
        .await
        .expect("router response");
    let status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body bytes")
        .to_bytes();
    let parsed: Value = serde_json::from_slice(&bytes).expect("valid JSON response");
    (status, parsed)
}

#[tokio::test]
async fn users_and_analytics_endpoints_work_end_to_end() {
    // Arrange
    let app = create_test_app();

    // Act
    let (create_status, created) = send_json_request(
        &app,
        Method::POST,
        "/api/v1/users",
        Some(json!({"name": "Budi Santoso", "email": "budi@example.com"})),
    )
    .await;
    let (list_status, listed) = send_json_request(&app, Method::GET, "/api/v1/users", None).await;
    let (act_status, activities) =
        send_json_request(&app, Method::GET, "/api/v1/analytics/activities", None).await;

    // Assert
    assert_eq!(create_status, StatusCode::CREATED);
    assert_eq!(created["status"], "success");
    assert_eq!(created["data"]["email"], "budi@example.com");
    assert_eq!(list_status, StatusCode::OK);
    assert_eq!(listed["data"].as_array().map(Vec::len), Some(1));
    assert_eq!(act_status, StatusCode::OK);
    assert_eq!(activities["data"][0]["action"], "user_registered");
}

#[tokio::test]
async fn alarm_endpoints_serve_primary_and_v1_alias_and_reject_invalid_ranges() {
    // Arrange
    let app = create_test_app();
    let valid_body = json!({"start_date": "2026-09-08", "end_date": "2026-09-14"});
    let invalid_body = json!({"start_date": "2026-09-20", "end_date": "2026-09-10"});

    // Act
    let (legacy_status, legacy_body) = send_json_request(
        &app,
        Method::POST,
        "/alarm_list_active",
        Some(valid_body.clone()),
    )
    .await;
    let (v1_status, v1_body) = send_json_request(
        &app,
        Method::POST,
        "/api/v1/alarms/active",
        Some(valid_body),
    )
    .await;
    let (bad_status, bad_body) =
        send_json_request(&app, Method::POST, "/alarm_list_active", Some(invalid_body)).await;

    // Assert
    assert_eq!(legacy_status, StatusCode::OK);
    assert_eq!(legacy_body["status_code"], "200");
    assert_eq!(legacy_body["total"], 1);
    assert_eq!(v1_status, StatusCode::OK);
    assert_eq!(v1_body["result"][0]["identifier"], "INT-ALARM-01");
    assert_eq!(bad_status, StatusCode::BAD_REQUEST);
    assert_eq!(bad_body["status"], "error");
}

#[tokio::test]
async fn unmatched_route_returns_centralized_json_404_via_error_handler() {
    // Arrange
    let app = create_test_app();

    // Act
    let (status, body) = send_json_request(&app, Method::GET, "/api/v1/nonexistent", None).await;

    // Assert
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["status"], "error");
    assert!(body["message"]
        .as_str()
        .expect("error message")
        .contains("/api/v1/nonexistent"));
}
