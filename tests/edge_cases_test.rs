// 边界条件和负面测试 - 极端值、错误处理和特殊情况

use wakem::config::{parse_key, wildcard_match, Config};
use wakem::types::{
    Action, InputEvent, KeyAction, KeyEvent, KeyState, Layer, LayerMode, Macro,
    MacroStep, MappingRule, ModifierState, MouseAction, MouseButton, MouseEvent,
    MouseEventType, Trigger, WindowAction,
};

// ==================== 空值和 None 处理 ====================

/// 测试空配置加载
#[test]
fn test_empty_config_loading() {
    let config: Config = toml::from_str("").unwrap();

    assert!(config.keyboard.remap.is_empty());
    assert!(config.keyboard.layers.is_empty());
    assert!(config.window.shortcuts.is_empty());
    assert!(config.launch.is_empty());
    assert!(config.macros.is_empty());
}

/// 测试空规则列表
#[test]
fn test_empty_rules_list() {
    use wakem::runtime::KeyMapper;

    let mut mapper = KeyMapper::new();
    mapper.load_rules(vec![]);

    // 空规则列表不应该 panic
    let event = InputEvent::Key(KeyEvent::new(0x1E, 0x41, KeyState::Pressed));
    let result = mapper.process_event_with_context(&event, None);
    assert!(result.is_none()); // 没有匹配的规则
}

/// 测试空宏
#[test]
fn test_empty_macro_operations() {
    let macro_def = Macro {
        name: "empty".to_string(),
        steps: vec![],
        created_at: None,
        description: None,
    };

    assert_eq!(macro_def.step_count(), 0);
    assert_eq!(macro_def.total_delay(), 0);
}

// ==================== 极端值测试 ====================

/// 测试非常长的键名（应该返回错误）
#[test]
fn test_very_long_key_name() {
    let long_name = "A".repeat(10000); // 10000 个字符

    let result = parse_key(&long_name);
    assert!(result.is_err(), "超长键名应该返回错误");
}

/// 测试非常长的宏名称
#[test]
fn test_very_long_macro_name() {
    let long_name = "macro_".repeat(1000); // "macro_" 重复 1000 次 = 6000 个字符

    // 宏名称可以很长，不应该出错
    let macro_def = Macro {
        name: long_name.clone(),
        steps: vec![],
        created_at: None,
        description: None,
    };

    assert_eq!(macro_def.name.len(), 6000); // "macro_" 是 6 个字符，1000 次重复
}

/// 测试大量层注册
#[test]
fn test_many_layers_registration() {
    use wakem::runtime::LayerManager;

    let mut manager = LayerManager::new();

    // 注册 100 个层
    for i in 0..100 {
        let name = format!("layer_{}", i);
        let layer = Layer::new(
            &name,
            0x01 + (i % 200) as u16, // 使用不同的扫描码
            0x01 + i as u16,
        )
        .with_mode(LayerMode::Hold);
        manager.register_layer(layer);
    }

    // 所有层初始状态都是未激活
    for i in 0..100 {
        let name = format!("layer_{}", i);
        assert!(!manager.is_layer_active(&name));
    }
}

/// 测试层内大量映射
#[test]
fn test_many_mappings_in_layer() {
    let mut layer = Layer::new("test", 0x3A, 0x14);

    // 添加 100 个映射（避免扫描码溢出）
    for i in 0..100u8 {
        let trigger_key = 0x04 + i as u16; // 从某个基础键开始
        let target_key = 0x04 + ((i + 50) % 100) as u16; // 映射到另一个键

        layer.add_mapping(
            Trigger::key(trigger_key, target_key),
            Action::key(KeyAction::click(target_key, target_key)),
        );
    }

    assert_eq!(layer.mappings.len(), 100);
}

