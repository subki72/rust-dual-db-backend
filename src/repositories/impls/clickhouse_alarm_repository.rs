//! ClickHouse implementation of [`AlarmRepository`] enforcing the mandatory `cleartime = 0` filter.

use async_trait::async_trait;
use clickhouse::Client;

use crate::dtos::AlarmItemDto;
use crate::repositories::traits::AlarmRepository;
use crate::utils::AppError;

/// Data access object querying active alarms from ClickHouse `active_alarms`.
#[derive(Clone)]
pub struct ClickHouseAlarmRepository {
    client: Client,
}

impl ClickHouseAlarmRepository {
    /// Creates a new [`ClickHouseAlarmRepository`] using the shared ClickHouse [`Client`].
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl AlarmRepository for ClickHouseAlarmRepository {
    async fn find_active_by_date_range(
        &self,
        start_date: &str,
        end_date: &str,
    ) -> Result<Vec<AlarmItemDto>, AppError> {
        // Enforce cleartime = 0 at the repository layer so callers cannot bypass active filtering.
        let query = "
            SELECT
                identifier,
                alarmname,
                severity,
                sitecode,
                node,
                firstoccurrence,
                cleartime,
                acknowledged
            FROM active_alarms
            WHERE cleartime = 0
              AND toDate(intDiv(firstoccurrence, if(firstoccurrence > 10000000000, 1000, 1)))
                  BETWEEN toDate(?) AND toDate(?)
            ORDER BY firstoccurrence DESC
            LIMIT 500
        ";

        let mut cursor = self
            .client
            .query(query)
            .bind(start_date)
            .bind(end_date)
            .fetch::<AlarmItemDto>()?;

        let mut list = Vec::new();
        while let Some(row) = cursor.next().await? {
            list.push(row);
        }

        Ok(list)
    }
}
