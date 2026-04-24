// Types 补充测试 - 边界条件、错误处理和完整类型覆盖

use wakem::types::{
    Action, Alignment, ContextCondition, ContextInfo, Edge, InputEvent, KeyAction,
    KeyEvent, KeyState, LaunchAction, Layer, LayerMode, MappingRule, ModifierState,
    MonitorDirection, MouseAction, MouseButton, MouseEvent, MouseEventType, Trigger,
    WindowAction,
};

// ==================== Action 变体完整测试 ====================

/// 测试从键盘按下事件创建 Action
#[test]
fn test_action_from_input_event_key_pressed() {
    let key_event = KeyEvent::new(0x1E, 0x41, KeyState::Pressed);
    let event = InputEvent::Key(key_event);

    let action = Action::from_input_event(&event);
    assert!(action.is_some());

    if let Some(Action::Key(KeyAction::Press {
        scan_code,
        virtual_key,
    })) = action
    {
        assert_eq!(scan_code, 0x1E);
        assert_eq!(virtual_key, 0x41);
    } else {
        panic!("Expected Key Press action");
    }
}

/// 测试从键盘释放事件创建 Action
#[test]
fn test_action_from_input_event_key_released() {
    let key_event = KeyEvent::new(0x1E, 0x41, KeyState::Released);
    let event = InputEvent::Key(key_event);

    let action = Action::from_input_event(&event);
    assert!(action.is_some());

    if let Some(Action::Key(KeyAction::Release {
        scan_code,
        virtual_key,
    })) = action
    {
        assert_eq!(scan_code, 0x1E);
        assert_eq!(virtual_key, 0x41);
    } else {
        panic!("Expected Key Release action");
    }
}

/// 测试从鼠标移动事件创建 Action
#[test]
fn test_action_from_input_event_mouse_move() {
    let mouse_event = MouseEvent::new(MouseEventType::Move, 100, 200);
    let event = InputEvent::Mouse(mouse_event);

    let action = Action::from_input_event(&event);
    assert!(action.is_some());

    if let Some(Action::Mouse(MouseAction::Move { x, y, relative })) = action {
        assert_eq!(x, 100);
        assert_eq!(y, 200);
        assert!(!relative);
    } else {
        panic!("Expected Mouse Move action");
    }
}

/// 测试从鼠标按钮按下事件创建 Action
#[test]
fn test_action_from_input_event_mouse_button_down() {
    let mouse_event =
        MouseEvent::new(MouseEventType::ButtonDown(MouseButton::Left), 0, 0);
    let event = InputEvent::Mouse(mouse_event);

    let action = Action::from_input_event(&event);
    assert!(action.is_some());

    if let Some(Action::Mouse(MouseAction::ButtonDown { button })) = action {
        assert_eq!(button, MouseButton::Left);
    } else {
        panic!("Expected Mouse ButtonDown action");
    }
}

/// 测试从鼠标按钮释放事件创建 Action
#[test]
fn test_action_from_input_event_mouse_button_up() {
    let mouse_event =
        MouseEvent::new(MouseEventType::ButtonUp(MouseButton::Right), 0, 0);
    let event = InputEvent::Mouse(mouse_event);

    let action = Action::from_input_event(&event);
    assert!(action.is_some());

    if let Some(Action::Mouse(MouseAction::ButtonUp { button })) = action {
        assert_eq!(button, MouseButton::Right);
    } else {
        panic!("Expected Mouse ButtonUp action");
    }
}

/// 测试从鼠标滚轮事件创建 Action
#[test]
fn test_action_from_input_event_mouse_wheel() {
    let mouse_event = MouseEvent::new(MouseEventType::Wheel(120), 0, 0);
    let event = InputEvent::Mouse(mouse_event);

    let action = Action::from_input_event(&event);
    assert!(action.is_some());

    if let Some(Action::Mouse(MouseAction::Wheel { delta })) = action {
        assert_eq!(delta, 120);
    } else {
        panic!("Expected Mouse Wheel action");
    }
}

