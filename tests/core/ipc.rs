// IPC 通信测试

use wakem::config::Config;
use wakem::ipc::{get_instance_address, get_instance_port, Message};

/// 测试 IPC 消息序列化
#[test]
fn test_ipc_message_serialization() {
    // 测试消息序列化和反序列化
    let messages = vec![
        Message::ReloadConfig,
        Message::GetStatus,
        Message::SetActive { active: true },
        Message::SetActive { active: false },
        Message::SaveConfig,
        Message::Ping,
        Message::Pong,
        Message::Success,
    ];

    for msg in messages {
        let json = serde_json::to_string(&msg).expect("Failed to serialize message");
        let deserialized: Message =
            serde_json::from_str(&json).expect("Failed to deserialize message");

        // 验证反序列化后的消息类型相同
        match (&msg, &deserialized) {
            (Message::ReloadConfig, Message::ReloadConfig) => {}
            (Message::GetStatus, Message::GetStatus) => {}
            (Message::SetActive { active: a1 }, Message::SetActive { active: a2 }) => {
                assert_eq!(a1, a2)
            }
            (Message::SaveConfig, Message::SaveConfig) => {}
            (Message::Ping, Message::Ping) => {}
            (Message::Pong, Message::Pong) => {}
            (Message::Success, Message::Success) => {}
            _ => panic!("Message type mismatch after deserialization"),
        }
    }
}

/// 测试状态响应消息
#[test]
fn test_status_response_message() {
    let msg = Message::StatusResponse {
        active: true,
        config_loaded: true,
    };

    let json = serde_json::to_string(&msg).expect("Failed to serialize");
    let deserialized: Message =
        serde_json::from_str(&json).expect("Failed to deserialize");

    if let Message::StatusResponse {
        active,
        config_loaded,
    } = deserialized
    {
        assert!(active);
        assert!(config_loaded);
    } else {
        panic!("Expected StatusResponse message");
    }
}

/// 测试错误消息
#[test]
fn test_error_message() {
    let msg = Message::Error {
        message: "Test error".to_string(),
    };

    let json = serde_json::to_string(&msg).expect("Failed to serialize");
    let deserialized: Message =
        serde_json::from_str(&json).expect("Failed to deserialize");

    if let Message::Error { message } = deserialized {
        assert_eq!(message, "Test error");
    } else {
        panic!("Expected Error message");
    }
}

/// 测试宏相关消息
#[test]
fn test_macro_messages() {
    // 测试开始录制
    let msg = Message::StartMacroRecording {
        name: "test_macro".to_string(),
    };
    let json = serde_json::to_string(&msg).expect("Failed to serialize");
    let deserialized: Message =
        serde_json::from_str(&json).expect("Failed to deserialize");

    if let Message::StartMacroRecording { name } = deserialized {
        assert_eq!(name, "test_macro");
    } else {
        panic!("Expected StartMacroRecording message");
    }

    // 测试录制结果
    let msg = Message::MacroRecordingResult {
        name: "test_macro".to_string(),
        action_count: 5,
    };
    let json = serde_json::to_string(&msg).expect("Failed to serialize");
    let deserialized: Message =
        serde_json::from_str(&json).expect("Failed to deserialize");

    if let Message::MacroRecordingResult { name, action_count } = deserialized {
        assert_eq!(name, "test_macro");
        assert_eq!(action_count, 5);
    } else {
        panic!("Expected MacroRecordingResult message");
    }

    // 测试宏列表
    let msg = Message::MacrosList {
        macros: vec!["macro1".to_string(), "macro2".to_string()],
    };
    let json = serde_json::to_string(&msg).expect("Failed to serialize");
    let deserialized: Message =
        serde_json::from_str(&json).expect("Failed to deserialize");

    if let Message::MacrosList { macros } = deserialized {
        assert_eq!(macros.len(), 2);
        assert_eq!(macros[0], "macro1");
        assert_eq!(macros[1], "macro2");
    } else {
        panic!("Expected MacrosList message");
    }
}

/// 测试配置消息
#[test]
fn test_config_message() {
    let config = Config::default();
    let msg = Message::SetConfig {
        config: Box::new(config),
    };

    let json = serde_json::to_string(&msg).expect("Failed to serialize");
    let deserialized: Message =
        serde_json::from_str(&json).expect("Failed to deserialize");

    if let Message::SetConfig { .. } = deserialized {
        // 配置对象已反序列化
    } else {
        panic!("Expected SetConfig message");
    }
}

/// 测试配置错误消息
#[test]
fn test_config_error_message() {
    let msg = Message::ConfigError {
        error: "Invalid config".to_string(),
    };

    let json = serde_json::to_string(&msg).expect("Failed to serialize");
    let deserialized: Message =
        serde_json::from_str(&json).expect("Failed to deserialize");

    if let Message::ConfigError { error } = deserialized {
        assert_eq!(error, "Invalid config");
    } else {
        panic!("Expected ConfigError message");
    }
}

