//! Domain model serialization and equality tests for [`User`] and [`UserActivity`].

use braincode_be::models::{User, UserActivity};

#[test]
fn user_activity_roundtrip_json_serialization() {
    // Arrange
    let event = UserActivity {
        user_id: 101,
        action: "user_registered".to_string(),
        details: "Pendaftaran pengguna baru dengan email noc@example.com".to_string(),
        timestamp: 1_757_308_934,
    };

    // Act
    let serialized = serde_json::to_string(&event).expect("serialize UserActivity");
    let deserialized: UserActivity =
        serde_json::from_str(&serialized).expect("deserialize UserActivity");

    // Assert
    assert_eq!(deserialized, event);
    assert_eq!(deserialized.user_id, 101);
    assert_eq!(deserialized.action, "user_registered");
}

#[test]
fn user_model_roundtrip_json_serialization() {
    // Arrange
    let user = User {
        id: 5,
        name: "Tono Prasetyo".to_string(),
        email: "tono@example.com".to_string(),
    };

    // Act
    let serialized = serde_json::to_string(&user).expect("serialize User");
    let deserialized: User = serde_json::from_str(&serialized).expect("deserialize User");

    // Assert
    assert_eq!(deserialized, user);
    assert_eq!(deserialized.id, 5);
}