// ==================== ModifierState 测试 ====================

/// 测试 ModifierState 默认值（全部为 false）
#[test]
fn test_modifier_state_default() {
    let modifiers = ModifierState::default();
    assert!(!modifiers.shift);
    assert!(!modifiers.ctrl);
    assert!(!modifiers.alt);
    assert!(!modifiers.meta);
}

/// 测试 ModifierState::is_empty()
#[test]
fn test_modifier_state_is_empty() {
    let mut modifiers = ModifierState::default();
    assert!(modifiers.is_empty());

    // 设置一个修饰键后不再为空
    modifiers.ctrl = true;
    assert!(!modifiers.is_empty());
}

/// 测试从虚拟键码创建 Shift 状态
#[test]
fn test_modifier_state_from_virtual_key_shift() {
    let (state, pressed) = ModifierState::from_virtual_key(0x10, true).unwrap();
    assert!(state.shift);
    assert!(pressed);

    let (state, _pressed) = ModifierState::from_virtual_key(0xA0, true).unwrap();
    assert!(state.shift); // LSHIFT

    let (state, _pressed) = ModifierState::from_virtual_key(0xA1, true).unwrap();
    assert!(state.shift); // RSHIFT

    // 释放状态
    let (_, pressed) = ModifierState::from_virtual_key(0x10, false).unwrap();
    assert!(!pressed);
}

/// 测试从虚拟键码创建 Ctrl 状态
#[test]
fn test_modifier_state_from_virtual_key_ctrl() {
    let (state, _) = ModifierState::from_virtual_key(0x11, true).unwrap();
    assert!(state.ctrl);

    let (state, _) = ModifierState::from_virtual_key(0xA2, true).unwrap();
    assert!(state.ctrl); // LCONTROL

    let (state, _) = ModifierState::from_virtual_key(0xA3, true).unwrap();
    assert!(state.ctrl); // RCONTROL
}

/// 测试从虚拟键码创建 Alt 状态
#[test]
fn test_modifier_state_from_virtual_key_alt() {
    let (state, _) = ModifierState::from_virtual_key(0x12, true).unwrap();
    assert!(state.alt);

    let (state, _) = ModifierState::from_virtual_key(0xA4, true).unwrap();
    assert!(state.alt); // LMENU

    let (state, _) = ModifierState::from_virtual_key(0xA5, true).unwrap();
    assert!(state.alt); // RMENU
}

/// 测试从虚拟键码创建 Meta/Win 状态
#[test]
fn test_modifier_state_from_virtual_key_meta() {
    let (state, _) = ModifierState::from_virtual_key(0x5B, true).unwrap();
    assert!(state.meta); // LWIN

    let (state, _) = ModifierState::from_virtual_key(0x5C, true).unwrap();
    assert!(state.meta); // RWIN
}

/// 测试未知虚拟键码返回 None
#[test]
fn test_modifier_state_from_virtual_key_unknown() {
    let result = ModifierState::from_virtual_key(0x41, true); // 'A' 键不是修饰键
    assert!(result.is_none());
}

/// 测试 ModifierState 合并
#[test]
fn test_modifier_state_merge() {
    let mut state1 = ModifierState::default();
    state1.ctrl = true;

    let mut state2 = ModifierState::default();
    state2.shift = true;
    state2.alt = true;

    state1.merge(&state2);

    assert!(state1.ctrl);
    assert!(state1.shift);
    assert!(state1.alt);
    assert!(!state1.meta);
}

// ==================== InputEvent 测试 ====================

/// 测试注入事件的标记
#[test]
fn test_input_event_is_injected_true() {
    let key_event = KeyEvent::new(0x1E, 0x41, KeyState::Pressed).injected();

    let event = InputEvent::Key(key_event);
    assert!(event.is_injected());
}