/// 测试 Unicode 和特殊字符
#[test]
fn test_unicode_and_special_characters() {
    // 测试 Unicode 键名（应该失败）
    let unicode_names = ["日本語", "中文", "🎉", "émoji", "привет"];

    for name in &unicode_names {
        let result = parse_key(name);
        assert!(result.is_err(), "Unicode 键名 '{}' 应该返回错误", name);
    }

    // 测试特殊字符键名（应该失败）
    let special_chars = ["", " ", "\t", "\n", "@#$%", "!@#"];

    for name in &special_chars {
        let result = parse_key(name);
        if !name.is_empty() {
            assert!(result.is_err(), "特殊字符键名 '{:?}' 应该返回错误", name);
        }
    }
}

// ==================== 通配符边界情况 ====================

/// 测试通配符 - 空字符串
#[test]
fn test_wildcard_empty_string() {
    // 空模式匹配空字符串
    assert!(wildcard_match("", ""));

    // * 模式可以匹配空字符串（取决于实现）
    // 这里验证具体行为：* 匹配任意字符串，包括空字符串
    assert!(wildcard_match("", "*"));

    // 非空字符串不匹配空模式
    assert!(!wildcard_match("test", ""));
}

/// 测试通配符 - 多个连续 *
#[test]
fn test_wildcard_multiple_stars() {
    assert!(wildcard_match("test", "**"));
    assert!(wildcard_match("test", "***"));
    assert!(wildcard_match("", "**"));
    assert!(wildcard_match("a*b*c", "a*b*c"));
}

/// 测试通配符 - 嵌套模式
#[test]
fn test_wildcard_complex_patterns() {
    // 以特定前缀开头
    assert!(wildcard_match("test_file.txt", "test*"));
    assert!(!wildcard_match("my_test.txt", "test*"));

    // 以特定后缀结尾
    assert!(wildcard_match("file.txt", "*.txt"));
    assert!(!wildcard_match("file.doc", "*.txt"));

    // 包含子串
    assert!(wildcard_match("a_test_b", "*test*"));
    assert!(!wildcard_match("a_xyz_b", "*test*"));
}

// ==================== ModifierState 边界 ====================

/// 测试所有修饰键组合
#[test]
fn test_all_modifier_combinations() {
    let mut modifiers = ModifierState::default();
    assert!(modifiers.is_empty());

    // 单个修饰键
    modifiers.shift = true;
    assert!(!modifiers.is_empty());
    assert!(modifiers.shift);
    assert!(!modifiers.ctrl);
    assert!(!modifiers.alt);
    assert!(!modifiers.meta);

    // 多个修饰键组合
    modifiers.ctrl = true;
    modifiers.alt = true;
    modifiers.meta = true;
    assert!(modifiers.shift && modifiers.ctrl && modifiers.alt && modifiers.meta);

    // 清除所有
    modifiers.shift = false;
    modifiers.ctrl = false;
    modifiers.alt = false;
    modifiers.meta = false;
    assert!(modifiers.is_empty());
}

// ==================== Action 边界情况 ====================

/// 测试 Action::None 的行为
#[test]
fn test_action_none_behavior() {
    let action = Action::None;
    assert!(action.is_none());

    // None 动作不应该引起任何问题
    let _ = format!("{:?}", action);
}

/// 测试嵌套动作序列
#[test]
fn test_nested_action_sequences() {
    // 创建一个包含序列的动作序列
    let inner_seq = Action::sequence(vec![
        Action::key(KeyAction::click(0x01, 0x1B)),
        Action::key(KeyAction::click(0x0E, 0x08)),
    ]);

    let outer_seq = Action::sequence(vec![
        inner_seq,
        Action::delay(100),
        Action::key(KeyAction::click(0x2C, 0x5A)), // Z key
    ]);

    if let Action::Sequence(actions) = outer_seq {
        assert_eq!(actions.len(), 3);
        // 第一个元素是内部序列
        if let Action::Sequence(inner) = &actions[0] {
            assert_eq!(inner.len(), 2);
        } else {
            panic!("Expected nested sequence");
        }
    }
}

