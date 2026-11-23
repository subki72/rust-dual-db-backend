//! Serialization and contract tests for Alarm request and response DTOs.

use braincode_be::dtos::{AlarmActiveRequest, AlarmActiveResponse, AlarmItemDto};

#[test]
fn alarm_active_request_roundtrip_serialization() {
    // Arrange
    let json_input = r#"{"start_date":"2026-09-08","end_date":"2026-09-14"}"#;

    // Act
    let parsed: AlarmActiveRequest =
        serde_json::from_str(json_input).expect("valid request payload");
    let serialized = serde_json::to_string(&parsed).expect("serialize request");

    // Assert
    assert_eq!(parsed.start_date, "2026-09-08");
    assert_eq!(parsed.end_date, "2026-09-14");
    assert!(serialized.contains("\"start_date\":\"2026-09-08\""));
}

#[test]
fn alarm_active_response_preserves_issue_contract_fields() {
    // Arrange
    let response = AlarmActiveResponse {
        status_code: "200".to_string(),
        result: vec![AlarmItemDto {
            identifier: "HASH-999".to_string(),
            alarmname: "LOS".to_string(),
            severity: 5,
            sitecode: "SBY".to_string(),
            node: "OLT-SBY-01".to_string(),
            firstoccurrence: 1_757_308_934_000,
            cleartime: 0,
            acknowledged: "1".to_string(),
        }],
        total: 1,
    };

    // Act
    let encoded = serde_json::to_string(&response).expect("encode response");
    let decoded: AlarmActiveResponse = serde_json::from_str(&encoded).expect("decode response");

    // Assert
    assert_eq!(decoded, response);
    assert_eq!(decoded.result[0].cleartime, 0);
    assert_eq!(decoded.total, 1);
}

#[test]
fn alarm_active_request_fails_when_required_field_missing() {
    // Arrange
    let incomplete_json = r#"{"start_date":"2026-09-08"}"#;

    // Act
    let parsed = serde_json::from_str::<AlarmActiveRequest>(incomplete_json);

    // Assert
    assert!(parsed.is_err());
}