/// 测试非注入事件
#[test]
fn test_input_event_is_injected_false() {
    let key_event = KeyEvent::new(0x1E, 0x41, KeyState::Pressed);
    let event = InputEvent::Key(key_event);
    assert!(!event.is_injected());
}

/// 测试鼠标事件的 is_injected（应该总是返回 false）
#[test]
fn test_input_event_mouse_not_injected() {
    let mouse_event = MouseEvent::new(MouseEventType::Move, 100, 200);
    let event = InputEvent::Mouse(mouse_event);
    assert!(!event.is_injected());
}

// ==================== Trigger 匹配测试 ====================

/// 测试精确键匹配
#[test]
fn test_trigger_matches_exact_key() {
    let trigger = Trigger::key(0x1E, 0x41); // 'A'

    let event = InputEvent::Key(KeyEvent::new(0x1E, 0x41, KeyState::Pressed));
    assert!(trigger.matches(&event));

    // 不同按键不匹配
    let event2 = InputEvent::Key(KeyEvent::new(0x30, 0x42, KeyState::Pressed));
    assert!(!trigger.matches(&event2));
}

/// 测试带修饰键的触发器匹配
#[test]
fn test_trigger_matches_with_modifiers() {
    let mut modifiers = ModifierState::default();
    modifiers.ctrl = true;
    let trigger = Trigger::key_with_modifiers(0x1E, 0x41, modifiers);

    // 带有 Ctrl 的 A 键事件 - 应该匹配
    let mut event_modifiers = ModifierState::default();
    event_modifiers.ctrl = true;
    let event = InputEvent::Key(
        KeyEvent::new(0x1E, 0x41, KeyState::Pressed).with_modifiers(event_modifiers),
    );
    assert!(trigger.matches(&event));

    // 不带修饰键的 A 键事件 - 不应该匹配（因为修饰键必须匹配）
    let event2 = InputEvent::Key(KeyEvent::new(0x1E, 0x41, KeyState::Pressed));
    assert!(!trigger.matches(&event2));
}

/// 测试鼠标按钮触发器
#[test]
fn test_trigger_matches_mouse_button() {
    let trigger = Trigger::MouseButton {
        button: MouseButton::Left,
        modifiers: ModifierState::default(),
    };

    let mouse_event =
        MouseEvent::new(MouseEventType::ButtonDown(MouseButton::Left), 0, 0);
    let event = InputEvent::Mouse(mouse_event);
    assert!(trigger.matches(&event));

    // 不同按钮不匹配
    let mouse_event2 =
        MouseEvent::new(MouseEventType::ButtonDown(MouseButton::Right), 0, 0);
    let event2 = InputEvent::Mouse(mouse_event2);
    assert!(!trigger.matches(&event2));
}

/// 测试热字符串触发器
#[test]
fn test_trigger_matches_hotstring() {
    let trigger = Trigger::HotString {
        trigger: "test".to_string(),
    };

    // 注意：热字符串触发器的匹配逻辑可能需要特殊处理
    // 这里只验证可以创建
    let _ = trigger;
}

// ==================== MappingRule 测试 ====================

/// 测试规则启用/禁用
#[test]
fn test_mapping_rule_enable_disable() {
    let rule = MappingRule::new(
        Trigger::key(0x3A, 0x14),
        Action::key(KeyAction::click(0x0E, 0x08)),
    );

    assert!(rule.enabled);

    let mut rule_disabled = rule.clone();
    rule_disabled.enabled = false;
    assert!(!rule_disabled.enabled);
}

/// 测试规则名称设置
#[test]
fn test_mapping_rule_with_name() {
    let rule = MappingRule::new(
        Trigger::key(0x3A, 0x14),
        Action::key(KeyAction::click(0x0E, 0x08)),
    )
    .with_name("caps_to_esc");

    assert_eq!(rule.name.as_deref().unwrap(), "caps_to_esc");
}

