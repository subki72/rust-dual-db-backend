use async_trait::async_trait;
use crate::models::UserActivity;
use crate::repositories::traits::ActivityRepository;
use crate::utils::AppError;
use clickhouse::Client;

#[derive(Clone)]
pub struct ClickHouseActivityRepository {
    client: Client,
}

impl ClickHouseActivityRepository {
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
