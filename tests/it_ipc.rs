// 集成测试 - IPC 通信测试

use wakem::ipc::Message;
use wakem::types::{
    Action, KeyAction, Layer, LayerMode, LayerStack, Macro, MacroStep, ModifierState,
};

/// 测试 IPC 消息序列化
#[tokio::test]
async fn test_ipc_message_serialization() {
    let msg = Message::ReloadConfig;
    let json = serde_json::to_string(&msg).unwrap();
    let deserialized: Message = serde_json::from_str(&json).unwrap();
    assert!(matches!(deserialized, Message::ReloadConfig));

    let msg = Message::GetStatus;
    let json = serde_json::to_string(&msg).unwrap();
    let deserialized: Message = serde_json::from_str(&json).unwrap();
    assert!(matches!(deserialized, Message::GetStatus));

    let msg = Message::SetActive { active: true };
    let json = serde_json::to_string(&msg).unwrap();
    let deserialized: Message = serde_json::from_str(&json).unwrap();
    assert!(matches!(deserialized, Message::SetActive { .. }));

    let msg = Message::StartMacroRecording {
        name: "test".to_string(),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let deserialized: Message = serde_json::from_str(&json).unwrap();
    assert!(matches!(deserialized, Message::StartMacroRecording { .. }));

    let msg = Message::StopMacroRecording;
    let json = serde_json::to_string(&msg).unwrap();
    let deserialized: Message = serde_json::from_str(&json).unwrap();
    assert!(matches!(deserialized, Message::StopMacroRecording));

    let msg = Message::PlayMacro {
        name: "test".to_string(),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let deserialized: Message = serde_json::from_str(&json).unwrap();
    assert!(matches!(deserialized, Message::PlayMacro { .. }));
}

/// 测试层堆栈操作
#[test]
fn test_layer_stack_operations() {
    let mut layer_stack = LayerStack::new();

    let base = Layer::new("base", 0x00, 0x00);
    let nav = Layer::new("navigation", 0x3A, 0x00).with_mode(LayerMode::Hold);
    let sym = Layer::new("symbols", 0x3B, 0x00).with_mode(LayerMode::Toggle);

    layer_stack.activate_layer(base);
    layer_stack.activate_layer(nav.clone());
    layer_stack.activate_layer(sym);

    assert!(layer_stack.is_layer_active("base"));
    assert!(layer_stack.is_layer_active("navigation"));
    assert!(layer_stack.is_layer_active("symbols"));

    layer_stack.deactivate_layer("navigation");
    assert!(!layer_stack.is_layer_active("navigation"));
}

/// 测试宏创建和属性
#[test]
fn test_macro_creation_and_properties() {
    let macro_def = Macro {
        name: "test_macro".to_string(),
        steps: vec![
            MacroStep::new(
                0,
                Action::Key(KeyAction::click(0x1E, 0x41)),
                ModifierState::default(),
                0,
            ),
            MacroStep::new(
                50,
                Action::Key(KeyAction::click(0x30, 0x42)),
                ModifierState::default(),
                50,
            ),
        ],
        created_at: Some("2024-01-01".to_string()),
        description: Some("Test macro".to_string()),
    };

    assert_eq!(macro_def.name, "test_macro");
    assert_eq!(macro_def.step_count(), 2);
    assert_eq!(macro_def.total_delay(), 50);
}

/// 测试多层工作流
#[test]
fn test_multi_layer_workflow() {
    let mut layer_stack = LayerStack::new();

    let base = Layer::new("base", 0x00, 0x00);
    let nav = Layer::new("navigation", 0x3A, 0x00).with_mode(LayerMode::Hold);
    let sym = Layer::new("symbols", 0x3B, 0x00).with_mode(LayerMode::Toggle);
    let num = Layer::new("numbers", 0x3C, 0x00).with_mode(LayerMode::Hold);

    layer_stack.activate_layer(base);
    layer_stack.activate_layer(nav.clone());
    layer_stack.activate_layer(sym.clone());
    layer_stack.activate_layer(num.clone());

    assert!(layer_stack.is_layer_active("base"));
    assert!(layer_stack.is_layer_active("navigation"));
    assert!(layer_stack.is_layer_active("symbols"));
    assert!(layer_stack.is_layer_active("numbers"));

    let active = layer_stack.get_active_layers();
    assert_eq!(active.len(), 4);
}

/// 测试 Unicode 名称
#[test]
fn test_unicode_in_names() {
    let layer = Layer::new("测试层 🎉", 0x3A, 0x00);
    assert_eq!(layer.name, "测试层 🎉");

    let macro_def = Macro {
        name: "日本語マクロ".to_string(),
        steps: vec![],
        created_at: None,
        description: Some("中文描述".to_string()),
    };
    assert_eq!(macro_def.name, "日本語マクロ");
}

/// 测试错误处理
#[test]
fn test_error_handling() {
    let mut layer_stack = LayerStack::new();

    assert!(!layer_stack.is_layer_active("nonexistent"));

    layer_stack.deactivate_layer("nonexistent");
    assert!(!layer_stack.is_layer_active("nonexistent"));
}
