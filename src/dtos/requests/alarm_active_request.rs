use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct AlarmActiveRequest {
    pub start_date: String,
    pub end_date: String,
}
