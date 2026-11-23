//! Response DTOs returned by the Active Alarm Service endpoints.

use serde::{Deserialize, Serialize};

/// Single active alarm projection retrieved from ClickHouse `active_alarms`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, clickhouse::Row)]
pub struct AlarmItemDto {
    /// Unique alarm hash identifier.
    pub identifier: String,
    /// Descriptive alarm name (e.g. `Multiple ONT Down`).
    pub alarmname: String,
    /// Numeric severity level (`1`..=`5`).
    pub severity: u8,
    /// Site code identifier (e.g. `RAP`).
    pub sitecode: String,
    /// Network element / OLT node identifier.
    pub node: String,
    /// Unix timestamp of initial alarm occurrence.
    pub firstoccurrence: i64,
    /// Clear timestamp (`0` indicates an active alarm).
    pub cleartime: i64,
    /// Acknowledgement status flag (`"0"` or `"1"`).
    pub acknowledged: String,
}

/// Envelope returned by `POST /alarm_list_active` and `POST /api/v1/alarms/active`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AlarmActiveResponse {
    /// String HTTP status code (`"200"` on success).
    pub status_code: String,
    /// Active alarm entries matching the requested date range.
    pub result: Vec<AlarmItemDto>,
    /// Total number of items in `result`.
    pub total: usize,
}
