//! Analytical activity event persisted in ClickHouse `user_activities`.

use clickhouse::Row;
use serde::{Deserialize, Serialize};

/// Immutable user activity record written to the ClickHouse OLAP store.
#[derive(Debug, Clone, Row, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserActivity {
    /// Associated PostgreSQL user identifier.
    pub user_id: u64,
    /// Event action label (e.g. `user_registered`).
    pub action: String,
    /// Human-readable event description.
    pub details: String,
    /// Unix timestamp in seconds when the event occurred.
    pub timestamp: u64,
}
