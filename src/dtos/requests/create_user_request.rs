//! Request DTO for creating a new user record.

use serde::{Deserialize, Serialize};

/// Payload submitted to `POST /api/v1/users`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreateUserRequest {
    /// Full display name of the user.
    pub name: String,
    /// Unique email address for the user account.
    pub email: String,
}