/// 测试绑定宏消息
#[test]
fn test_bind_macro_message() {
    let msg = Message::BindMacro {
        macro_name: "my_macro".to_string(),
        trigger: "F1".to_string(),
    };

    let json = serde_json::to_string(&msg).expect("Failed to serialize");
    let deserialized: Message =
        serde_json::from_str(&json).expect("Failed to deserialize");

    if let Message::BindMacro {
        macro_name,
        trigger,
    } = deserialized
    {
        assert_eq!(macro_name, "my_macro");
        assert_eq!(trigger, "F1");
    } else {
        panic!("Expected BindMacro message");
    }
}

/// 测试播放宏消息
#[test]
fn test_play_macro_message() {
    let msg = Message::PlayMacro {
        name: "test_macro".to_string(),
    };

    let json = serde_json::to_string(&msg).expect("Failed to serialize");
    let deserialized: Message =
        serde_json::from_str(&json).expect("Failed to deserialize");

    if let Message::PlayMacro { name } = deserialized {
        assert_eq!(name, "test_macro");
    } else {
        panic!("Expected PlayMacro message");
    }
}

/// 测试删除宏消息
#[test]
fn test_delete_macro_message() {
    let msg = Message::DeleteMacro {
        name: "old_macro".to_string(),
    };

    let json = serde_json::to_string(&msg).expect("Failed to serialize");
    let deserialized: Message =
        serde_json::from_str(&json).expect("Failed to deserialize");

    if let Message::DeleteMacro { name } = deserialized {
        assert_eq!(name, "old_macro");
    } else {
        panic!("Expected DeleteMacro message");
    }
}

/// 测试获取下一个按键信息消息
#[test]
fn test_get_next_key_info_message() {
    let msg = Message::GetNextKeyInfo;
    let json = serde_json::to_string(&msg).expect("Failed to serialize");
    let deserialized: Message =
        serde_json::from_str(&json).expect("Failed to deserialize");

    assert!(matches!(deserialized, Message::GetNextKeyInfo));
}

/// 测试下一个按键信息响应
#[test]
fn test_next_key_info_message() {
    let msg = Message::NextKeyInfo {
        info: "Key: A, Scan: 0x1E".to_string(),
    };

    let json = serde_json::to_string(&msg).expect("Failed to serialize");
    let deserialized: Message =
        serde_json::from_str(&json).expect("Failed to deserialize");

    if let Message::NextKeyInfo { info } = deserialized {
        assert_eq!(info, "Key: A, Scan: 0x1E");
    } else {
        panic!("Expected NextKeyInfo message");
    }
}

/// 测试注册消息窗口
#[test]
fn test_register_message_window() {
    let msg = Message::RegisterMessageWindow { hwnd: 12345 };

    let json = serde_json::to_string(&msg).expect("Failed to serialize");
    let deserialized: Message =
        serde_json::from_str(&json).expect("Failed to deserialize");

    if let Message::RegisterMessageWindow { hwnd } = deserialized {
        assert_eq!(hwnd, 12345);
    } else {
        panic!("Expected RegisterMessageWindow message");
    }
}

/// 测试停止录制宏
#[test]
fn test_stop_macro_recording() {
    let msg = Message::StopMacroRecording;
    let json = serde_json::to_string(&msg).expect("Failed to serialize");
    let deserialized: Message =
        serde_json::from_str(&json).expect("Failed to deserialize");

    assert!(matches!(deserialized, Message::StopMacroRecording));
}

/// 测试获取宏列表
#[test]
fn test_get_macros() {
    let msg = Message::GetMacros;
    let json = serde_json::to_string(&msg).expect("Failed to serialize");
    let deserialized: Message =
        serde_json::from_str(&json).expect("Failed to deserialize");

    assert!(matches!(deserialized, Message::GetMacros));
}

/// 测试配置已加载消息
#[test]
fn test_config_loaded() {
    let msg = Message::ConfigLoaded;
    let json = serde_json::to_string(&msg).expect("Failed to serialize");
    let deserialized: Message =
        serde_json::from_str(&json).expect("Failed to deserialize");

    assert!(matches!(deserialized, Message::ConfigLoaded));
}

/// 测试实例端口计算
#[test]
fn test_get_instance_port() {
    assert_eq!(get_instance_port(0), 57427);
    assert_eq!(get_instance_port(1), 57428);
    assert_eq!(get_instance_port(10), 57437);
    assert_eq!(get_instance_port(100), 57527);
}

/// 测试实例地址生成
#[test]
fn test_get_instance_address() {
    assert_eq!(get_instance_address(0), "127.0.0.1:57427");
    assert_eq!(get_instance_address(1), "127.0.0.1:57428");
    assert_eq!(get_instance_address(5), "127.0.0.1:57432");
}

/// 测试 IPC 错误序列化
#[test]
fn test_ipc_error_serialization() {
    use wakem::ipc::IpcError;

    // 测试错误类型
    let err = IpcError::ConnectionRefused;
    assert_eq!(err.to_string(), "Connection refused");

    let err = IpcError::ConnectionClosed;
    assert_eq!(err.to_string(), "Connection closed");

    let err = IpcError::Timeout;
    assert_eq!(err.to_string(), "Timeout");
}
