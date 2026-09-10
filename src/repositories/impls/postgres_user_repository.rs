use async_trait::async_trait;
use crate::models::User;
use crate::repositories::traits::UserRepository;
use crate::utils::AppError;
use sqlx::PgPool;

#[derive(Clone)]
pub struct PostgresUserRepository {
    pool: PgPool,
}

impl PostgresUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserRepository for PostgresUserRepository {
    async fn create(&self, name: &str, email: &str) -> Result<User, AppError> {
        let user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (name, email)
            VALUES ($1, $2)
            RETURNING id, name, email;
            "#,
        )
        .bind(name)
        .bind(email)
        .fetch_one(&self.pool)
        .await?;

        Ok(user)
    }

    async fn find_all(&self) -> Result<Vec<User>, AppError> {
        let users = sqlx::query_as::<_, User>(
            "SELECT id, name, email FROM users ORDER BY id DESC LIMIT 50;",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(users)
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError> {
        let user = sqlx::query_as::<_, User>(
            "SELECT id, name, email FROM users WHERE email = $1;",
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }
}
