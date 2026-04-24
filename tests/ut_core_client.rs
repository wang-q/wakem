// Client communication layer tests

use wakem::client::DaemonClient;
use wakem::ipc::Message;

// ==================== DaemonClient initialization ====================

/// Test client initialization (disconnected state)
#[test]
fn test_client_new() {
    let client = DaemonClient::new();
    assert!(
        !client.is_connected(),
        "Newly created client should not be in connected state"
    );
}

/// Test Default trait implementation
#[test]
fn test_client_default() {
    let client = DaemonClient::default();
    assert!(!client.is_connected());
}

// ==================== Error handling in disconnected state ====================

/// Test get_status should return error when not connected
#[tokio::test]
async fn test_get_status_not_connected() {
    let mut client = DaemonClient::new();
    let result = client.get_status().await;
    assert!(
        result.is_err(),
        "get_status should return error when not connected"
    );
}

/// Test set_active should return error when not connected
#[tokio::test]
async fn test_set_active_not_connected() {
    let mut client = DaemonClient::new();
    let result = client.set_active(true).await;
    assert!(
        result.is_err(),
        "set_active should return error when not connected"
    );
}

/// Test reload_config should return error when not connected
#[tokio::test]
async fn test_reload_config_not_connected() {
    let mut client = DaemonClient::new();
    let result = client.reload_config().await;
    assert!(
        result.is_err(),
        "reload_config should return error when not connected"
    );
}

/// Test save_config should return error when not connected
#[tokio::test]
async fn test_save_config_not_connected() {
    let mut client = DaemonClient::new();
    let result = client.save_config().await;
    assert!(
        result.is_err(),
        "save_config should return error when not connected"
    );
}

/// Test start_macro_recording should return error when not connected
#[tokio::test]
async fn test_start_macro_recording_not_connected() {
    let mut client = DaemonClient::new();
    let result = client.start_macro_recording("test").await;
    assert!(
        result.is_err(),
        "start_macro_recording should return error when not connected"
    );
}

/// Test stop_macro_recording should return error when not connected
#[tokio::test]
async fn test_stop_macro_recording_not_connected() {
    let mut client = DaemonClient::new();
    let result = client.stop_macro_recording().await;
    assert!(
        result.is_err(),
        "stop_macro_recording should return error when not connected"
    );
}

/// Test play_macro should return error when not connected
#[tokio::test]
async fn test_play_macro_not_connected() {
    let mut client = DaemonClient::new();
    let result = client.play_macro("test").await;
    assert!(
        result.is_err(),
        "play_macro should return error when not connected"
    );
}

/// Test get_macros should return error when not connected
#[tokio::test]
async fn test_get_macros_not_connected() {
    let mut client = DaemonClient::new();
    let result = client.get_macros().await;
    assert!(
        result.is_err(),
        "get_macros should return error when not connected"
    );
}

/// Test delete_macro should return error when not connected
#[tokio::test]
async fn test_delete_macro_not_connected() {
    let mut client = DaemonClient::new();
    let result = client.delete_macro("test").await;
    assert!(
        result.is_err(),
        "delete_macro should return error when not connected"
    );
}

/// Test bind_macro should return error when not connected
#[tokio::test]
async fn test_bind_macro_not_connected() {
    let mut client = DaemonClient::new();
    let result = client.bind_macro("test", "F5").await;
    assert!(
        result.is_err(),
        "bind_macro should return error when not connected"
    );
}

/// Test register_message_window should return error when not connected
#[tokio::test]
async fn test_register_message_window_not_connected() {
    let mut client = DaemonClient::new();
    let result = client.register_message_window(12345).await;
    assert!(
        result.is_err(),
        "register_message_window should return error when not connected"
    );
}

// ==================== IPC message serialization validation ====================

