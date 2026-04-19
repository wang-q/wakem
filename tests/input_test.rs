// 输入事件测试
// 测试键盘和鼠标事件处理

use wakem::types::*;

/// 测试 KeyEvent 创建
#[test]
fn test_key_event_creation() {
    let event = KeyEvent::new(0x1E, 0x41, KeyState::Pressed);
    
    assert_eq!(event.scan_code, 0x1E);
    assert_eq!(event.virtual_key, 0x41);
    assert!(matches!(event.state, KeyState::Pressed));
    assert_eq!(event.device_type, DeviceType::Keyboard);
    assert!(!event.is_injected);
}

/// 测试 KeyState 枚举
#[test]
fn test_key_state_enum() {
    let pressed = KeyState::Pressed;
    let released = KeyState::Released;
    
    assert!(matches!(pressed, KeyState::Pressed));
    assert!(matches!(released, KeyState::Released));
}

/// 测试 MouseEvent 创建
#[test]
fn test_mouse_event_creation() {
    let event = MouseEvent::new(MouseEventType::Move, 100, 200);
    
    assert_eq!(event.x, 100);
    assert_eq!(event.y, 200);
    assert!(matches!(event.event_type, MouseEventType::Move));
    assert!(!event.is_injected);
}

/// 测试 MouseButton 枚举
#[test]
fn test_mouse_button_enum() {
    let left = MouseButton::Left;
    let right = MouseButton::Right;
    let middle = MouseButton::Middle;
    let x1 = MouseButton::X1;
    let x2 = MouseButton::X2;
    
    assert!(matches!(left, MouseButton::Left));
    assert!(matches!(right, MouseButton::Right));
    assert!(matches!(middle, MouseButton::Middle));
    assert!(matches!(x1, MouseButton::X1));
    assert!(matches!(x2, MouseButton::X2));
}

/// 测试 InputEvent 变体
#[test]
fn test_input_event_variants() {
    let key_event = InputEvent::Key(KeyEvent::new(0x1E, 0x41, KeyState::Pressed));
    
    let mouse_event = InputEvent::Mouse(MouseEvent::new(MouseEventType::Move, 100, 200));
    
    assert!(matches!(key_event, InputEvent::Key(_)));
    assert!(matches!(mouse_event, InputEvent::Mouse(_)));
}

/// 测试 DeviceType 枚举
#[test]
fn test_device_type_enum() {
    let keyboard = DeviceType::Keyboard;
    let mouse = DeviceType::Mouse;
    
    assert!(matches!(keyboard, DeviceType::Keyboard));
    assert!(matches!(mouse, DeviceType::Mouse));
}

/// 测试 ModifierState 默认值
#[test]
fn test_modifier_state_default() {
    let state = ModifierState::default();
    
    assert!(!state.shift);
    assert!(!state.ctrl);
    assert!(!state.alt);
    assert!(!state.meta);
}

/// 测试 ModifierState 从虚拟键码创建 - Shift
#[test]
fn test_modifier_state_from_vk_shift() {
    // VK_SHIFT = 0x10
    let (state, pressed) = ModifierState::from_virtual_key(0x10, true).unwrap();
    assert!(state.shift);
    assert!(!state.ctrl);
    assert!(!state.alt);
    assert!(!state.meta);
    assert!(pressed);
    
    // VK_LSHIFT = 0xA0
    let (state, _) = ModifierState::from_virtual_key(0xA0, true).unwrap();
    assert!(state.shift);
    
    // VK_RSHIFT = 0xA1
    let (state, _) = ModifierState::from_virtual_key(0xA1, true).unwrap();
    assert!(state.shift);
}

/// 测试 ModifierState 从虚拟键码创建 - Control
#[test]
fn test_modifier_state_from_vk_control() {
    // VK_CONTROL = 0x11
    let (state, pressed) = ModifierState::from_virtual_key(0x11, true).unwrap();
    assert!(!state.shift);
    assert!(state.ctrl);
    assert!(!state.alt);
    assert!(!state.meta);
    assert!(pressed);
    
    // VK_LCONTROL = 0xA2
    let (state, _) = ModifierState::from_virtual_key(0xA2, true).unwrap();
    assert!(state.ctrl);
    
    // VK_RCONTROL = 0xA3
    let (state, _) = ModifierState::from_virtual_key(0xA3, true).unwrap();
    assert!(state.ctrl);
}

