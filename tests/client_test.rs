// Client 通信层测试

use wakem::client::DaemonClient;
use wakem::ipc::Message;

// ==================== DaemonClient 初始化 ====================

/// 测试客户端初始化（未连接状态）
#[test]
fn test_client_new() {
    let client = DaemonClient::new();
    assert!(!client.is_connected(), "新创建的客户端不应该处于连接状态");
}

/// 测试 Default trait 实现
#[test]
fn test_client_default() {
    let client = DaemonClient::default();
    assert!(!client.is_connected());
}

// ==================== 未连接状态下的错误处理 ====================

/// 测试未连接时调用 get_status 应该返回错误
#[tokio::test]
async fn test_get_status_not_connected() {
    let mut client = DaemonClient::new();
    let result = client.get_status().await;
    assert!(result.is_err(), "未连接时 get_status 应该返回错误");
}

/// 测试未连接时调用 set_active 应该返回错误
#[tokio::test]
async fn test_set_active_not_connected() {
    let mut client = DaemonClient::new();
    let result = client.set_active(true).await;
    assert!(result.is_err(), "未连接时 set_active 应该返回错误");
}

/// 测试未连接时调用 reload_config 应该返回错误
#[tokio::test]
async fn test_reload_config_not_connected() {
    let mut client = DaemonClient::new();
    let result = client.reload_config().await;
    assert!(result.is_err(), "未连接时 reload_config 应该返回错误");
}

/// 测试未连接时调用 save_config 应该返回错误
#[tokio::test]
async fn test_save_config_not_connected() {
    let mut client = DaemonClient::new();
    let result = client.save_config().await;
    assert!(result.is_err(), "未连接时 save_config 应该返回错误");
}

/// 测试未连接时调用 start_macro_recording 应该返回错误
#[tokio::test]
async fn test_start_macro_recording_not_connected() {
    let mut client = DaemonClient::new();
    let result = client.start_macro_recording("test").await;
    assert!(
        result.is_err(),
        "未连接时 start_macro_recording 应该返回错误"
    );
}

/// 测试未连接时调用 stop_macro_recording 应该返回错误
#[tokio::test]
async fn test_stop_macro_recording_not_connected() {
    let mut client = DaemonClient::new();
    let result = client.stop_macro_recording().await;
    assert!(
        result.is_err(),
        "未连接时 stop_macro_recording 应该返回错误"
    );
}

/// 测试未连接时调用 play_macro 应该返回错误
#[tokio::test]
async fn test_play_macro_not_connected() {
    let mut client = DaemonClient::new();
    let result = client.play_macro("test").await;
    assert!(result.is_err(), "未连接时 play_macro 应该返回错误");
}

/// 测试未连接时调用 get_macros 应该返回错误
#[tokio::test]
async fn test_get_macros_not_connected() {
    let mut client = DaemonClient::new();
    let result = client.get_macros().await;
    assert!(result.is_err(), "未连接时 get_macros 应该返回错误");
}

/// 测试未连接时调用 delete_macro 应该返回错误
#[tokio::test]
async fn test_delete_macro_not_connected() {
    let mut client = DaemonClient::new();
    let result = client.delete_macro("test").await;
    assert!(result.is_err(), "未连接时 delete_macro 应该返回错误");
}

/// 测试未连接时调用 bind_macro 应该返回错误
#[tokio::test]
async fn test_bind_macro_not_connected() {
    let mut client = DaemonClient::new();
    let result = client.bind_macro("test", "F5").await;
    assert!(result.is_err(), "未连接时 bind_macro 应该返回错误");
}

/// 测试未连接时调用 register_message_window 应该返回错误
#[tokio::test]
async fn test_register_message_window_not_connected() {
    let mut client = DaemonClient::new();
    let result = client.register_message_window(12345).await;
    assert!(
        result.is_err(),
        "未连接时 register_message_window 应该返回错误"
    );
}

// ==================== IPC 消息序列化验证 ====================

/// 验证客户端使用的消息类型可以正确序列化/反序列化
#[test]
fn test_client_message_serialization() {
    // GetStatus 消息
    let msg = Message::GetStatus;
    let json = serde_json::to_string(&msg).unwrap();
    let deserialized: Message = serde_json::from_str(&json).unwrap();
    assert!(matches!(deserialized, Message::GetStatus));

    // SetActive 消息
    let msg = Message::SetActive { active: true };
    let json = serde_json::to_string(&msg).unwrap();
    let deserialized: Message = serde_json::from_str(&json).unwrap();
    if let Message::SetActive { active } = deserialized {
        assert!(active);
    } else {
        panic!("Expected SetActive message");
    }

    // ReloadConfig 消息
    let msg = Message::ReloadConfig;
    let json = serde_json::to_string(&msg).unwrap();
    let deserialized: Message = serde_json::from_str(&json).unwrap();
    assert!(matches!(deserialized, Message::ReloadConfig));

    // SaveConfig 消息
    let msg = Message::SaveConfig;
    let json = serde_json::to_string(&msg).unwrap();
    let deserialized: Message = serde_json::from_str(&json).unwrap();
    assert!(matches!(deserialized, Message::SaveConfig));

    // StartMacroRecording 消息
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

    // StopMacroRecording 消息
    let msg = Message::StopMacroRecording;
    let json = serde_json::to_string(&msg).unwrap();
    let deserialized: Message = serde_json::from_str(&json).unwrap();
    assert!(matches!(deserialized, Message::StopMacroRecording));

    // PlayMacro 消息
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

    // GetMacros 消息
    let msg = Message::GetMacros;
    let json = serde_json::to_string(&msg).unwrap();
    let deserialized: Message = serde_json::from_str(&json).unwrap();
    assert!(matches!(deserialized, Message::GetMacros));

    // DeleteMacro 消息
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

    // BindMacro 消息
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

    // RegisterMessageWindow 消息
    let msg = Message::RegisterMessageWindow { hwnd: 12345 };
    let json = serde_json::to_string(&msg).unwrap();
    let deserialized: Message = serde_json::from_str(&json).unwrap();
    if let Message::RegisterMessageWindow { hwnd } = deserialized {
        assert_eq!(hwnd, 12345);
    } else {
        panic!("Expected RegisterMessageWindow message");
    }
}

// ==================== 响应消息解析验证 ====================

/// 验证 StatusResponse 响应的解析
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

/// 验证 MacrosList 响应的解析
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

/// 验证 MacroRecordingResult 响应的解析
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

/// 验证 Error 响应的解析
#[test]
fn test_error_response_parsing() {
    let error_messages = vec![
        "Connection failed",
        "Config not found",
        "Unknown error",
        "",
        "特殊字符测试 🎉",
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

// ==================== 连接参数验证 ====================

/// 验证实例地址生成逻辑
#[test]
fn test_instance_address_generation() {
    use wakem::ipc::{get_instance_address, get_instance_port};

    // 实例 0
    assert_eq!(get_instance_port(0), 57427);
    assert_eq!(get_instance_address(0), "127.0.0.1:57427");

    // 实例 1
    assert_eq!(get_instance_port(1), 57428);
    assert_eq!(get_instance_address(1), "127.0.0.1:57428");

    // 实例 5
    assert_eq!(get_instance_port(5), 57432);
    assert_eq!(get_instance_address(5), "127.0.0.1:57432");

    // 大实例 ID
    assert_eq!(get_instance_port(100), 57527);
    assert_eq!(get_instance_address(100), "127.0.0.1:57527");
}
