// 运行时测试 - mapper, layer_manager, macro_player

use wakem::types::{
    Action, ContextCondition, InputEvent, KeyAction, KeyEvent, KeyState, Layer,
    LayerMode, MappingRule, ModifierState, Trigger,
};

/// 测试映射规则匹配
#[test]
fn test_mapping_rule_matching() {
    let rule = MappingRule::new(
        Trigger::key(0x1E, 0x41),                  // 'A' key
        Action::key(KeyAction::click(0x1F, 0x42)), // 'B' key
    );

    let event = InputEvent::Key(KeyEvent::new(0x1E, 0x41, KeyState::Pressed));
    assert!(rule.trigger.matches(&event));
}

/// 测试带修饰键的映射规则
#[test]
fn test_mapping_rule_with_modifiers() {
    let mut modifiers = ModifierState::new();
    modifiers.ctrl = true;
    let trigger = Trigger::key_with_modifiers(0x1E, 0x41, modifiers); // Ctrl + 'A'

    let rule = MappingRule::new(trigger, Action::key(KeyAction::click(0x1F, 0x42)));

    // 创建带 Ctrl 修饰符的事件
    let mut event_modifiers = ModifierState::new();
    event_modifiers.ctrl = true;
    let event = InputEvent::Key(
        KeyEvent::new(0x1E, 0x41, KeyState::Pressed).with_modifiers(event_modifiers),
    );

    assert!(rule.trigger.matches(&event));
}

/// 测试层创建
#[test]
fn test_layer_creation() {
    let layer = Layer::new("test", 0x3A, 0x14).with_mode(LayerMode::Hold);

    assert_eq!(layer.name, "test");
    assert_eq!(layer.activation_key, 0x3A);
    assert_eq!(layer.activation_vk, 0x14);
    assert!(matches!(layer.mode, LayerMode::Hold));
}

/// 测试层添加映射
#[test]
fn test_layer_add_mapping() {
    let mut layer = Layer::new("nav", 0x3A, 0x14);

    let trigger = Trigger::key(0x1E, 0x41);
    let action = Action::key(KeyAction::click(0x1F, 0x42));

    layer.add_mapping(trigger, action);
    assert_eq!(layer.mappings.len(), 1);
}

/// 测试层激活键检查
#[test]
fn test_layer_activation_key() {
    let layer = Layer::new("test", 0x3A, 0x14);

    assert!(layer.is_activation_key(0x3A, 0x14));
    assert!(!layer.is_activation_key(0x3B, 0x15));
}

/// 测试 Action 变体创建
#[test]
fn test_action_variants() {
    let key_action = Action::key(KeyAction::click(0x1E, 0x41));
    assert!(matches!(key_action, Action::Key(_)));

    let mouse_action = Action::mouse(wakem::types::MouseAction::Move {
        x: 100,
        y: 100,
        relative: false,
    });
    assert!(matches!(mouse_action, Action::Mouse(_)));

    let window_action = Action::window(wakem::types::WindowAction::Maximize);
    assert!(matches!(window_action, Action::Window(_)));

    let launch_action = Action::launch("notepad.exe");
    assert!(matches!(launch_action, Action::Launch(_)));

    let delay_action = Action::delay(100);
    assert!(matches!(delay_action, Action::Delay { .. }));
}

/// 测试 KeyAction 变体
#[test]
fn test_key_action_variants() {
    let press = KeyAction::Press {
        scan_code: 0x1E,
        virtual_key: 0x41,
    };
    assert!(matches!(press, KeyAction::Press { .. }));

    let release = KeyAction::Release {
        scan_code: 0x1E,
        virtual_key: 0x41,
    };
    assert!(matches!(release, KeyAction::Release { .. }));

    let click = KeyAction::click(0x1E, 0x41);
    assert!(matches!(click, KeyAction::Click { .. }));

    let mut modifiers = ModifierState::new();
    modifiers.ctrl = true;
    let combo = KeyAction::combo(modifiers, 0x1E, 0x41);
    assert!(matches!(combo, KeyAction::Combo { .. }));

    let type_text = KeyAction::TypeText("hello".to_string());
    assert!(matches!(type_text, KeyAction::TypeText(_)));

    let none = KeyAction::None;
    assert!(matches!(none, KeyAction::None));
}

/// 测试 KeyEvent 创建
#[test]
fn test_key_event_creation() {
    let event = KeyEvent::new(0x1E, 0x41, KeyState::Pressed);

    assert_eq!(event.scan_code, 0x1E);
    assert_eq!(event.virtual_key, 0x41);
    assert!(matches!(event.state, KeyState::Pressed));
}

