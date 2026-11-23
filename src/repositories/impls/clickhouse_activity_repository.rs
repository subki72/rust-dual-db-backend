//! ClickHouse implementation of [`ActivityRepository`] for analytical audit logging.

use async_trait::async_trait;
use clickhouse::Client;

use crate::models::UserActivity;
use crate::repositories::traits::ActivityRepository;
use crate::utils::AppError;

/// Data access object writing and querying `user_activities` in ClickHouse.
#[derive(Clone)]
pub struct ClickHouseActivityRepository {
    client: Client,
}

impl ClickHouseActivityRepository {
    /// Creates a new [`ClickHouseActivityRepository`] using the shared ClickHouse [`Client`].
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl ActivityRepository for ClickHouseActivityRepository {
    async fn record(&self, activity: &UserActivity) -> Result<(), AppError> {
        let mut insert = self.client.insert("user_activities")?;
        insert.write(activity).await?;
        insert.end().await?;
        Ok(())
    }

    async fn get_recent(&self, limit: u64) -> Result<Vec<UserActivity>, AppError> {
        let mut cursor = self
            .client
            .query("SELECT ?fields FROM user_activities ORDER BY timestamp DESC LIMIT ?")
            .bind(limit)
            .fetch::<UserActivity>()?;

        let mut list = Vec::new();
        while let Some(row) = cursor.next().await? {
            list.push(row);
        }
        Ok(list)
    }
}