/// 测试 ModifierState 从虚拟键码创建 - Alt
#[test]
fn test_modifier_state_from_vk_alt() {
    // VK_MENU = 0x12
    let (state, pressed) = ModifierState::from_virtual_key(0x12, true).unwrap();
    assert!(!state.shift);
    assert!(!state.ctrl);
    assert!(state.alt);
    assert!(!state.meta);
    assert!(pressed);
    
    // VK_LMENU = 0xA4
    let (state, _) = ModifierState::from_virtual_key(0xA4, true).unwrap();
    assert!(state.alt);
    
    // VK_RMENU = 0xA5
    let (state, _) = ModifierState::from_virtual_key(0xA5, true).unwrap();
    assert!(state.alt);
}

/// 测试 ModifierState 从虚拟键码创建 - Meta/Win
#[test]
fn test_modifier_state_from_vk_meta() {
    // VK_LWIN = 0x5B
    let (state, pressed) = ModifierState::from_virtual_key(0x5B, true).unwrap();
    assert!(!state.shift);
    assert!(!state.ctrl);
    assert!(!state.alt);
    assert!(state.meta);
    assert!(pressed);
    
    // VK_RWIN = 0x5C
    let (state, _) = ModifierState::from_virtual_key(0x5C, true).unwrap();
    assert!(state.meta);
}

/// 测试 ModifierState 从虚拟键码创建 - 非修饰键
#[test]
fn test_modifier_state_from_vk_non_modifier() {
    // 'A' key = 0x41
    let result = ModifierState::from_virtual_key(0x41, true);
    assert!(result.is_none());
    
    // '1' key = 0x31
    let result = ModifierState::from_virtual_key(0x31, true);
    assert!(result.is_none());
}

/// 测试 ModifierState 释放状态
#[test]
fn test_modifier_state_release() {
    let (state, pressed) = ModifierState::from_virtual_key(0x10, false).unwrap();
    assert!(!state.shift); // 释放时 shift 为 false
    assert!(!pressed);
}

/// 测试 ModifierState 合并
#[test]
fn test_modifier_state_merge_multiple() {
    let mut state1 = ModifierState::new();
    state1.ctrl = true;
    
    let mut state2 = ModifierState::new();
    state2.shift = true;
    
    let mut state3 = ModifierState::new();
    state3.alt = true;
    
    state1.merge(&state2);
    state1.merge(&state3);
    
    assert!(state1.ctrl);
    assert!(state1.shift);
    assert!(state1.alt);
    assert!(!state1.meta);
}

/// 测试复杂的事件序列
#[test]
fn test_event_sequence() {
    let events = vec![
        InputEvent::Key(KeyEvent::new(0x1D, 0x11, KeyState::Pressed).with_modifiers(ModifierState { ctrl: true, ..Default::default() })), // Ctrl
        InputEvent::Key(KeyEvent::new(0x1E, 0x41, KeyState::Pressed).with_modifiers(ModifierState { ctrl: true, ..Default::default() })), // 'A'
        InputEvent::Key(KeyEvent::new(0x1E, 0x41, KeyState::Released).with_modifiers(ModifierState { ctrl: true, ..Default::default() })), // 'A' release
        InputEvent::Key(KeyEvent::new(0x1D, 0x11, KeyState::Released)), // Ctrl release
    ];
    
    assert_eq!(events.len(), 4);
}

/// 测试时间戳函数
#[test]
fn test_timestamp() {
    let ts1 = now();
    std::thread::sleep(std::time::Duration::from_millis(10));
    let ts2 = now();
    
    assert!(ts2 >= ts1);
    assert!(ts2 - ts1 >= 10);
}

/// 测试鼠标按钮按下事件
#[test]
fn test_mouse_button_down_event() {
    let event = MouseEvent::new(MouseEventType::ButtonDown(MouseButton::Left), 100, 100);
    
    assert!(event.is_button_down(MouseButton::Left));
    assert!(!event.is_button_down(MouseButton::Right));
}

/// 测试鼠标按钮释放事件
#[test]
fn test_mouse_button_up_event() {
    let event = MouseEvent::new(MouseEventType::ButtonUp(MouseButton::Right), 100, 100);
    
    assert!(event.is_button_up(MouseButton::Right));
    assert!(!event.is_button_up(MouseButton::Left));
}

