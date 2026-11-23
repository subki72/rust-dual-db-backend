//! Request DTO for querying active network alarms within a date range.

use serde::{Deserialize, Serialize};

/// Payload submitted to `POST /alarm_list_active` and `POST /api/v1/alarms/active`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AlarmActiveRequest {
    /// Inclusive lower date bound in ISO `YYYY-MM-DD` format.
    pub start_date: String,
    /// Inclusive upper date bound in ISO `YYYY-MM-DD` format.
    pub end_date: String,
}
