// Action 类型测试
// 测试动作类型的创建和序列化

use wakem_common::types::*;

/// 测试 KeyAction 创建
#[test]
fn test_key_action_creation() {
    let press = KeyAction::Press {
        scan_code: 0x1E,
        virtual_key: 0x41, // 'A'
    };
    
    let release = KeyAction::Release {
        scan_code: 0x1E,
        virtual_key: 0x41,
    };
    
    let click = KeyAction::click(0x1E, 0x41);
    
    match click {
        KeyAction::Click { scan_code, virtual_key } => {
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
fn test_modifier_state_merge() {
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
    let fixed_ratio = WindowAction::FixedRatio { ratio: 1.333, scale_index: 0 };
    
    // 验证它们是不同的变体
    assert!(matches!(center, WindowAction::Center));
    assert!(matches!(half_screen, WindowAction::HalfScreen(Edge::Left)));
    assert!(matches!(move_to_edge, WindowAction::MoveToEdge(Edge::Right)));
    assert!(matches!(loop_width, WindowAction::LoopWidth(Alignment::Left)));
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
        button: MouseButton::Left 
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
fn test_action_is_none() {
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
    let move_rel = MouseAction::Move { x: 100, y: 50, relative: true };
    let move_abs = MouseAction::Move { x: 500, y: 300, relative: false };
    let button_down = MouseAction::ButtonDown { button: MouseButton::Left };
    let wheel = MouseAction::Wheel { delta: 120 };
    let h_wheel = MouseAction::HWheel { delta: -120 };
    
    assert!(matches!(move_rel, MouseAction::Move { relative: true, .. }));
    assert!(matches!(move_abs, MouseAction::Move { relative: false, .. }));
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
        Alignment::Center
    ];
    
    assert_eq!(edges.len(), 4);
    assert_eq!(alignments.len(), 5);
}