/// 测试 KeyEvent 带修饰符
#[test]
fn test_key_event_with_modifiers() {
    let mut modifiers = ModifierState::new();
    modifiers.ctrl = true;
    modifiers.shift = true;
    let event = KeyEvent::new(0x1E, 0x41, KeyState::Pressed).with_modifiers(modifiers);

    assert!(event.modifiers.ctrl);
    assert!(event.modifiers.shift);
    assert!(!event.modifiers.alt);
}

/// 测试 KeyEvent 注入标记
#[test]
fn test_key_event_injected() {
    let event = KeyEvent::new(0x1E, 0x41, KeyState::Pressed).injected();

    assert!(event.is_injected);
}

/// 测试修饰键状态
#[test]
fn test_modifier_state() {
    let mut state = ModifierState::new();
    state.ctrl = true;
    assert!(state.ctrl);
    assert!(!state.shift);

    let mut state = ModifierState::new();
    state.shift = true;
    assert!(!state.ctrl);
    assert!(state.shift);

    let mut state = ModifierState::new();
    state.alt = true;
    assert!(state.alt);

    let mut state = ModifierState::new();
    state.meta = true;
    assert!(state.meta);
}

/// 测试修饰键合并
#[test]
fn test_modifier_state_merge() {
    let mut state1 = ModifierState::new();
    state1.ctrl = true;
    let mut state2 = ModifierState::new();
    state2.shift = true;

    state1.merge(&state2);

    assert!(state1.ctrl);
    assert!(state1.shift);
}

/// 测试触发器匹配
#[test]
fn test_trigger_matching() {
    let trigger = Trigger::key(0x1E, 0x41);

    let matching_event = InputEvent::Key(KeyEvent::new(0x1E, 0x41, KeyState::Pressed));
    assert!(trigger.matches(&matching_event));

    let non_matching_event =
        InputEvent::Key(KeyEvent::new(0x1F, 0x42, KeyState::Pressed));
    assert!(!trigger.matches(&non_matching_event));
}

/// 测试窗口动作变体
#[test]
fn test_window_action_variants() {
    use wakem::types::WindowAction;

    let center = WindowAction::Center;
    assert!(matches!(center, WindowAction::Center));

    let maximize = WindowAction::Maximize;
    assert!(matches!(maximize, WindowAction::Maximize));

    let minimize = WindowAction::Minimize;
    assert!(matches!(minimize, WindowAction::Minimize));

    let close = WindowAction::Close;
    assert!(matches!(close, WindowAction::Close));

    let resize = WindowAction::Resize {
        width: 800,
        height: 600,
    };
    assert!(matches!(resize, WindowAction::Resize { .. }));
}

/// 测试鼠标动作变体
#[test]
fn test_mouse_action_variants() {
    use wakem::types::MouseAction;

    let move_action = MouseAction::Move {
        x: 100,
        y: 200,
        relative: false,
    };
    assert!(matches!(move_action, MouseAction::Move { .. }));

    let relative_move = MouseAction::Move {
        x: 10,
        y: -10,
        relative: true,
    };
    assert!(matches!(relative_move, MouseAction::Move { .. }));
}

/// 测试启动动作
#[test]
fn test_launch_action() {
    use wakem::types::LaunchAction;

    let action = LaunchAction {
        program: "notepad.exe".to_string(),
        args: vec![],
        working_dir: None,
        env_vars: vec![],
    };
    assert_eq!(action.program, "notepad.exe");
    assert!(action.args.is_empty());

    let action = LaunchAction {
        program: "code".to_string(),
        args: vec![".".to_string()],
        working_dir: None,
        env_vars: vec![],
    };
    assert_eq!(action.program, "code");
    assert_eq!(action.args, vec!["."]);
}

/// 测试系统动作变体
#[test]
fn test_system_action_variants() {
    use wakem::types::SystemAction;

    assert!(matches!(SystemAction::VolumeUp, SystemAction::VolumeUp));
    assert!(matches!(SystemAction::VolumeDown, SystemAction::VolumeDown));
    assert!(matches!(SystemAction::VolumeMute, SystemAction::VolumeMute));
    assert!(matches!(
        SystemAction::BrightnessUp,
        SystemAction::BrightnessUp
    ));
    assert!(matches!(
        SystemAction::BrightnessDown,
        SystemAction::BrightnessDown
    ));
}

