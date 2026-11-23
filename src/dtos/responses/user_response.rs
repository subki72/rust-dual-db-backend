//! Response DTO projecting a domain [`User`] entity for external clients.

use serde::{Deserialize, Serialize};

use crate::models::User;

/// Public API representation of a persisted user account.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserResponse {
    /// Surrogate primary key assigned by PostgreSQL.
    pub id: i32,
    /// User display name.
    pub name: String,
    /// Registered user email address.
    pub email: String,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            name: user.name,
            email: user.email,
        }
    }
}