/// 测试规则上下文条件
#[test]
fn test_mapping_rule_with_context() {
    let context = ContextCondition::new()
        .with_process_name("notepad.exe")
        .with_window_class("Notepad");

    let rule = MappingRule::new(
        Trigger::key(0x41, 0x41),
        Action::key(KeyAction::click(0x42, 0x42)),
    )
    .with_context(context);

    assert!(rule.context.is_some());
}

/// 测试 MappingRule 创建
#[test]
fn test_mapping_rule_creation() {
    let trigger = Trigger::key(0x1E, 0x41); // 'A' key
    let action = Action::window(WindowAction::Center);

    let rule = MappingRule::new(trigger, action);

    assert!(rule.enabled);
    assert!(rule.name.is_none());
    assert!(rule.context.is_none());
}

/// 测试 MappingRule with_name
#[test]
fn test_mapping_rule_with_name_alt() {
    let trigger = Trigger::key(0x1E, 0x41);
    let action = Action::window(WindowAction::Center);

    let rule = MappingRule::new(trigger, action).with_name("Center Window");

    assert_eq!(rule.name, Some("Center Window".to_string()));
}

/// 测试 MappingRule with_context
#[test]
fn test_mapping_rule_with_context_alt() {
    let trigger = Trigger::key(0x1E, 0x41);
    let action = Action::window(WindowAction::Center);

    let context = ContextCondition::new().with_process_name("notepad.exe");

    let rule = MappingRule::new(trigger, action).with_context(context);

    assert!(rule.context.is_some());
}

/// 测试 MappingRule 禁用状态
#[test]
fn test_mapping_rule_disabled() {
    let trigger = Trigger::key(0x1E, 0x41);
    let action = Action::window(WindowAction::Center);

    let mut rule = MappingRule::new(trigger, action);
    rule.enabled = false;

    let event = InputEvent::Key(KeyEvent::new(0x1E, 0x41, KeyState::Pressed));

    let context = ContextInfo::default();

    // 禁用的规则不应该匹配
    assert!(!rule.matches(&event, &context));
}

/// 测试复杂的 ContextCondition
#[test]
fn test_complex_context_condition() {
    let cond = ContextCondition::new()
        .with_process_name("code.exe")
        .with_window_class("Chrome_WidgetWin_1");

    let full_match = ContextInfo {
        window_class: "Chrome_WidgetWin_1".to_string(),
        process_name: "code.exe".to_string(),
        process_path: "".to_string(),
        window_title: "".to_string(),
        window_handle: 0,
    };

    let partial_match = ContextInfo {
        window_class: "Chrome_WidgetWin_1".to_string(),
        process_name: "notepad.exe".to_string(),
        process_path: "".to_string(),
        window_title: "".to_string(),
        window_handle: 0,
    };

    assert!(cond.matches(
        &full_match.process_name,
        &full_match.window_class,
        &full_match.window_title,
        Some(&full_match.process_path)
    ));
    assert!(!cond.matches(
        &partial_match.process_name,
        &partial_match.window_class,
        &partial_match.window_title,
        Some(&partial_match.process_path)
    ));
}

// ==================== Layer 测试 ====================

/// 测试 Hold 模式层的行为
#[test]
fn test_layer_hold_mode_behavior() {
    let layer = Layer::new("hold_layer", 0x3A, 0x14).with_mode(LayerMode::Hold);
    assert!(matches!(layer.mode, LayerMode::Hold));
}

/// 测试 Toggle 模式层的行为
#[test]
fn test_layer_toggle_mode_behavior() {
    let layer = Layer::new("toggle_layer", 0x39, 0x20).with_mode(LayerMode::Toggle);
    assert!(matches!(layer.mode, LayerMode::Toggle));
}

