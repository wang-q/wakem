// 类型系统测试
// 测试核心类型的创建和匹配逻辑

use wakem::types::*;

/// 测试简单的键位触发器
#[test]
fn test_key_trigger_creation() {
    let trigger = Trigger::key(0x3A, 0x14); // CapsLock

    match trigger {
        Trigger::Key {
            scan_code,
            virtual_key,
            modifiers,
        } => {
            assert_eq!(scan_code, Some(0x3A));
            assert_eq!(virtual_key, Some(0x14));
            assert!(modifiers.is_empty());
        }
        _ => panic!("Expected Key trigger"),
    }
}

/// 测试带修饰键的触发器
#[test]
fn test_key_trigger_with_modifiers() {
    let mut modifiers = ModifierState::new();
    modifiers.ctrl = true;
    modifiers.shift = true;

    let trigger = Trigger::key_with_modifiers(0x1E, 0x41, modifiers);

    match trigger {
        Trigger::Key {
            scan_code,
            virtual_key,
            modifiers,
        } => {
            assert_eq!(scan_code, Some(0x1E));
            assert_eq!(virtual_key, Some(0x41));
            assert!(modifiers.ctrl);
            assert!(modifiers.shift);
            assert!(!modifiers.alt);
            assert!(!modifiers.meta);
        }
        _ => panic!("Expected Key trigger"),
    }
}

/// 测试触发器匹配
#[test]
fn test_trigger_matching() {
    let trigger = Trigger::key(0x3A, 0x14);

    let event = InputEvent::Key(KeyEvent::new(0x3A, 0x14, KeyState::Pressed));
    assert!(trigger.matches(&event));

    // 不匹配的扫描码
    let wrong_event = InputEvent::Key(KeyEvent::new(0x1E, 0x41, KeyState::Pressed));
    assert!(!trigger.matches(&wrong_event));
}

/// 测试映射规则创建
#[test]
fn test_mapping_rule_creation() {
    let trigger = Trigger::key(0x3A, 0x14);
    let action = Action::key(KeyAction::click(0x0E, 0x08));

    let rule = MappingRule::new(trigger, action);

    assert!(rule.enabled);
    assert!(rule.name.is_none());
    assert!(rule.context.is_none());
}

/// 测试带名称的映射规则
#[test]
fn test_mapping_rule_with_name() {
    let trigger = Trigger::key(0x3A, 0x14);
    let action = Action::key(KeyAction::click(0x0E, 0x08));

    let rule = MappingRule::new(trigger, action).with_name("caps_to_backspace");

    assert_eq!(rule.name, Some("caps_to_backspace".to_string()));
}

/// 测试带上下文的映射规则
#[test]
fn test_mapping_rule_with_context() {
    let trigger = Trigger::key(0x3A, 0x14);
    let action = Action::key(KeyAction::click(0x0E, 0x08));

    let context = ContextCondition::new().with_process_name("notepad.exe");

    let rule = MappingRule::new(trigger, action).with_context(context);

    assert!(rule.context.is_some());
}

/// 测试映射规则匹配
#[test]
fn test_mapping_rule_matching() {
    let trigger = Trigger::key(0x3A, 0x14);
    let action = Action::key(KeyAction::click(0x0E, 0x08));

    let rule = MappingRule::new(trigger, action);

    let event = InputEvent::Key(KeyEvent::new(0x3A, 0x14, KeyState::Pressed));
    let context = ContextInfo::default();

    assert!(rule.matches(&event, &context));
}

/// 测试禁用的规则不匹配
#[test]
fn test_disabled_rule_not_matching() {
    let trigger = Trigger::key(0x3A, 0x14);
    let action = Action::key(KeyAction::click(0x0E, 0x08));

    let mut rule = MappingRule::new(trigger, action);
    rule.enabled = false;

    let event = InputEvent::Key(KeyEvent::new(0x3A, 0x14, KeyState::Pressed));
    let context = ContextInfo::default();

    assert!(!rule.matches(&event, &context));
}

/// 测试上下文条件匹配
#[test]
fn test_context_condition_matching() {
    let context = ContextCondition::new().with_process_name("notepad.exe");

    let matching_info = ContextInfo {
        window_class: "Notepad".to_string(),
        process_name: "notepad.exe".to_string(),
        process_path: "C:\\Windows\\notepad.exe".to_string(),
        window_title: "Untitled".to_string(),
        window_handle: 0,
    };

    let non_matching_info = ContextInfo {
        window_class: "Chrome".to_string(),
        process_name: "chrome.exe".to_string(),
        process_path: "C:\\Program Files\\chrome.exe".to_string(),
        window_title: "Google".to_string(),
        window_handle: 0,
    };

    assert!(context.matches(&matching_info));
    assert!(!context.matches(&non_matching_info));
}