/// 测试动作序列
#[test]
fn test_action_sequence() {
    let actions = vec![
        Action::key(KeyAction::click(0x1E, 0x41)),
        Action::key(KeyAction::click(0x1F, 0x42)),
        Action::key(KeyAction::click(0x20, 0x43)),
    ];

    assert_eq!(actions.len(), 3);
}

/// 测试事件序列
#[test]
fn test_event_sequence() {
    let events = vec![
        InputEvent::Key(KeyEvent::new(0x1E, 0x41, KeyState::Pressed)),
        InputEvent::Key(KeyEvent::new(0x1E, 0x41, KeyState::Released)),
        InputEvent::Key(KeyEvent::new(0x1F, 0x42, KeyState::Pressed)),
        InputEvent::Key(KeyEvent::new(0x1F, 0x42, KeyState::Released)),
    ];

    assert_eq!(events.len(), 4);
}

/// 测试上下文条件创建
#[test]
fn test_context_condition() {
    let context = ContextCondition::new()
        .with_process_name("notepad.exe")
        .with_window_class("NotepadClass");

    assert!(context.process_name.is_some());
    assert_eq!(context.process_name.unwrap(), "notepad.exe");
    assert!(context.window_class.is_some());
}

/// 测试映射规则启用/禁用
#[test]
fn test_mapping_rule_enabled() {
    let rule = MappingRule::new(
        Trigger::key(0x1E, 0x41),
        Action::key(KeyAction::click(0x1F, 0x42)),
    );

    assert!(rule.enabled);
}

/// 测试 LayerMode 变体
#[test]
fn test_layer_modes() {
    let toggle_mode = LayerMode::Toggle;
    assert!(matches!(toggle_mode, LayerMode::Toggle));

    let hold_mode = LayerMode::Hold;
    assert!(matches!(hold_mode, LayerMode::Hold));
}

/// 测试 Trigger 变体
#[test]
fn test_trigger_variants() {
    let key_trigger = Trigger::key(0x1E, 0x41);
    assert!(matches!(key_trigger, Trigger::Key { .. }));

    let mouse_trigger = Trigger::MouseButton {
        button: wakem::types::MouseButton::Left,
        modifiers: ModifierState::new(),
    };
    assert!(matches!(mouse_trigger, Trigger::MouseButton { .. }));

    let hotstring_trigger = Trigger::HotString {
        trigger: "test".to_string(),
    };
    assert!(matches!(hotstring_trigger, Trigger::HotString { .. }));
}

/// 测试空 Action
#[test]
fn test_action_none() {
    let none = Action::None;
    assert!(none.is_none());
    assert!(matches!(none, Action::None));
}

/// 测试 KeyState 变体
#[test]
fn test_key_state_variants() {
    let pressed = KeyState::Pressed;
    assert!(matches!(pressed, KeyState::Pressed));

    let released = KeyState::Released;
    assert!(matches!(released, KeyState::Released));
}

/// 测试 MouseButton 变体
#[test]
fn test_mouse_button_variants() {
    use wakem::types::MouseButton;

    assert!(matches!(MouseButton::Left, MouseButton::Left));
    assert!(matches!(MouseButton::Right, MouseButton::Right));
    assert!(matches!(MouseButton::Middle, MouseButton::Middle));
    assert!(matches!(MouseButton::X1, MouseButton::X1));
    assert!(matches!(MouseButton::X2, MouseButton::X2));
}

/// 测试修饰键从虚拟键码创建
#[test]
fn test_modifier_from_virtual_key() {
    // VK_SHIFT = 0x10
    let (state, pressed) = ModifierState::from_virtual_key(0x10, true).unwrap();
    assert!(state.shift);
    assert!(pressed);

    // VK_CONTROL = 0x11
    let (state, pressed) = ModifierState::from_virtual_key(0x11, true).unwrap();
    assert!(state.ctrl);
    assert!(pressed);

    // VK_MENU = 0x12 (Alt)
    let (state, pressed) = ModifierState::from_virtual_key(0x12, true).unwrap();
    assert!(state.alt);
    assert!(pressed);

    // VK_LWIN = 0x5B (Meta)
    let (state, pressed) = ModifierState::from_virtual_key(0x5B, true).unwrap();
    assert!(state.meta);
    assert!(pressed);

    // 非修饰键应返回 None
    assert!(ModifierState::from_virtual_key(0x41, true).is_none());
}
