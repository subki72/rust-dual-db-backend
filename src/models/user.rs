//! Domain entity representing a transactional user stored in PostgreSQL.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Persisted user record mapped from the PostgreSQL `users` table.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, FromRow)]
pub struct User {
    /// Auto-increment primary key.
    pub id: i32,
    /// Full name of the user.
    pub name: String,
    /// Unique email address.
    pub email: String,
}
