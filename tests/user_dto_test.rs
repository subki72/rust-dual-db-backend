//! Contract and mapping tests for [`CreateUserRequest`] and [`UserResponse`].

use braincode_be::dtos::{CreateUserRequest, UserResponse};
use braincode_be::models::User;

#[test]
fn user_response_converts_cleanly_from_domain_user() {
    // Arrange
    let domain_user = User {
        id: 42,
        name: "Rina Wijaya".to_string(),
        email: "rina@example.com".to_string(),
    };

    // Act
    let dto = UserResponse::from(domain_user);
    let json = serde_json::to_value(&dto).expect("serialize UserResponse");

    // Assert
    assert_eq!(dto.id, 42);
    assert_eq!(json["name"], "Rina Wijaya");
    assert_eq!(json["email"], "rina@example.com");
}

#[test]
fn create_user_request_deserializes_and_rejects_missing_fields() {
    // Arrange
    let valid_json = r#"{"name":"Arif","email":"arif@example.com"}"#;
    let missing_email = r#"{"name":"Arif"}"#;

    // Act
    let parsed: CreateUserRequest = serde_json::from_str(valid_json).expect("valid payload");
    let failed = serde_json::from_str::<CreateUserRequest>(missing_email);

    // Assert
    assert_eq!(parsed.name, "Arif");
    assert_eq!(parsed.email, "arif@example.com");
    assert!(failed.is_err());
}