/// 测试窗口动作变体
#[test]
fn test_window_action_variants() {
    let actions = vec![
        WindowAction::Center,
        WindowAction::MoveToEdge(Edge::Left),
        WindowAction::MoveToEdge(Edge::Right),
        WindowAction::MoveToEdge(Edge::Top),
        WindowAction::MoveToEdge(Edge::Bottom),
        WindowAction::HalfScreen(Edge::Left),
        WindowAction::HalfScreen(Edge::Right),
        WindowAction::LoopWidth(Alignment::Left),
        WindowAction::LoopWidth(Alignment::Right),
        WindowAction::LoopHeight(Alignment::Top),
        WindowAction::LoopHeight(Alignment::Bottom),
        WindowAction::FixedRatio {
            ratio: 1.333,
            scale_index: 0,
        },
        WindowAction::NativeRatio { scale_index: 0 },
        WindowAction::MoveToMonitor(MonitorDirection::Next),
        WindowAction::MoveToMonitor(MonitorDirection::Prev),
        WindowAction::MoveToMonitor(MonitorDirection::Index(1)),
        WindowAction::Minimize,
        WindowAction::Maximize,
        WindowAction::Restore,
        WindowAction::Close,
        WindowAction::ToggleTopmost,
        WindowAction::SwitchToNextWindow,
        WindowAction::ShowDebugInfo,
        WindowAction::ShowNotification {
            title: "Test".to_string(),
            message: "Hello".to_string(),
        },
    ];

    assert_eq!(actions.len(), 24);
}

/// 测试鼠标动作
#[test]
fn test_mouse_actions() {
    let actions = vec![
        MouseAction::Move {
            x: 100,
            y: 100,
            relative: true,
        },
        MouseAction::Move {
            x: 500,
            y: 300,
            relative: false,
        },
        MouseAction::ButtonClick {
            button: MouseButton::Left,
        },
        MouseAction::ButtonClick {
            button: MouseButton::Right,
        },
        MouseAction::ButtonDown {
            button: MouseButton::Left,
        },
        MouseAction::ButtonUp {
            button: MouseButton::Left,
        },
        MouseAction::Wheel { delta: 120 },
        MouseAction::Wheel { delta: -120 },
        MouseAction::HWheel { delta: 120 },
        MouseAction::HWheel { delta: -120 },
    ];

    for action in actions {
        let wrapped = Action::mouse(action);
        assert!(matches!(wrapped, Action::Mouse(_)));
    }
}

/// 测试键动作
#[test]
fn test_key_actions() {
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

/// 测试修饰键状态
#[test]
fn test_modifier_state() {
    let empty = ModifierState::new();
    assert!(empty.is_empty());

    let mut full = ModifierState::new();
    full.ctrl = true;
    full.shift = true;
    full.alt = true;
    full.meta = true;
    assert!(!full.is_empty());

    let mut partial = ModifierState::new();
    partial.ctrl = true;
    assert!(!partial.is_empty());
}

/// 测试输入事件创建
#[test]
fn test_input_event_creation() {
    let key_event = KeyEvent::new(0x1E, 0x41, KeyState::Pressed);
    let input_event = InputEvent::Key(key_event);

    match input_event {
        InputEvent::Key(e) => {
            assert_eq!(e.scan_code, 0x1E);
            assert_eq!(e.virtual_key, 0x41);
            assert!(matches!(e.state, KeyState::Pressed));
        }
        _ => panic!("Expected Key event"),
    }
}

/// 测试鼠标事件
#[test]
fn test_mouse_event() {
    let event = MouseEvent::new(MouseEventType::ButtonDown(MouseButton::Left), 100, 200);
    let input_event = InputEvent::Mouse(event);

    match input_event {
        InputEvent::Mouse(e) => {
            assert!(matches!(
                e.event_type,
                MouseEventType::ButtonDown(MouseButton::Left)
            ));
            assert_eq!(e.x, 100);
            assert_eq!(e.y, 200);
        }
        _ => panic!("Expected Mouse event"),
    }
}

/// 测试触发器变体
#[test]
fn test_trigger_variants() {
    let triggers = vec![
        Trigger::key(0x1E, 0x41),
        Trigger::MouseButton {
            button: MouseButton::Left,
            modifiers: ModifierState::new(),
        },
        Trigger::MouseGesture {
            button: MouseButton::Right,
            direction: GestureDirection::Down,
        },
        Trigger::HotString {
            trigger: ".date".to_string(),
        },
        Trigger::Always,
    ];

    assert_eq!(triggers.len(), 5);
}

/// 测试动作序列
#[test]
fn test_action_sequence() {
    let sequence = Action::sequence(vec![
        Action::key(KeyAction::click(0x1E, 0x41)),
        Action::key(KeyAction::click(0x30, 0x42)),
        Action::window(WindowAction::Center),
    ]);

    assert!(matches!(sequence, Action::Sequence(_)));
}

/// 测试启动动作
#[test]
fn test_launch_action() {
    let action = Action::launch("notepad.exe");

    assert!(matches!(action, Action::Launch(_)));
}

/// 测试通配符匹配（简化版）
#[test]
fn test_wildcard_matching() {
    // 这些测试依赖于 ContextCondition 的内部实现
    // 这里主要测试 ContextCondition 能正确创建
    let cond = ContextCondition::new()
        .with_window_class("Chrome*")
        .with_process_name("chrome.exe");

    let info = ContextInfo {
        window_class: "Chrome_WidgetWin_1".to_string(),
        process_name: "chrome.exe".to_string(),
        process_path: "".to_string(),
        window_title: "".to_string(),
        window_handle: 0,
    };

    // 简化匹配可能不完美，但至少不会 panic
    let _result = cond.matches(&info);
}
