use async_trait::async_trait;
use crate::dtos::{CreateUserRequest, UserResponse};
use crate::models::UserActivity;
use crate::repositories::traits::{ActivityRepository, UserRepository};
use crate::services::traits::UserService;
use crate::utils::AppError;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct UserServiceImpl {
    user_repo: Arc<dyn UserRepository>,
    activity_repo: Arc<dyn ActivityRepository>,
}

impl UserServiceImpl {
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
        // 1. Validasi Input Bisnis
        if req.name.trim().is_empty() {
            return Err(AppError::ValidationError("Nama tidak boleh kosong.".into()));
        }
        if !req.email.contains('@') {
            return Err(AppError::ValidationError("Format email tidak valid.".into()));
        }

        // 2. Cek apakah email sudah terdaftar di PostgreSQL
        if (self.user_repo.find_by_email(&req.email).await?).is_some() {
            return Err(AppError::ValidationError("Email sudah terdaftar.".into()));
        }

        // 3. Simpan ke PostgreSQL (OLTP)
        let user = self.user_repo.create(&req.name, &req.email).await?;

        // 4. Catat Jejak Analitik ke ClickHouse (OLAP)
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| AppError::InternalError(e.to_string()))?
            .as_secs();

        let activity = UserActivity {
            user_id: user.id as u64,
            action: "user_registered".to_string(),
            details: format!("Pendaftaran pengguna baru dengan email {}", user.email),
            timestamp: now,
        };

        // Simpan log analitik tanpa menggagalkan flow utama jika error
        if let Err(err) = self.activity_repo.record(&activity).await {
            tracing::warn!("Gagal mencatat analitik ClickHouse: {:?}", err);
        }

        Ok(UserResponse::from(user))
    }

    async fn get_all_users(&self) -> Result<Vec<UserResponse>, AppError> {
        let users = self.user_repo.find_all().await?;
        Ok(users.into_iter().map(UserResponse::from).collect())
    }

    async fn get_recent_activities(&self) -> Result<Vec<UserActivity>, AppError> {
        self.activity_repo.get_recent(20).await
    }
}