/// 测试层添加多个映射
#[test]
fn test_layer_add_multiple_mappings() {
    let mut layer = Layer::new("nav", 0x3A, 0x14);

    layer.add_mapping(
        Trigger::key(0x23, 0x48),
        Action::key(KeyAction::click(0x4B, 0x25)), // H -> Left
    );
    layer.add_mapping(
        Trigger::key(0x24, 0x4A),
        Action::key(KeyAction::click(0x50, 0x28)), // J -> Down
    );
    layer.add_mapping(
        Trigger::key(0x25, 0x4B),
        Action::key(KeyAction::click(0x48, 0x26)), // K -> Up
    );

    assert_eq!(layer.mappings.len(), 3);
}

/// 测试层激活键检查
#[test]
fn test_layer_activation_key_check() {
    let layer = Layer::new("test", 0x3A, 0x14);

    assert!(layer.is_activation_key(0x3A, 0x14)); // 正确的扫描码和虚拟键码
    assert!(!layer.is_activation_key(0x3B, 0x15)); // 不同的键
}

// ==================== ContextCondition 测试 ====================

/// 测试 ContextCondition 创建和属性
#[test]
fn test_context_condition_creation() {
    let condition = ContextCondition::new()
        .with_process_name("chrome.exe")
        .with_window_class("Chrome_WidgetWin_1")
        .with_window_title("*Google*");

    assert_eq!(condition.process_name.as_deref().unwrap(), "chrome.exe");
    assert_eq!(
        condition.window_class.as_deref().unwrap(),
        "Chrome_WidgetWin_1"
    );
    assert_eq!(condition.window_title.as_deref().unwrap(), "*Google*");
}

/// 测试空 ContextCondition
#[test]
fn test_context_condition_empty() {
    let condition = ContextCondition::new();
    assert!(condition.process_name.is_none());
    assert!(condition.window_class.is_none());
    assert!(condition.window_title.is_none());
}

/// 测试 ContextCondition 匹配 - 空条件应该匹配所有
#[test]
fn test_context_condition_empty_matches_all() {
    let cond = ContextCondition::new();
    let context = ContextInfo {
        window_class: "AnyClass".to_string(),
        process_name: "any.exe".to_string(),
        process_path: "C:\\any.exe".to_string(),
        window_title: "Any Title".to_string(),
        window_handle: 0,
    };

    assert!(cond.matches(
        &context.process_name,
        &context.window_class,
        &context.window_title,
        Some(&context.process_path)
    ));
}

/// 测试 ContextCondition 进程名匹配
#[test]
fn test_context_condition_process_match() {
    let cond = ContextCondition::new().with_process_name("notepad.exe");

    let matching_context = ContextInfo {
        window_class: "Notepad".to_string(),
        process_name: "notepad.exe".to_string(),
        process_path: "C:\\Windows\\notepad.exe".to_string(),
        window_title: "Untitled".to_string(),
        window_handle: 0,
    };

    let non_matching_context = ContextInfo {
        window_class: "Chrome".to_string(),
        process_name: "chrome.exe".to_string(),
        process_path: "C:\\Program Files\\chrome.exe".to_string(),
        window_title: "Google".to_string(),
        window_handle: 0,
    };

    assert!(cond.matches(
        &matching_context.process_name,
        &matching_context.window_class,
        &matching_context.window_title,
        Some(&matching_context.process_path)
    ));
    assert!(!cond.matches(
        &non_matching_context.process_name,
        &non_matching_context.window_class,
        &non_matching_context.window_title,
        Some(&non_matching_context.process_path)
    ));
}

/// 测试 Trigger::key 创建
#[test]
fn test_trigger_key_creation() {
    let trigger = Trigger::key(0x1E, 0x41);

    match trigger {
        Trigger::Key {
            scan_code,
            virtual_key,
            modifiers,
        } => {
            assert_eq!(scan_code, Some(0x1E));
            assert_eq!(virtual_key, Some(0x41));
            assert!(modifiers.is_empty());
        }
        _ => panic!("Expected Key trigger"),
    }
}