/// 测试极端延迟值
#[test]
fn test_extreme_delay_values() {
    // 零延迟
    let zero_delay = Action::delay(0);
    if let Action::Delay { milliseconds } = zero_delay {
        assert_eq!(milliseconds, 0);
    }

    // 最大合理延迟（1小时）
    let max_delay = Action::delay(3600000);
    if let Action::Delay { milliseconds } = max_delay {
        assert_eq!(milliseconds, 3600000);
    }
}

// ==================== InputEvent 边界 ====================

/// 测试各种键盘事件状态
#[test]
fn test_all_keyboard_event_states() {
    let states = [KeyState::Pressed, KeyState::Released];

    for state in &states {
        let event = InputEvent::Key(KeyEvent::new(0x1E, 0x41, *state));
        assert!(!event.is_injected());

        let action = Action::from_input_event(&event);
        assert!(action.is_some());
    }
}

/// 测试鼠标事件类型
#[test]
fn test_all_mouse_event_types() {
    let mouse_events = vec![
        MouseEvent::new(MouseEventType::Move, 100, 200),
        MouseEvent::new(MouseEventType::ButtonDown(MouseButton::Left), 0, 0),
        MouseEvent::new(MouseEventType::ButtonUp(MouseButton::Left), 0, 0),
        MouseEvent::new(MouseEventType::ButtonDown(MouseButton::Right), 0, 0),
        MouseEvent::new(MouseEventType::ButtonUp(MouseButton::Right), 0, 0),
        MouseEvent::new(MouseEventType::ButtonDown(MouseButton::Middle), 0, 0),
        MouseEvent::new(MouseEventType::ButtonUp(MouseButton::Middle), 0, 0),
        MouseEvent::new(MouseEventType::Wheel(120), 0, 0),
        MouseEvent::new(MouseEventType::Wheel(-120), 0, 0),
        MouseEvent::new(MouseEventType::HWheel(10), 0, 0),
        MouseEvent::new(MouseEventType::HWheel(-10), 0, 0),
    ];

    for mouse_event in mouse_events {
        let event = InputEvent::Mouse(mouse_event);
        assert!(!event.is_injected());

        let action = Action::from_input_event(&event);
        assert!(action.is_some());
    }
}

// ==================== 配置解析边界 ====================

/// 测试配置中的极端数值
#[test]
fn test_config_extreme_values() {
    // 大数值
    let config_str = r#"
[mouse.wheel]
speed = 1000000
acceleration_multiplier = 99999.99
"#;

    let config: Config = toml::from_str(config_str).unwrap();
    assert_eq!(config.mouse.wheel.speed, 1000000);
    assert!((config.mouse.wheel.acceleration_multiplier - 99999.99).abs() < 0.001);

    // 小数值
    let config_str = r#"
[mouse.wheel]
speed = 1
acceleration_multiplier = 0.001
"#;

    let config: Config = toml::from_str(config_str).unwrap();
    assert_eq!(config.mouse.wheel.speed, 1);
    assert!((config.mouse.wheel.acceleration_multiplier - 0.001).abs() < 0.0001);
}

/// 测试网络配置边界值
#[test]
fn test_network_config_boundary() {
    // instance_id = 0（默认）
    let config_str = r#"
[network]
enabled = true
instance_id = 0
"#;

    let config: Config = toml::from_str(config_str).unwrap();
    assert!(config.network.enabled);
    assert_eq!(config.network.instance_id, 0);

    // 大 instance_id
    let config_str = r#"
[network]
enabled = true
instance_id = 65535
"#;

    let config: Config = toml::from_str(config_str).unwrap();
    assert_eq!(config.network.instance_id, 65535);
}

// ==================== MappingRule 边界 ====================

/// 测试规则的启用/禁用切换
#[test]
fn test_rule_enable_disable_toggle() {
    let rule = MappingRule::new(
        Trigger::key(0x3A, 0x14),
        Action::key(KeyAction::click(0x0E, 0x08)),
    );

    assert!(rule.enabled);

    // 禁用
    let mut disabled_rule = rule.clone();
    disabled_rule.enabled = false;
    assert!(!disabled_rule.enabled);

    // 重新启用
    disabled_rule.enabled = true;
    assert!(disabled_rule.enabled);
}

