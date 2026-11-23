//! Security tests for DSN credential redaction and CORS origin configuration parsing.

use braincode_be::config::{
    DB_CONNECT_TIMEOUT_SECS, DB_MAX_CONNECTIONS, DEFAULT_ALLOWED_ORIGINS, DEFAULT_PORT,
};
use braincode_be::utils::validation::{parse_allowed_origins, redact_dsn};

#[test]
fn redact_dsn_masks_postgres_password_and_preserves_host_and_database() {
    // Arrange
    let dsn = "postgres://admin:dummy_placeholder_pass@db.internal:5432/braincode_db";

    // Act
    let sanitized = redact_dsn(dsn);

    // Assert
    assert_eq!(
        sanitized,
        "postgres://admin:***@db.internal:5432/braincode_db"
    );
    assert!(!sanitized.contains("dummy_placeholder_pass"));
}

#[test]
fn redact_dsn_handles_special_characters_and_query_params() {
    // Arrange
    let complex_dsn =
        "postgres://svc_user:dummy@placeholder:val@10.0.0.5:5432/prod_db?sslmode=require";

    // Act
    let sanitized = redact_dsn(complex_dsn);

    // Assert
    assert_eq!(
        sanitized,
        "postgres://svc_user:***@10.0.0.5:5432/prod_db?sslmode=require"
    );
}

#[test]
fn redact_dsn_leaves_passwordless_urls_unchanged() {
    // Arrange
    let clickhouse_url = "http://localhost:8123";
    let user_only_dsn = "postgres://readonly@localhost:5432/app";

    // Act & Assert
    assert_eq!(redact_dsn(clickhouse_url), "http://localhost:8123");
    assert_eq!(redact_dsn(user_only_dsn), user_only_dsn);
    assert_eq!(redact_dsn(""), "");
}

#[test]
fn default_configuration_constants_enforce_safe_production_bounds() {
    // Arrange & Act
    let default_origins = parse_allowed_origins(DEFAULT_ALLOWED_ORIGINS);

    // Assert
    assert_eq!(DEFAULT_PORT, 3000);
    assert_eq!(DB_MAX_CONNECTIONS, 10);
    assert_eq!(DB_CONNECT_TIMEOUT_SECS, 10);
    assert_eq!(
        default_origins,
        vec![
            "http://localhost:3000".to_string(),
            "http://localhost:5173".to_string()
        ]
    );
}
