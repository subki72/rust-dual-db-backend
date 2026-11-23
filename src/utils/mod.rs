//! Shared utility modules for error handling and input validation.

pub mod errors;
pub mod validation;

pub use errors::AppError;
pub use validation::{
    parse_allowed_origins, redact_dsn, validate_date_range, validate_email, validate_non_empty,
};