/// 测试带上下文的规则
#[test]
fn test_rule_with_context_condition() {
    use wakem::types::ContextCondition;

    let context = ContextCondition::new()
        .with_process_name("chrome.exe")
        .with_window_class("Chrome_WidgetWin_1")
        .with_window_title("*Google*");

    let rule = MappingRule::new(
        Trigger::key(0x41, 0x41),
        Action::key(KeyAction::click(0x42, 0x42)),
    )
    .with_context(context);

    assert!(rule.context.is_some());

    let ctx = rule.context.unwrap();
    assert_eq!(ctx.process_name.as_deref().unwrap(), "chrome.exe");
    assert_eq!(ctx.window_class.as_deref().unwrap(), "Chrome_WidgetWin_1");
    assert_eq!(ctx.window_title.as_deref().unwrap(), "*Google*");
}

// ==================== Layer 边界 ====================

/// 测试层的 Hold/Toggle 模式行为差异
#[test]
fn test_layer_mode_differences() {
    let hold_layer = Layer::new("hold", 0x3A, 0x14).with_mode(LayerMode::Hold);
    let toggle_layer = Layer::new("toggle", 0x39, 0x20).with_mode(LayerMode::Toggle);

    assert!(matches!(hold_layer.mode, LayerMode::Hold));
    assert!(matches!(toggle_layer.mode, LayerMode::Toggle));

    // 两种模式都可以添加映射
    let mut hold_clone = hold_layer.clone();
    hold_clone.add_mapping(
        Trigger::key(0x23, 0x48),
        Action::key(KeyAction::click(0x4B, 0x25)),
    );
    assert_eq!(hold_clone.mappings.len(), 1);

    let mut toggle_clone = toggle_layer.clone();
    toggle_clone.add_mapping(
        Trigger::key(0x23, 0x48),
        Action::key(KeyAction::click(0x4B, 0x25)),
    );
    assert_eq!(toggle_clone.mappings.len(), 1);
}

// ==================== 序列化和反序列化边界 ====================

/// 测试 IPC Message 序列化边界
#[test]
fn test_message_serialization_edge_cases() {
    use wakem::ipc::Message;

    // 空字符串消息
    let msg = Message::Error {
        message: String::new(),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let parsed: Message = serde_json::from_str(&json).unwrap();
    if let Message::Error { message } = parsed {
        assert!(message.is_empty());
    }

    // Unicode 内容
    let msg = Message::Error {
        message: "错误信息 🎉 日本語".to_string(),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let parsed: Message = serde_json::from_str(&json).unwrap();
    if let Message::Error { message } = parsed {
        assert_eq!(message, "错误信息 🎉 日本語");
    }

    // 超长消息
    let long_msg = "x".repeat(10000);
    let msg = Message::Error {
        message: long_msg.clone(),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let parsed: Message = serde_json::from_str(&json).unwrap();
    if let Message::Error { message } = parsed {
        assert_eq!(message.len(), 10000);
    }
}

// ==================== 性能相关边界测试 ====================

/// 测试大量规则的处理性能（不测量时间，只验证正确性）
#[test]
fn test_many_rules_processing() {
    use wakem::runtime::KeyMapper;

    let mut mapper = KeyMapper::new();

    // 创建 1000 条规则
    let rules: Vec<MappingRule> = (0..1000)
        .map(|i| {
            let scan_code = (i % 256) as u16;
            MappingRule::new(
                Trigger::key(scan_code, scan_code + 0x40),
                Action::key(KeyAction::click(scan_code + 1, scan_code + 0x41)),
            )
        })
        .collect();

    mapper.load_rules(rules);

    // 测试其中一条规则能正常工作
    let event = InputEvent::Key(KeyEvent::new(0, 0x40, KeyState::Pressed));
    let result = mapper.process_event_with_context(&event, None);
    assert!(result.is_some());
}