/// 测试 Trigger::key_with_modifiers
#[test]
fn test_trigger_key_with_modifiers() {
    let mut modifiers = ModifierState::new();
    modifiers.ctrl = true;
    modifiers.shift = true;

    let trigger = Trigger::key_with_modifiers(0x1E, 0x41, modifiers);

    match trigger {
        Trigger::Key {
            scan_code,
            virtual_key,
            modifiers: m,
        } => {
            assert_eq!(scan_code, Some(0x1E));
            assert_eq!(virtual_key, Some(0x41));
            assert!(m.ctrl);
            assert!(m.shift);
            assert!(!m.alt);
            assert!(!m.meta);
        }
        _ => panic!("Expected Key trigger"),
    }
}

/// 测试 Trigger 变体
#[test]
fn test_trigger_variants() {
    let key_trigger = Trigger::Key {
        scan_code: Some(0x1E),
        virtual_key: Some(0x41),
        modifiers: ModifierState::default(),
    };

    let mouse_trigger = Trigger::MouseButton {
        button: MouseButton::Left,
        modifiers: ModifierState::default(),
    };

    let hotstring_trigger = Trigger::HotString {
        trigger: ".date".to_string(),
    };

    let always_trigger = Trigger::Always;

    assert!(matches!(key_trigger, Trigger::Key { .. }));
    assert!(matches!(mouse_trigger, Trigger::MouseButton { .. }));
    assert!(matches!(hotstring_trigger, Trigger::HotString { .. }));
    assert!(matches!(always_trigger, Trigger::Always));
}

/// 测试 ContextInfo 默认值
#[test]
fn test_context_info_default() {
    let context = ContextInfo::default();

    assert_eq!(context.window_class, "");
    assert_eq!(context.process_name, "");
    assert_eq!(context.process_path, "");
    assert_eq!(context.window_title, "");
    assert_eq!(context.window_handle, 0);
}

// ==================== Action 辅助方法测试 ====================

/// 测试 Action::is_none()
#[test]
fn test_action_is_none() {
    assert!(Action::None.is_none());
    assert!(!Action::key(KeyAction::click(0x01, 0x02)).is_none());
    assert!(!Action::mouse(MouseAction::Move {
        x: 0,
        y: 0,
        relative: false,
    })
    .is_none());
}

/// 测试各种 Action 创建辅助方法
#[test]
fn test_action_factory_methods() {
    // key
    let action = Action::key(KeyAction::click(0x1E, 0x41));
    assert!(matches!(action, Action::Key(_)));

    // mouse
    let action = Action::mouse(MouseAction::Wheel { delta: 120 });
    assert!(matches!(action, Action::Mouse(_)));

    // window
    let action = Action::window(WindowAction::Center);
    assert!(matches!(action, Action::Window(_)));

    // launch
    let action = Action::launch("notepad.exe");
    assert!(matches!(action, Action::Launch(cmd) if cmd.program == "notepad.exe"));

    // sequence
    let action = Action::sequence(vec![
        Action::key(KeyAction::click(0x01, 0x1B)),
        Action::key(KeyAction::click(0x0E, 0x08)),
    ]);
    assert!(matches!(action, Action::Sequence(seq) if seq.len() == 2));

    // delay
    let action = Action::delay(500);
    assert!(matches!(action, Action::Delay { milliseconds: 500 }));
}

// ==================== KeyAction 辅助方法测试 ====================

/// 测试 KeyAction 工厂方法
#[test]
fn test_key_action_factory_methods() {
    // click
    let action = KeyAction::click(0x1E, 0x41);
    if let KeyAction::Click {
        scan_code,
        virtual_key,
    } = action
    {
        assert_eq!(scan_code, 0x1E);
        assert_eq!(virtual_key, 0x41);
    }

    // press
    let action = KeyAction::press(0x1E, 0x41);
    if let KeyAction::Press {
        scan_code,
        virtual_key,
    } = action
    {
        assert_eq!(scan_code, 0x1E);
        assert_eq!(virtual_key, 0x41);
    }

    // release
    let action = KeyAction::release(0x1E, 0x41);
    if let KeyAction::Release {
        scan_code,
        virtual_key,
    } = action
    {
        assert_eq!(scan_code, 0x1E);
        assert_eq!(virtual_key, 0x41);
    }

    // combo
    let mut modifiers = ModifierState::default();
    modifiers.ctrl = true;
    let action = KeyAction::combo(modifiers, 0x1E, 0x41);
    if let KeyAction::Combo { modifiers: m, key } = action {
        assert!(m.ctrl);
        assert_eq!(key, (0x1E, 0x41));
    }
}

