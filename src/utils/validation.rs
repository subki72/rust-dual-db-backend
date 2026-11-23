//! Input validation, DSN redaction, and CORS origin parsing utilities.

use chrono::NaiveDate;

use crate::utils::AppError;

/// Maximum email address length per RFC 5321.
const MAX_EMAIL_LENGTH: usize = 254;

/// Masks the password segment of a database or service DSN so credentials are never logged.
///
/// # Arguments
/// * `dsn` - Raw connection string such as `postgres://user:pass@host:5432/db`.
///
/// # Returns
/// A sanitized DSN with the password replaced by `***`, or the original string if no
/// userinfo password segment is present.
pub fn redact_dsn(dsn: &str) -> String {
    let Some((scheme, remainder)) = dsn.split_once("://") else {
        return dsn.to_string();
    };

    let boundary = remainder.find(['/', '?']).unwrap_or(remainder.len());
    let (authority, suffix) = remainder.split_at(boundary);

    // Use rsplit_once so passwords containing '@' still separate cleanly from the host.
    let Some((userinfo, host)) = authority.rsplit_once('@') else {
        return dsn.to_string();
    };

    let Some((username, _password)) = userinfo.split_once(':') else {
        return dsn.to_string();
    };

    format!("{scheme}://{username}:***@{host}{suffix}")
}

/// Validates that a required text field contains non-whitespace characters.
///
/// # Arguments
/// * `field` - Human-readable field label included in validation error responses.
/// * `value` - Raw string value to verify.
///
/// # Errors
/// Returns [`AppError::ValidationError`] when `value` is empty or whitespace-only.
pub fn validate_non_empty(field: &str, value: &str) -> Result<(), AppError> {
    if value.trim().is_empty() {
        return Err(AppError::ValidationError(format!(
            "{field} tidak boleh kosong."
        )));
    }
    Ok(())
}

/// Validates basic RFC-style email syntax and length constraints.
///
/// # Arguments
/// * `email` - Candidate email address.
///
/// # Errors
/// Returns [`AppError::ValidationError`] if the email is empty, exceeds 254 characters,
/// contains whitespace, or lacks a valid `local@domain.tld` structure.
pub fn validate_email(email: &str) -> Result<(), AppError> {
    let trimmed = email.trim();
    if trimmed.is_empty()
        || trimmed.len() > MAX_EMAIL_LENGTH
        || trimmed.chars().any(char::is_whitespace)
    {
        return Err(AppError::ValidationError(
            "Format email tidak valid.".to_string(),
        ));
    }

    let Some((local, domain)) = trimmed.split_once('@') else {
        return Err(AppError::ValidationError(
            "Format email tidak valid.".to_string(),
        ));
    };

    let valid_domain = domain.contains('.')
        && !domain.contains('@')
        && domain.split('.').all(|label| !label.is_empty());

    if local.is_empty() || !valid_domain {
        return Err(AppError::ValidationError(
            "Format email tidak valid.".to_string(),
        ));
    }

    Ok(())
}

/// Validates and parses an inclusive ISO `YYYY-MM-DD` date range.
///
/// # Arguments
/// * `start_date` - Start boundary in `YYYY-MM-DD` format.
/// * `end_date` - End boundary in `YYYY-MM-DD` format.
///
/// # Returns
/// Tuple `(start, end)` as parsed [`NaiveDate`] instances.
///
/// # Errors
/// Returns [`AppError::ValidationError`] if either date is malformed or `start_date > end_date`.
pub fn validate_date_range(
    start_date: &str,
    end_date: &str,
) -> Result<(NaiveDate, NaiveDate), AppError> {
    let start = NaiveDate::parse_from_str(start_date.trim(), "%Y-%m-%d").map_err(|_| {
        AppError::ValidationError(format!(
            "Format start_date tidak valid: '{start_date}'. Gunakan format YYYY-MM-DD."
        ))
    })?;

    let end = NaiveDate::parse_from_str(end_date.trim(), "%Y-%m-%d").map_err(|_| {
        AppError::ValidationError(format!(
            "Format end_date tidak valid: '{end_date}'. Gunakan format YYYY-MM-DD."
        ))
    })?;

    if start > end {
        return Err(AppError::ValidationError(
            "start_date tidak boleh lebih besar dari end_date.".to_string(),
        ));
    }

    Ok((start, end))
}

/// Parses a comma-separated list of CORS origins into normalized, deduplicated entries.
///
/// # Arguments
/// * `raw` - Raw environment string such as `"http://localhost:3000, http://localhost:5173"`.
///
/// # Returns
/// Ordered list of trimmed origins with trailing slashes stripped.
pub fn parse_allowed_origins(raw: &str) -> Vec<String> {
    let mut origins = Vec::new();
    for entry in raw.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        // Strip trailing slashes because browsers emit Origin headers without them.
        let normalized = entry.trim_end_matches('/').to_string();
        if !normalized.is_empty() && !origins.contains(&normalized) {
            origins.push(normalized);
        }
    }
    origins
}
