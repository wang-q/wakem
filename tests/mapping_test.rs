// 映射规则测试
// 测试触发条件和上下文匹配

use wakem::types::*;

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
fn test_mapping_rule_with_name() {
    let trigger = Trigger::key(0x1E, 0x41);
    let action = Action::window(WindowAction::Center);
    
    let rule = MappingRule::new(trigger, action)
        .with_name("Center Window");
    
    assert_eq!(rule.name, Some("Center Window".to_string()));
}

/// 测试 MappingRule with_context
#[test]
fn test_mapping_rule_with_context() {
    let trigger = Trigger::key(0x1E, 0x41);
    let action = Action::window(WindowAction::Center);
    
    let context = ContextCondition::new()
        .with_process_name("notepad.exe");
    
    let rule = MappingRule::new(trigger, action)
        .with_context(context);
    
    assert!(rule.context.is_some());
}

/// 测试 ContextCondition 创建
#[test]
fn test_context_condition_creation() {
    let cond = ContextCondition::new()
        .with_window_class("Notepad")
        .with_process_name("notepad.exe")
        .with_window_title("*Untitled*");
    
    assert_eq!(cond.window_class, Some("Notepad".to_string()));
    assert_eq!(cond.process_name, Some("notepad.exe".to_string()));
    assert_eq!(cond.window_title, Some("*Untitled*".to_string()));
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
    
    assert!(cond.matches(&context));
}

/// 测试 ContextCondition 进程名匹配
#[test]
fn test_context_condition_process_match() {
    let cond = ContextCondition::new()
        .with_process_name("notepad.exe");
    
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
    
    assert!(cond.matches(&matching_context));
    assert!(!cond.matches(&non_matching_context));
}

/// 测试 Trigger::key 创建
#[test]
fn test_trigger_key_creation() {
    let trigger = Trigger::key(0x1E, 0x41);
    
    match trigger {
        Trigger::Key { scan_code, virtual_key, modifiers } => {
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
        Trigger::Key { scan_code, virtual_key, modifiers: m } => {
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
    
    let gesture_trigger = Trigger::MouseGesture {
        button: MouseButton::Right,
        direction: GestureDirection::Down,
    };
    
    let hotstring_trigger = Trigger::HotString {
        trigger: ".date".to_string(),
    };
    
    let always_trigger = Trigger::Always;
    
    assert!(matches!(key_trigger, Trigger::Key { .. }));
    assert!(matches!(mouse_trigger, Trigger::MouseButton { .. }));
    assert!(matches!(gesture_trigger, Trigger::MouseGesture { .. }));
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

/// 测试 GestureDirection 枚举
#[test]
fn test_gesture_direction() {
    let directions = vec![
        GestureDirection::Up,
        GestureDirection::Down,
        GestureDirection::Left,
        GestureDirection::Right,
        GestureDirection::Circle,
    ];
    
    assert_eq!(directions.len(), 5);
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
    
    assert!(cond.matches(&full_match));
    assert!(!cond.matches(&partial_match));
}