/// Verify that message types used by the client can be correctly serialized/deserialized
#[test]
fn test_client_message_serialization() {
    // GetStatus message
    let msg = Message::GetStatus;
    let json = serde_json::to_string(&msg).unwrap();
    let deserialized: Message = serde_json::from_str(&json).unwrap();
    assert!(matches!(deserialized, Message::GetStatus));

    // SetActive message
    let msg = Message::SetActive { active: true };
    let json = serde_json::to_string(&msg).unwrap();
    let deserialized: Message = serde_json::from_str(&json).unwrap();
    if let Message::SetActive { active } = deserialized {
        assert!(active);
    } else {
        panic!("Expected SetActive message");
    }

    // ReloadConfig message
    let msg = Message::ReloadConfig;
    let json = serde_json::to_string(&msg).unwrap();
    let deserialized: Message = serde_json::from_str(&json).unwrap();
    assert!(matches!(deserialized, Message::ReloadConfig));

    // SaveConfig message
    let msg = Message::SaveConfig;
    let json = serde_json::to_string(&msg).unwrap();
    let deserialized: Message = serde_json::from_str(&json).unwrap();
    assert!(matches!(deserialized, Message::SaveConfig));

    // StartMacroRecording message
    let msg = Message::StartMacroRecording {
        name: "test".to_string(),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let deserialized: Message = serde_json::from_str(&json).unwrap();
    if let Message::StartMacroRecording { name } = deserialized {
        assert_eq!(name, "test");
    } else {
        panic!("Expected StartMacroRecording message");
    }

    // StopMacroRecording message
    let msg = Message::StopMacroRecording;
    let json = serde_json::to_string(&msg).unwrap();
    let deserialized: Message = serde_json::from_str(&json).unwrap();
    assert!(matches!(deserialized, Message::StopMacroRecording));

    // PlayMacro message
    let msg = Message::PlayMacro {
        name: "macro1".to_string(),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let deserialized: Message = serde_json::from_str(&json).unwrap();
    if let Message::PlayMacro { name } = deserialized {
        assert_eq!(name, "macro1");
    } else {
        panic!("Expected PlayMacro message");
    }

    // GetMacros message
    let msg = Message::GetMacros;
    let json = serde_json::to_string(&msg).unwrap();
    let deserialized: Message = serde_json::from_str(&json).unwrap();
    assert!(matches!(deserialized, Message::GetMacros));

    // DeleteMacro message
    let msg = Message::DeleteMacro {
        name: "old_macro".to_string(),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let deserialized: Message = serde_json::from_str(&json).unwrap();
    if let Message::DeleteMacro { name } = deserialized {
        assert_eq!(name, "old_macro");
    } else {
        panic!("Expected DeleteMacro message");
    }

    // BindMacro message
    let msg = Message::BindMacro {
        macro_name: "my_macro".to_string(),
        trigger: "F5".to_string(),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let deserialized: Message = serde_json::from_str(&json).unwrap();
    if let Message::BindMacro {
        macro_name,
        trigger,
    } = deserialized
    {
        assert_eq!(macro_name, "my_macro");
        assert_eq!(trigger, "F5");
    } else {
        panic!("Expected BindMacro message");
    }

    // RegisterMessageWindow message
    let msg = Message::RegisterMessageWindow { hwnd: 12345 };
    let json = serde_json::to_string(&msg).unwrap();
    let deserialized: Message = serde_json::from_str(&json).unwrap();
    if let Message::RegisterMessageWindow { hwnd } = deserialized {
        assert_eq!(hwnd, 12345);
    } else {
        panic!("Expected RegisterMessageWindow message");
    }
}

// ==================== Response message parsing validation ====================

/// Verify parsing of StatusResponse response
#[test]
fn test_status_response_parsing() {
    let response = Message::StatusResponse {
        active: true,
        config_loaded: false,
    };

    let json = serde_json::to_string(&response).unwrap();
    let parsed: Message = serde_json::from_str(&json).unwrap();

    match parsed {
        Message::StatusResponse {
            active,
            config_loaded,
        } => {
            assert!(active);
            assert!(!config_loaded);
        }
        _ => panic!("Expected StatusResponse"),
    }
}

/// Verify parsing of MacrosList response
#[test]
fn test_macros_list_response_parsing() {
    let response = Message::MacrosList {
        macros: vec![
            "macro1".to_string(),
            "macro2".to_string(),
            "macro3".to_string(),
        ],
    };

    let json = serde_json::to_string(&response).unwrap();
    let parsed: Message = serde_json::from_str(&json).unwrap();

    match parsed {
        Message::MacrosList { macros } => {
            assert_eq!(macros.len(), 3);
            assert!(macros.contains(&"macro1".to_string()));
            assert!(macros.contains(&"macro2".to_string()));
            assert!(macros.contains(&"macro3".to_string()));
        }
        _ => panic!("Expected MacrosList"),
    }
}

/// Verify parsing of MacroRecordingResult response
#[test]
fn test_macro_recording_result_parsing() {
    let response = Message::MacroRecordingResult {
        name: "test_macro".to_string(),
        action_count: 10,
    };

    let json = serde_json::to_string(&response).unwrap();
    let parsed: Message = serde_json::from_str(&json).unwrap();

    match parsed {
        Message::MacroRecordingResult { name, action_count } => {
            assert_eq!(name, "test_macro");
            assert_eq!(action_count, 10);
        }
        _ => panic!("Expected MacroRecordingResult"),
    }
}

/// Verify parsing of Error response
#[test]
fn test_error_response_parsing() {
    let error_messages = vec![
        "Connection failed",
        "Config not found",
        "Unknown error",
        "",
        "Special characters测试 🎉",
    ];

    for msg in error_messages {
        let response = Message::Error {
            message: msg.to_string(),
        };
        let json = serde_json::to_string(&response).unwrap();
        let parsed: Message = serde_json::from_str(&json).unwrap();

        if let Message::Error { message } = parsed {
            assert_eq!(message, msg);
        } else {
            panic!("Expected Error response");
        }
    }
}

// ==================== Connection parameter validation ====================

/// Verify instance address generation logic
#[test]
fn test_instance_address_generation() {
    use wakem::ipc::{get_instance_address, get_instance_port};

    // Instance 0
    assert_eq!(get_instance_port(0), 57427);
    assert_eq!(get_instance_address(0), "127.0.0.1:57427");

    // Instance 1
    assert_eq!(get_instance_port(1), 57428);
    assert_eq!(get_instance_address(1), "127.0.0.1:57428");

    // Instance 5
    assert_eq!(get_instance_port(5), 57432);
    assert_eq!(get_instance_address(5), "127.0.0.1:57432");

    // Large instance ID
    assert_eq!(get_instance_port(100), 57527);
    assert_eq!(get_instance_address(100), "127.0.0.1:57527");
}
