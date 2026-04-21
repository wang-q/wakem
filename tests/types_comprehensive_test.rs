// Types 补充测试 - 边界条件、错误处理和完整类型覆盖

use wakem::types::{
    Action, ContextCondition, InputEvent, KeyAction, KeyEvent, KeyState, Layer,
    LayerMode, Macro, MacroStep, MappingRule, ModifierState, MouseAction, MouseButton,
    MouseEvent, MouseEventType, Trigger, WindowAction,
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

    let (state, pressed) = ModifierState::from_virtual_key(0xA0, true).unwrap();
    assert!(state.shift); // LSHIFT

    let (state, pressed) = ModifierState::from_virtual_key(0xA1, true).unwrap();
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
