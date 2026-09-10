use clickhouse::Row;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Row, Serialize, Deserialize)]
pub struct UserActivity {
    pub user_id: u64,
    pub action: String,
    pub details: String,
    pub timestamp: u64,
}
