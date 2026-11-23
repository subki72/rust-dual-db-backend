//! Concrete implementation of [`UserService`] orchestrating PostgreSQL OLTP and ClickHouse OLAP.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;

use crate::dtos::{CreateUserRequest, UserResponse};
use crate::models::UserActivity;
use crate::repositories::traits::{ActivityRepository, UserRepository};
use crate::services::traits::UserService;
use crate::utils::validation::{validate_email, validate_non_empty};
use crate::utils::AppError;

/// Default maximum number of recent activity records returned by analytics queries.
const RECENT_ACTIVITIES_LIMIT: u64 = 20;

/// Domain service combining PostgreSQL user persistence with ClickHouse activity telemetry.
pub struct UserServiceImpl {
    user_repo: Arc<dyn UserRepository>,
    activity_repo: Arc<dyn ActivityRepository>,
}

impl UserServiceImpl {
    /// Creates a new [`UserServiceImpl`] with injected repository implementations.
    pub fn new(
        user_repo: Arc<dyn UserRepository>,
        activity_repo: Arc<dyn ActivityRepository>,
    ) -> Self {
        Self {
            user_repo,
            activity_repo,
        }
    }
}

#[async_trait]
impl UserService for UserServiceImpl {
    async fn register_user(&self, req: CreateUserRequest) -> Result<UserResponse, AppError> {
        validate_non_empty("Nama", &req.name)?;
        validate_email(&req.email)?;

        if self.user_repo.find_by_email(&req.email).await?.is_some() {
            return Err(AppError::ValidationError("Email sudah terdaftar.".into()));
        }

        let user = self
            .user_repo
            .create(req.name.trim(), req.email.trim())
            .await?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| AppError::InternalError(e.to_string()))?
            .as_secs();

        let activity = UserActivity {
            user_id: u64::try_from(user.id).unwrap_or(0),
            action: "user_registered".to_string(),
            details: format!("Pendaftaran pengguna baru dengan email {}", user.email),
            timestamp: now,
        };

        // Analytics logging degrades gracefully so OLAP outages never block user sign-up.
        if let Err(err) = self.activity_repo.record(&activity).await {
            tracing::warn!(user_id = user.id, error = ?err, "Gagal mencatat analitik ClickHouse");
        }

        tracing::info!(user_id = user.id, "Pengguna baru berhasil didaftarkan");
        Ok(UserResponse::from(user))
    }

    async fn get_all_users(&self) -> Result<Vec<UserResponse>, AppError> {
        let users = self.user_repo.find_all().await?;
        tracing::info!(count = users.len(), "Daftar pengguna berhasil diambil");
        Ok(users.into_iter().map(UserResponse::from).collect())
    }

    async fn get_recent_activities(&self) -> Result<Vec<UserActivity>, AppError> {
        let activities = self
            .activity_repo
            .get_recent(RECENT_ACTIVITIES_LIMIT)
            .await?;
        tracing::info!(
            count = activities.len(),
            "Aktivitas terbaru ClickHouse berhasil diambil"
        );
        Ok(activities)
    }
}
