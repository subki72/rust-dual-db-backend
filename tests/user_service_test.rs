//! Offline unit tests for [`UserServiceImpl`] covering dual-DB orchestration and resilience.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use braincode_be::dtos::CreateUserRequest;
use braincode_be::models::{User, UserActivity};
use braincode_be::repositories::traits::{ActivityRepository, UserRepository};
use braincode_be::services::{UserService, UserServiceImpl};
use braincode_be::utils::AppError;

/// In-memory PostgreSQL user repository mock.
#[derive(Default)]
struct MockUserRepo {
    users: Mutex<Vec<User>>,
}

#[async_trait]
impl UserRepository for MockUserRepo {
    async fn create(&self, name: &str, email: &str) -> Result<User, AppError> {
        let mut guard = self.users.lock().expect("lock poisoned");
        let next_id = i32::try_from(guard.len() + 1).unwrap_or(1);
        let user = User {
            id: next_id,
            name: name.to_string(),
            email: email.to_string(),
        };
        guard.push(user.clone());
        Ok(user)
    }

    async fn find_all(&self) -> Result<Vec<User>, AppError> {
        let guard = self.users.lock().expect("lock poisoned");
        Ok(guard.clone())
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError> {
        let guard = self.users.lock().expect("lock poisoned");
        Ok(guard.iter().find(|u| u.email == email).cloned())
    }
}

/// In-memory ClickHouse activity repository mock supporting simulated failures.
struct MockActivityRepo {
    events: Mutex<Vec<UserActivity>>,
    should_fail: bool,
}

#[async_trait]
impl ActivityRepository for MockActivityRepo {
    async fn record(&self, activity: &UserActivity) -> Result<(), AppError> {
        if self.should_fail {
            return Err(AppError::InternalError(
                "simulated olap failure".to_string(),
            ));
        }
        let mut guard = self.events.lock().expect("lock poisoned");
        guard.push(activity.clone());
        Ok(())
    }

    async fn get_recent(&self, limit: u64) -> Result<Vec<UserActivity>, AppError> {
        let guard = self.events.lock().expect("lock poisoned");
        let count = usize::try_from(limit).unwrap_or(20);
        Ok(guard.iter().take(count).cloned().collect())
    }
}

#[tokio::test]
async fn register_user_persists_user_and_records_clickhouse_activity() {
    // Arrange
    let user_repo = Arc::new(MockUserRepo::default());
    let activity_repo = Arc::new(MockActivityRepo {
        events: Mutex::new(vec![]),
        should_fail: false,
    });
    let service = UserServiceImpl::new(user_repo, activity_repo.clone());

    // Act
    let created = service
        .register_user(CreateUserRequest {
            name: "Budi Santoso".to_string(),
            email: "budi@example.com".to_string(),
        })
        .await
        .expect("registration succeeds");
    let activities = service
        .get_recent_activities()
        .await
        .expect("activities fetched");
    let users = service.get_all_users().await.expect("users fetched");

    // Assert
    assert_eq!(created.id, 1);
    assert_eq!(created.email, "budi@example.com");
    assert_eq!(users.len(), 1);
    assert_eq!(activities.len(), 1);
    assert_eq!(activities[0].action, "user_registered");
}

#[tokio::test]
async fn register_user_rejects_duplicate_email_and_invalid_inputs() {
    // Arrange
    let user_repo = Arc::new(MockUserRepo::default());
    let activity_repo = Arc::new(MockActivityRepo {
        events: Mutex::new(vec![]),
        should_fail: false,
    });
    let service = UserServiceImpl::new(user_repo, activity_repo);

    service
        .register_user(CreateUserRequest {
            name: "Siti Rahma".to_string(),
            email: "siti@example.com".to_string(),
        })
        .await
        .expect("initial registration");

    // Act
    let dup = service
        .register_user(CreateUserRequest {
            name: "Siti Clone".to_string(),
            email: "siti@example.com".to_string(),
        })
        .await;
    let empty_name = service
        .register_user(CreateUserRequest {
            name: "   ".to_string(),
            email: "valid@example.com".to_string(),
        })
        .await;
    let invalid_email = service
        .register_user(CreateUserRequest {
            name: "Andi".to_string(),
            email: "not-an-email".to_string(),
        })
        .await;

    // Assert
    assert!(matches!(dup, Err(AppError::ValidationError(_))));
    assert!(matches!(empty_name, Err(AppError::ValidationError(_))));
    assert!(matches!(invalid_email, Err(AppError::ValidationError(_))));
}

#[tokio::test]
async fn register_user_degrades_gracefully_when_clickhouse_fails() {
    // Arrange
    let user_repo = Arc::new(MockUserRepo::default());
    let activity_repo = Arc::new(MockActivityRepo {
        events: Mutex::new(vec![]),
        should_fail: true,
    });
    let service = UserServiceImpl::new(user_repo, activity_repo);

    // Act
    let created = service
        .register_user(CreateUserRequest {
            name: "Dewi Lestari".to_string(),
            email: "dewi@example.com".to_string(),
        })
        .await
        .expect("OLTP signup must succeed even if OLAP logging fails");

    // Assert
    assert_eq!(created.name, "Dewi Lestari");
    assert_eq!(created.email, "dewi@example.com");
}
