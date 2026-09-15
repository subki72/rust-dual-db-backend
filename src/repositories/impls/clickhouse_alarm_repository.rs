use async_trait::async_trait;
use crate::dtos::AlarmItemDto;
use crate::repositories::traits::AlarmRepository;
use crate::utils::AppError;
use clickhouse::Client;

#[derive(Clone)]
pub struct ClickHouseAlarmRepository {
    client: Client,
}

impl ClickHouseAlarmRepository {
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
        // Query sesuai spesifikasi GitHub Issue #2:
        // 1. cleartime = 0 (Mandatory active filter, tidak bisa di-override)
        // 2. Filter rentang tanggal pada kolom firstoccurrence (mendukung format ms maupun detik)
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
