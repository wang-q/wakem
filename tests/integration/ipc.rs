//! 集成测试 - 验证各组件协同工作

#[cfg(test)]
mod integration_tests {
    use wakem::ipc::Message;
    use wakem::types::{
        Action, KeyAction, Layer, LayerMode, LayerStack, Macro, MacroStep, MappingRule,
        ModifierState, Trigger,
    };

    // ==================== IPC 集成测试 ====================

    #[tokio::test]
    async fn test_ipc_message_serialization() {
        // 测试各种 IPC 消息的序列化和反序列化

        // ReloadConfig 消息
        let msg = Message::ReloadConfig;
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: Message = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, Message::ReloadConfig));

        // GetStatus 消息
        let msg = Message::GetStatus;
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: Message = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, Message::GetStatus));

        // SetActive 消息（替代 ToggleActive）
        let msg = Message::SetActive { active: true };
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: Message = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, Message::SetActive { .. }));

        // StartMacroRecording 消息
        let msg = Message::StartMacroRecording {
            name: "test".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: Message = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, Message::StartMacroRecording { .. }));

        // StopMacroRecording 消息
        let msg = Message::StopMacroRecording;
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: Message = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, Message::StopMacroRecording));

        // PlayMacro 消息
        let msg = Message::PlayMacro {
            name: "test".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: Message = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, Message::PlayMacro { .. }));
    }

    // ==================== 层管理集成测试 ====================

    #[test]
    fn test_layer_stack_operations() {
        let mut layer_stack = LayerStack::new();

        // 创建多层结构
        let base = Layer::new("base", 0x00, 0x00);
        let nav = Layer::new("navigation", 0x3A, 0x00).with_mode(LayerMode::Hold);
        let sym = Layer::new("symbols", 0x3B, 0x00).with_mode(LayerMode::Toggle);

        // 激活层
        layer_stack.activate_layer(base);
        layer_stack.activate_layer(nav.clone());
        layer_stack.activate_layer(sym);

        // 验证层结构
        let active_layers = layer_stack.get_active_layers();
        assert!(!active_layers.is_empty());

        // 测试层激活状态
        assert!(layer_stack.is_layer_active("base"));
        assert!(layer_stack.is_layer_active("navigation"));
        assert!(layer_stack.is_layer_active("symbols"));

        // 取消激活
        layer_stack.deactivate_layer("navigation");
        assert!(!layer_stack.is_layer_active("navigation"));
    }

    #[test]
    fn test_layer_hold_mechanism() {
        let mut layer_stack = LayerStack::new();

        // 创建并激活基础层
        let base = Layer::new("base", 0x00, 0x00);
        layer_stack.activate_layer(base);

        // 按住层（模拟按住激活键）
        layer_stack.hold_layer("base");

        // 验证层仍然激活
        assert!(layer_stack.is_layer_active("base"));
    }

    // ==================== 宏集成测试 ====================

    #[test]
    fn test_macro_creation_and_properties() {
        // 创建测试宏
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
                MacroStep::new(
                    100,
                    Action::Key(KeyAction::click(0x2E, 0x43)),
                    ModifierState::default(),
                    100,
                ),
            ],
            created_at: Some("2024-01-01".to_string()),
            description: Some("Test macro description".to_string()),
        };

        // 验证宏属性
        assert_eq!(macro_def.name, "test_macro");
        assert_eq!(macro_def.step_count(), 3);
        assert_eq!(macro_def.total_delay(), 150); // 0 + 50 + 100
    }

    #[test]
    fn test_empty_macro() {
        let empty_macro = Macro {
            name: "empty".to_string(),
            steps: vec![],
            created_at: None,
            description: None,
        };

        // 空宏应该正确报告属性
        assert_eq!(empty_macro.step_count(), 0);
        assert_eq!(empty_macro.total_delay(), 0);
    }

    // ==================== 映射规则集成测试 ====================

    #[test]
    fn test_mapping_rule_creation() {
        // 创建映射规则
        let rule = MappingRule::new(
            Trigger::Key {
                scan_code: Some(0x14),
                virtual_key: Some(0x14),
                modifiers: ModifierState::default(),
            },
            Action::Key(KeyAction::click(0x01, 0x1B)),
        )
        .with_name("caps_to_esc");

        assert_eq!(rule.name, Some("caps_to_esc".to_string()));
        assert!(rule.enabled);
    }

    #[test]
    fn test_mapping_rule_with_context() {
        use wakem::types::ContextCondition;

        let context = ContextCondition::new()
            .with_process_name("notepad.exe")
            .with_window_class("Notepad");

        let rule = MappingRule::new(
            Trigger::Key {
                scan_code: Some(0x41),
                virtual_key: Some(0x41),
                modifiers: ModifierState::default(),
            },
            Action::Key(KeyAction::click(0x42, 0x42)),
        )
        .with_context(context);

        assert!(rule.context.is_some());
    }

    // ==================== 复杂工作流测试 ====================

    #[test]
    fn test_multi_layer_workflow() {
        let mut layer_stack = LayerStack::new();

        // 创建多层结构
        let base = Layer::new("base", 0x00, 0x00);
        let nav = Layer::new("navigation", 0x3A, 0x00).with_mode(LayerMode::Hold);
        let sym = Layer::new("symbols", 0x3B, 0x00).with_mode(LayerMode::Toggle);
        let num = Layer::new("numbers", 0x3C, 0x00).with_mode(LayerMode::Hold);

        // 激活所有层
        layer_stack.activate_layer(base);
        layer_stack.activate_layer(nav.clone());
        layer_stack.activate_layer(sym.clone());
        layer_stack.activate_layer(num.clone());

        // 验证所有层都激活
        assert!(layer_stack.is_layer_active("base"));
        assert!(layer_stack.is_layer_active("navigation"));
        assert!(layer_stack.is_layer_active("symbols"));
        assert!(layer_stack.is_layer_active("numbers"));

        // 获取活动层列表
        let active = layer_stack.get_active_layers();
        assert_eq!(active.len(), 4);

        // 逐个取消激活
        layer_stack.deactivate_layer("numbers");
        assert!(!layer_stack.is_layer_active("numbers"));

        layer_stack.deactivate_layer("symbols");
        assert!(!layer_stack.is_layer_active("symbols"));

        // 基础层和导航层仍然激活
        assert!(layer_stack.is_layer_active("base"));
        assert!(layer_stack.is_layer_active("navigation"));
    }

    #[test]
    fn test_layer_priority() {
        let mut layer_stack = LayerStack::new();

        // 创建具有不同优先级的层
        let low_priority = Layer::new("low", 0x00, 0x00);
        let high_priority = Layer::new("high", 0x3A, 0x00);

        layer_stack.activate_layer(low_priority);
        layer_stack.activate_layer(high_priority);

        // 验证两个层都激活
        assert!(layer_stack.is_layer_active("low"));
        assert!(layer_stack.is_layer_active("high"));
    }

    // ==================== 边界情况测试 ====================

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

    #[test]
    fn test_large_number_of_layers() {
        let mut layer_stack = LayerStack::new();

        // 创建大量层
        for i in 0..50 {
            let layer = Layer::new(&format!("layer_{}", i), (i % 256) as u16, 0x00);
            layer_stack.activate_layer(layer);
        }

        // 验证所有层都激活
        for i in 0..50 {
            assert!(layer_stack.is_layer_active(&format!("layer_{}", i)));
        }

        let active = layer_stack.get_active_layers();
        assert_eq!(active.len(), 50);
    }

    #[test]
    fn test_complex_macro_with_delays() {
        // 创建包含多个延迟的复杂宏
        let steps: Vec<MacroStep> = (0..10)
            .map(|i| {
                MacroStep::new(
                    i as u64 * 100, // 递增延迟
                    Action::Key(KeyAction::click(i as u16, i as u16)),
                    ModifierState::default(),
                    i as u64 * 100,
                )
            })
            .collect();

        let macro_def = Macro {
            name: "complex_macro".to_string(),
            steps,
            created_at: None,
            description: None,
        };

        assert_eq!(macro_def.step_count(), 10);
        assert_eq!(macro_def.total_delay(), 4500); // 0+100+200+...+900
    }

    #[test]
    fn test_trigger_variants() {
        // 测试不同类型的触发器

        // 键盘触发器
        let key_trigger = Trigger::Key {
            scan_code: Some(0x1E),
            virtual_key: Some(0x41),
            modifiers: ModifierState::default(),
        };

        // 鼠标按钮触发器
        let mouse_trigger = Trigger::MouseButton {
            button: wakem::types::MouseButton::Left,
            modifiers: ModifierState::default(),
        };

        // 热字符串触发器
        let hotstring_trigger = Trigger::HotString {
            trigger: "test".to_string(),
        };

        // 创建对应的映射规则
        let _key_rule = MappingRule::new(key_trigger, Action::None);
        let _mouse_rule = MappingRule::new(mouse_trigger, Action::None);
        let _hotstring_rule = MappingRule::new(hotstring_trigger, Action::None);
    }

    #[test]
    fn test_action_variants() {
        // 测试不同类型的动作

        // 按键动作
        let key_action = Action::Key(KeyAction::click(0x1E, 0x41));

        // 鼠标动作
        let mouse_action = Action::mouse(wakem::types::MouseAction::Move {
            x: 100,
            y: 100,
            relative: false,
        });

        // 启动程序动作
        let launch_action = Action::launch("notepad.exe");

        // 窗口动作
        let window_action = Action::window(wakem::types::WindowAction::Maximize);

        // 验证所有动作都可以创建
        assert!(!matches!(key_action, Action::None));
        assert!(!matches!(mouse_action, Action::None));
        assert!(!matches!(launch_action, Action::None));
        assert!(!matches!(window_action, Action::None));
    }

    #[test]
    fn test_error_handling() {
        let mut layer_stack = LayerStack::new();

        // 检查不存在的层
        assert!(!layer_stack.is_layer_active("nonexistent"));

        // 尝试取消激活不存在的层（不应 panic）
        layer_stack.deactivate_layer("nonexistent");
        assert!(!layer_stack.is_layer_active("nonexistent"));
    }
}
