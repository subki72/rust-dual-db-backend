//! Standard JSON envelope for successful REST API responses.

use serde::{Deserialize, Serialize};

/// Generic API response wrapper carrying status metadata alongside typed data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiResponse<T> {
    /// Machine-readable status indicator (`"success"`).
    pub status: String,
    /// Human-readable operation summary.
    pub message: String,
    /// Domain payload serialized inside the response envelope.
    pub data: T,
}

impl<T> ApiResponse<T> {
    /// Constructs a standardized success envelope around `data`.
    pub fn success(message: impl Into<String>, data: T) -> Self {
        Self {
            status: "success".to_string(),
            message: message.into(),
            data,
        }
    }
}