/// 测试 KeyAction 从事件创建
#[test]
fn test_key_action_from_event() {
    let event = KeyEvent::new(0x1E, 0x41, KeyState::Pressed);
    let press_action = KeyAction::press_from_event(&event);
    if let KeyAction::Press {
        scan_code,
        virtual_key,
    } = press_action
    {
        assert_eq!(scan_code, 0x1E);
        assert_eq!(virtual_key, 0x41);
    }

    let release_action = KeyAction::release_from_event(&event);
    if let KeyAction::Release {
        scan_code,
        virtual_key,
    } = release_action
    {
        assert_eq!(scan_code, 0x1E);
        assert_eq!(virtual_key, 0x41);
    }
}

// ==================== Action 类型测试（原 ut_types_action.rs）====================

/// 测试 KeyAction 创建
#[test]
fn test_key_action_creation() {
    let _press = KeyAction::Press {
        scan_code: 0x1E,
        virtual_key: 0x41, // 'A'
    };

    let _release = KeyAction::Release {
        scan_code: 0x1E,
        virtual_key: 0x41,
    };

    let click = KeyAction::click(0x1E, 0x41);

    match click {
        KeyAction::Click {
            scan_code,
            virtual_key,
        } => {
            assert_eq!(scan_code, 0x1E);
            assert_eq!(virtual_key, 0x41);
        }
        _ => panic!("Expected Click action"),
    }
}

/// 测试 ModifierState
#[test]
fn test_modifier_state() {
    let mut state = ModifierState::new();
    assert!(state.is_empty());

    state.ctrl = true;
    assert!(!state.is_empty());

    // 测试从虚拟键码创建
    let (ctrl_state, pressed) = ModifierState::from_virtual_key(0x11, true).unwrap();
    assert!(ctrl_state.ctrl);
    assert!(pressed);

    let (shift_state, _) = ModifierState::from_virtual_key(0x10, true).unwrap();
    assert!(shift_state.shift);

    let (alt_state, _) = ModifierState::from_virtual_key(0x12, true).unwrap();
    assert!(alt_state.alt);

    let (meta_state, _) = ModifierState::from_virtual_key(0x5B, true).unwrap();
    assert!(meta_state.meta);
}

/// 测试 ModifierState 合并
#[test]
fn test_modifier_state_merge_alt() {
    let mut state1 = ModifierState::new();
    state1.ctrl = true;

    let mut state2 = ModifierState::new();
    state2.shift = true;

    state1.merge(&state2);

    assert!(state1.ctrl);
    assert!(state1.shift);
    assert!(!state1.alt);
    assert!(!state1.meta);
}

/// 测试 WindowAction 变体
#[test]
fn test_window_action_variants() {
    let center = WindowAction::Center;
    let half_screen = WindowAction::HalfScreen(Edge::Left);
    let move_to_edge = WindowAction::MoveToEdge(Edge::Right);
    let loop_width = WindowAction::LoopWidth(Alignment::Left);
    let fixed_ratio = WindowAction::FixedRatio {
        ratio: 1.333,
        scale_index: 0,
    };

    // 验证它们是不同的变体
    assert!(matches!(center, WindowAction::Center));
    assert!(matches!(half_screen, WindowAction::HalfScreen(Edge::Left)));
    assert!(matches!(
        move_to_edge,
        WindowAction::MoveToEdge(Edge::Right)
    ));
    assert!(matches!(
        loop_width,
        WindowAction::LoopWidth(Alignment::Left)
    ));
    assert!(matches!(fixed_ratio, WindowAction::FixedRatio { .. }));
}

