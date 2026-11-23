//! Exhaustive unit tests for shared input validation helpers in [`braincode_be::utils::validation`].

use braincode_be::utils::validation::{
    parse_allowed_origins, validate_date_range, validate_email, validate_non_empty,
};
use braincode_be::utils::AppError;
use chrono::NaiveDate;

#[test]
fn validate_non_empty_accepts_text_and_rejects_blank_strings() {
    // Arrange & Act & Assert
    assert!(validate_non_empty("Nama", "Budi").is_ok());
    assert!(matches!(
        validate_non_empty("Nama", ""),
        Err(AppError::ValidationError(msg)) if msg.contains("Nama tidak boleh kosong")
    ));
    assert!(matches!(
        validate_non_empty("Nama", "   \t\n "),
        Err(AppError::ValidationError(_))
    ));
}

#[test]
fn validate_email_checks_syntax_whitespace_and_length_boundaries() {
    // Arrange
    let oversized = format!("{}@example.com", "a".repeat(245));

    // Act & Assert
    assert!(validate_email("engineer@telkomsel.co.id").is_ok());
    assert!(validate_email("user.name+tag@sub.domain.org").is_ok());
    assert!(validate_email("").is_err());
    assert!(validate_email("plainaddress").is_err());
    assert!(validate_email("@domain.com").is_err());
    assert!(validate_email("user@domain").is_err());
    assert!(validate_email("user@domain..com").is_err());
    assert!(validate_email("user name@domain.com").is_err());
    assert!(validate_email("a@b@c.com").is_err());
    assert!(validate_email(&oversized).is_err());
}

#[test]
fn validate_date_range_handles_valid_leap_year_and_error_cases() {
    // Arrange & Act
    let (start, end) =
        validate_date_range("2024-02-28", "2024-02-29").expect("valid leap-year range");
    let invalid_leap = validate_date_range("2025-02-29", "2025-03-01");
    let inverted = validate_date_range("2026-10-02", "2026-10-01");

    // Assert
    assert_eq!(
        start,
        NaiveDate::from_ymd_opt(2024, 2, 28).expect("valid date")
    );
    assert_eq!(
        end,
        NaiveDate::from_ymd_opt(2024, 2, 29).expect("valid date")
    );
    assert!(matches!(invalid_leap, Err(AppError::ValidationError(_))));
    assert!(matches!(inverted, Err(AppError::ValidationError(_))));
}

#[test]
fn parse_allowed_origins_trims_normalizes_and_deduplicates_entries() {
    // Arrange
    let raw = " http://localhost:3000/ , https://noc.example.com/ , , http://localhost:3000 ";

    // Act
    let origins = parse_allowed_origins(raw);

    // Assert
    assert_eq!(
        origins,
        vec![
            "http://localhost:3000".to_string(),
            "https://noc.example.com".to_string()
        ]
    );
}
