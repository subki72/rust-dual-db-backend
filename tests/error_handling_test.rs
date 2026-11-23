//! Unit tests verifying HTTP status codes and safe JSON envelopes for [`AppError`] and [`ErrorHandler`].

use axum::http::{StatusCode, Uri};
use axum::response::IntoResponse;
use braincode_be::utils::AppError;
use braincode_be::ErrorHandler;
use http_body_util::BodyExt;
use serde_json::Value;

/// Extracts the HTTP status code and parsed JSON payload from an [`axum::response::Response`].
async fn unpack_response(response: axum::response::Response) -> (StatusCode, Value) {
    let status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("response bytes")
        .to_bytes();
    let json: Value = serde_json::from_slice(&bytes).expect("valid JSON error envelope");
    (status, json)
}

#[tokio::test]
async fn validation_error_returns_400_with_client_message() {
    // Arrange
    let err = AppError::ValidationError("Format email tidak valid.".to_string());

    // Act
    let (status, body) = unpack_response(err.into_response()).await;

    // Assert
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["status"], "error");
    assert_eq!(body["message"], "Format email tidak valid.");
}

#[tokio::test]
async fn not_found_error_and_fallback_handler_return_404_json() {
    // Arrange
    let uri: Uri = "/api/v1/unknown-resource".parse().expect("valid uri");

    // Act
    let err = ErrorHandler::handle_not_found(uri).await;
    let (status, body) = unpack_response(err.into_response()).await;

    // Assert
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["status"], "error");
    assert!(body["message"]
        .as_str()
        .expect("message string")
        .contains("/api/v1/unknown-resource"));
}

#[tokio::test]
async fn internal_error_hides_sensitive_internal_diagnostics_from_clients() {
    // Arrange
    let err = AppError::InternalError("raw sqlx stack trace details".to_string());

    // Act
    let (status, body) = unpack_response(err.into_response()).await;

    // Assert
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(body["status"], "error");
    let msg = body["message"].as_str().expect("error string");
    assert!(!msg.contains("raw sqlx stack trace"));
}