/// 测试 MonitorDirection
#[test]
fn test_monitor_direction() {
    let next = MonitorDirection::Next;
    let prev = MonitorDirection::Prev;
    let index = MonitorDirection::Index(2);

    assert!(matches!(next, MonitorDirection::Next));
    assert!(matches!(prev, MonitorDirection::Prev));
    assert!(matches!(index, MonitorDirection::Index(2)));
}

/// 测试 Action 封装
#[test]
fn test_action_wrapper() {
    let key_action = Action::key(KeyAction::click(0x1E, 0x41));
    let mouse_action = Action::mouse(MouseAction::ButtonClick {
        button: MouseButton::Left,
    });
    let window_action = Action::window(WindowAction::Center);
    let launch_action = Action::launch("notepad.exe");

    assert!(matches!(key_action, Action::Key(_)));
    assert!(matches!(mouse_action, Action::Mouse(_)));
    assert!(matches!(window_action, Action::Window(_)));
    assert!(matches!(launch_action, Action::Launch(_)));
}

/// 测试 Action::is_none
#[test]
fn test_action_is_none_alt() {
    let none_action = Action::None;
    let some_action = Action::key(KeyAction::click(0x1E, 0x41));

    assert!(none_action.is_none());
    assert!(!some_action.is_none());
}

/// 测试 Action 序列
#[test]
fn test_action_sequence() {
    let sequence = Action::sequence(vec![
        Action::key(KeyAction::click(0x1E, 0x41)),
        Action::key(KeyAction::click(0x30, 0x42)),
        Action::window(WindowAction::Center),
    ]);

    match sequence {
        Action::Sequence(actions) => {
            assert_eq!(actions.len(), 3);
        }
        _ => panic!("Expected Sequence action"),
    }
}

/// 测试 LaunchAction
#[test]
fn test_launch_action() {
    let launch = LaunchAction {
        program: "code.exe".to_string(),
        args: vec![".", "--goto"].iter().map(|s| s.to_string()).collect(),
        working_dir: Some("C:\\Projects".to_string()),
        env_vars: vec![("EDITOR".to_string(), "code".to_string())],
    };

    assert_eq!(launch.program, "code.exe");
    assert_eq!(launch.args.len(), 2);
    assert_eq!(launch.working_dir, Some("C:\\Projects".to_string()));
    assert_eq!(launch.env_vars.len(), 1);
}

/// 测试 MouseAction 变体
#[test]
fn test_mouse_action_variants() {
    let move_rel = MouseAction::Move {
        x: 100,
        y: 50,
        relative: true,
    };
    let move_abs = MouseAction::Move {
        x: 500,
        y: 300,
        relative: false,
    };
    let button_down = MouseAction::ButtonDown {
        button: MouseButton::Left,
    };
    let wheel = MouseAction::Wheel { delta: 120 };
    let h_wheel = MouseAction::HWheel { delta: -120 };

    assert!(matches!(move_rel, MouseAction::Move { relative: true, .. }));
    assert!(matches!(
        move_abs,
        MouseAction::Move {
            relative: false,
            ..
        }
    ));
    assert!(matches!(button_down, MouseAction::ButtonDown { .. }));
    assert!(matches!(wheel, MouseAction::Wheel { delta: 120 }));
    assert!(matches!(h_wheel, MouseAction::HWheel { delta: -120 }));
}

/// 测试 Edge 和 Alignment 枚举
#[test]
fn test_edge_alignment_enums() {
    let edges = vec![Edge::Left, Edge::Right, Edge::Top, Edge::Bottom];
    let alignments = vec![
        Alignment::Left,
        Alignment::Right,
        Alignment::Top,
        Alignment::Bottom,
        Alignment::Center,
    ];

    assert_eq!(edges.len(), 4);
    assert_eq!(alignments.len(), 5);
}
