//! Unit tests for the generic [`ApiResponse`] success envelope.

use braincode_be::dtos::{ApiResponse, UserResponse};

#[test]
fn api_response_wraps_scalar_and_struct_payloads() {
    // Arrange
    let user = UserResponse {
        id: 7,
        name: "Nadia".to_string(),
        email: "nadia@example.com".to_string(),
    };

    // Act
    let envelope = ApiResponse::success("Pengguna berhasil dibuat!", user.clone());
    let json = serde_json::to_value(&envelope).expect("serialize envelope");

    // Assert
    assert_eq!(envelope.status, "success");
    assert_eq!(envelope.message, "Pengguna berhasil dibuat!");
    assert_eq!(envelope.data, user);
    assert_eq!(json["status"], "success");
    assert_eq!(json["data"]["id"], 7);
}

#[test]
fn api_response_wraps_empty_and_non_empty_collections() {
    // Arrange
    let empty_list: Vec<UserResponse> = vec![];

    // Act
    let envelope = ApiResponse::success("Daftar kosong.", empty_list);
    let json = serde_json::to_value(&envelope).expect("serialize empty list");

    // Assert
    assert_eq!(json["status"], "success");
    assert_eq!(json["data"].as_array().map(Vec::len), Some(0));
}
