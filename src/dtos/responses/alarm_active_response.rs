use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, clickhouse::Row)]
pub struct AlarmItemDto {
    pub identifier: String,
    pub alarmname: String,
    pub severity: u8,
    pub sitecode: String,
    pub node: String,
    pub firstoccurrence: i64,
    pub cleartime: i64,
    pub acknowledged: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AlarmActiveResponse {
    pub status_code: String,
    pub result: Vec<AlarmItemDto>,
    pub total: usize,
}