/// 测试鼠标滚轮事件
#[test]
fn test_mouse_wheel_event() {
    let event_up = MouseEvent::new(MouseEventType::Wheel(120), 100, 100);
    let event_down = MouseEvent::new(MouseEventType::Wheel(-120), 100, 100);
    
    assert!(matches!(event_up.event_type, MouseEventType::Wheel(120)));
    assert!(matches!(event_down.event_type, MouseEventType::Wheel(-120)));
}

/// 测试鼠标水平滚轮事件
#[test]
fn test_mouse_hwheel_event() {
    let event_right = MouseEvent::new(MouseEventType::HWheel(120), 100, 100);
    let event_left = MouseEvent::new(MouseEventType::HWheel(-120), 100, 100);
    
    assert!(matches!(event_right.event_type, MouseEventType::HWheel(120)));
    assert!(matches!(event_left.event_type, MouseEventType::HWheel(-120)));
}

/// 测试 KeyEvent 是否是修饰键
#[test]
fn test_key_event_is_modifier() {
    let shift = KeyEvent::new(0x2A, 0x10, KeyState::Pressed);
    let ctrl = KeyEvent::new(0x1D, 0x11, KeyState::Pressed);
    let alt = KeyEvent::new(0x38, 0x12, KeyState::Pressed);
    let win = KeyEvent::new(0x5B, 0x5B, KeyState::Pressed);
    let a_key = KeyEvent::new(0x1E, 0x41, KeyState::Pressed);
    
    assert!(shift.is_modifier());
    assert!(ctrl.is_modifier());
    assert!(alt.is_modifier());
    assert!(win.is_modifier());
    assert!(!a_key.is_modifier());
}

/// 测试 KeyEvent 修饰键标识符
#[test]
fn test_key_event_modifier_identifier() {
    let shift = KeyEvent::new(0x2A, 0x10, KeyState::Pressed);
    let ctrl = KeyEvent::new(0x1D, 0x11, KeyState::Pressed);
    let alt = KeyEvent::new(0x38, 0x12, KeyState::Pressed);
    let win = KeyEvent::new(0x5B, 0x5B, KeyState::Pressed);
    let a_key = KeyEvent::new(0x1E, 0x41, KeyState::Pressed);
    
    assert_eq!(shift.modifier_identifier(), Some("Shift"));
    assert_eq!(ctrl.modifier_identifier(), Some("Control"));
    assert_eq!(alt.modifier_identifier(), Some("Alt"));
    assert_eq!(win.modifier_identifier(), Some("Meta"));
    assert_eq!(a_key.modifier_identifier(), None);
}

/// 测试 KeyEvent with_modifiers
#[test]
fn test_key_event_with_modifiers() {
    let mut modifiers = ModifierState::new();
    modifiers.ctrl = true;
    modifiers.shift = true;
    
    let event = KeyEvent::new(0x1E, 0x41, KeyState::Pressed)
        .with_modifiers(modifiers);
    
    assert!(event.modifiers.ctrl);
    assert!(event.modifiers.shift);
    assert!(!event.modifiers.alt);
}

/// 测试 KeyEvent injected
#[test]
fn test_key_event_injected() {
    let event = KeyEvent::new(0x1E, 0x41, KeyState::Pressed).injected();
    
    assert!(event.is_injected);
}

/// 测试 MouseEvent injected
#[test]
fn test_mouse_event_injected() {
    let event = MouseEvent::new(MouseEventType::Move, 100, 100).injected();
    
    assert!(event.is_injected);
}

/// 测试 InputEvent timestamp
#[test]
fn test_input_event_timestamp() {
    let key_event = InputEvent::Key(KeyEvent::new(0x1E, 0x41, KeyState::Pressed));
    let mouse_event = InputEvent::Mouse(MouseEvent::new(MouseEventType::Move, 100, 100));
    
    assert!(key_event.timestamp() > 0);
    assert!(mouse_event.timestamp() > 0);
}

/// 测试 InputEvent is_injected
#[test]
fn test_input_event_is_injected() {
    let key_event = InputEvent::Key(KeyEvent::new(0x1E, 0x41, KeyState::Pressed));
    let injected_key = InputEvent::Key(KeyEvent::new(0x1E, 0x41, KeyState::Pressed).injected());
    
    assert!(!key_event.is_injected());
    assert!(injected_key.is_injected());
}
