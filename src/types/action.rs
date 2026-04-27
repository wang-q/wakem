use serde::{Deserialize, Serialize};

/// Key action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KeyAction {
    /// Press key
    Press { scan_code: u16, virtual_key: u16 },
    /// Release key
    Release { scan_code: u16, virtual_key: u16 },
    /// Click key (press and release)
    Click { scan_code: u16, virtual_key: u16 },
    /// Input text
    TypeText(String),
    /// Key combination (e.g., Ctrl+C)
    Combo {
        modifiers: super::ModifierState,
        key: (u16, u16), // (scan_code, virtual_key)
    },
    /// No operation
    None,
}

impl KeyAction {
    /// Create corresponding Press action from KeyEvent
    pub fn press_from_event(event: &super::KeyEvent) -> Self {
        Self::Press {
            scan_code: event.scan_code,
            virtual_key: event.virtual_key,
        }
    }

    /// Create corresponding Release action from KeyEvent
    pub fn release_from_event(event: &super::KeyEvent) -> Self {
        Self::Release {
            scan_code: event.scan_code,
            virtual_key: event.virtual_key,
        }
    }

    /// Create click action
    pub fn click(scan_code: u16, virtual_key: u16) -> Self {
        Self::Click {
            scan_code,
            virtual_key,
        }
    }

    /// Create press action
    pub fn press(scan_code: u16, virtual_key: u16) -> Self {
        Self::Press {
            scan_code,
            virtual_key,
        }
    }

    /// Create release action
    pub fn release(scan_code: u16, virtual_key: u16) -> Self {
        Self::Release {
            scan_code,
            virtual_key,
        }
    }

    /// Create key combination action
    pub fn combo(
        modifiers: super::ModifierState,
        scan_code: u16,
        virtual_key: u16,
    ) -> Self {
        Self::Combo {
            modifiers,
            key: (scan_code, virtual_key),
        }
    }
}

/// Mouse action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MouseAction {
    /// Move mouse
    Move { x: i32, y: i32, relative: bool },
    /// Press button
    ButtonDown { button: super::MouseButton },
    /// Release button
    ButtonUp { button: super::MouseButton },
    /// Click button
    ButtonClick { button: super::MouseButton },
    /// Scroll wheel
    Wheel { delta: i32 },
    /// Horizontal scroll
    HWheel { delta: i32 },
    /// No operation
    None,
}

/// Edge enum (for window management)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Edge {
    Left,
    Right,
    Top,
    Bottom,
}

/// Alignment enum
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Alignment {
    Left,
    Right,
    Top,
    Bottom,
    Center,
}

/// Monitor direction enum
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum MonitorDirection {
    Next,
    Prev,
    Index(i32),
}

/// Window action (inspired by mrw project)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WindowAction {
    /// Center window
    Center,
    /// Move to screen edge
    MoveToEdge(Edge),
    /// Half screen display
    HalfScreen(Edge),
    /// Cycle width adjustment
    LoopWidth(Alignment),
    /// Cycle height adjustment
    LoopHeight(Alignment),
    /// Fixed ratio window (ratio value, scale index)
    FixedRatio { ratio: f32, scale_index: usize },
    /// Native ratio window (based on screen ratio, scale index)
    NativeRatio { scale_index: usize },
    /// Same process window switch (Alt+` feature)
    SwitchToNextWindow,
    /// Move across monitors
    MoveToMonitor(MonitorDirection),
    /// Move window (absolute coordinates)
    Move { x: i32, y: i32 },
    /// Resize window
    Resize { width: i32, height: i32 },
    /// Minimize window
    Minimize,
    /// Maximize window
    Maximize,
    /// Restore window
    Restore,
    /// Close window
    Close,
    /// Toggle always on top
    ToggleTopmost,
    /// Show debug info (Hyper+W)
    ShowDebugInfo,
    /// Show notification (Hyper+Shift+W)
    ShowNotification { title: String, message: String },
    /// Save current window as preset
    SavePreset { name: String },
    /// Load specified preset to current window
    LoadPreset { name: String },
    /// Apply matching preset to current window
    ApplyPreset,
    /// No operation
    None,
}

/// Launch program action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchAction {
    pub program: String,
    pub args: Vec<String>,
    pub working_dir: Option<String>,
    pub env_vars: Vec<(String, String)>,
}

/// All possible action types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Action {
    Key(KeyAction),
    Mouse(MouseAction),
    Window(WindowAction),
    Launch(LaunchAction),
    /// Execute multiple actions
    Sequence(Vec<Action>),
    /// Delay (for macro playback)
    Delay {
        milliseconds: u64,
    },
    /// No operation
    None,
}

impl Action {
    /// Create key action
    pub fn key(action: KeyAction) -> Self {
        Self::Key(action)
    }

    /// Create mouse action
    pub fn mouse(action: MouseAction) -> Self {
        Self::Mouse(action)
    }

    /// Create window action
    pub fn window(action: WindowAction) -> Self {
        Self::Window(action)
    }

    /// Create launch program action
    pub fn launch(program: impl Into<String>) -> Self {
        Self::Launch(LaunchAction {
            program: program.into(),
            args: Vec::new(),
            working_dir: None,
            env_vars: Vec::new(),
        })
    }

    /// Create action sequence
    pub fn sequence(actions: Vec<Action>) -> Self {
        Self::Sequence(actions)
    }

    /// Create delay action
    pub fn delay(milliseconds: u64) -> Self {
        Self::Delay { milliseconds }
    }

    /// Create corresponding Action from input event
    pub fn from_input_event(event: &super::InputEvent) -> Option<Self> {
        match event {
            super::InputEvent::Key(key_event) => {
                let key_action = if key_event.state == super::KeyState::Pressed {
                    KeyAction::press_from_event(key_event)
                } else {
                    KeyAction::release_from_event(key_event)
                };
                Some(Self::Key(key_action))
            }
            super::InputEvent::Mouse(mouse_event) => {
                let mouse_action = match mouse_event.event_type {
                    super::MouseEventType::ButtonDown(button) => {
                        MouseAction::ButtonDown { button }
                    }
                    super::MouseEventType::ButtonUp(button) => {
                        MouseAction::ButtonUp { button }
                    }
                    super::MouseEventType::Move => MouseAction::Move {
                        x: mouse_event.x,
                        y: mouse_event.y,
                        relative: false,
                    },
                    super::MouseEventType::Wheel(delta) => MouseAction::Wheel { delta },
                    super::MouseEventType::HWheel(delta) => {
                        MouseAction::HWheel { delta }
                    }
                };
                Some(Self::Mouse(mouse_action))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== KeyAction Tests ====================

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
        let mut modifiers = super::super::ModifierState::default();
        modifiers.ctrl = true;
        let action = KeyAction::combo(modifiers, 0x1E, 0x41);
        if let KeyAction::Combo { modifiers: m, key } = action {
            assert!(m.ctrl);
            assert_eq!(key, (0x1E, 0x41));
        }
    }

    #[test]
    fn test_key_action_from_event() {
        let event =
            super::super::KeyEvent::new(0x1E, 0x41, super::super::KeyState::Pressed);
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

    // ==================== MouseAction Tests ====================

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
            button: super::super::MouseButton::Left,
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
                button: super::super::MouseButton::Left,
            },
            MouseAction::ButtonClick {
                button: super::super::MouseButton::Right,
            },
            MouseAction::ButtonDown {
                button: super::super::MouseButton::Left,
            },
            MouseAction::ButtonUp {
                button: super::super::MouseButton::Left,
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

    // ==================== WindowAction Tests ====================

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

    #[test]
    fn test_window_action_variants_alt() {
        let center = WindowAction::Center;
        let half_screen = WindowAction::HalfScreen(Edge::Left);
        let move_to_edge = WindowAction::MoveToEdge(Edge::Right);
        let loop_width = WindowAction::LoopWidth(Alignment::Left);
        let fixed_ratio = WindowAction::FixedRatio {
            ratio: 1.333,
            scale_index: 0,
        };

        // Verify they are different variants
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

    // ==================== Edge and Alignment Tests ====================

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

    // ==================== MonitorDirection Tests ====================

    #[test]
    fn test_monitor_direction() {
        let next = MonitorDirection::Next;
        let prev = MonitorDirection::Prev;
        let index = MonitorDirection::Index(2);

        assert!(matches!(next, MonitorDirection::Next));
        assert!(matches!(prev, MonitorDirection::Prev));
        assert!(matches!(index, MonitorDirection::Index(2)));
    }

    // ==================== LaunchAction Tests ====================

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

    // ==================== Action Wrapper Tests ====================

    #[test]
    fn test_action_wrapper() {
        let key_action = Action::key(KeyAction::click(0x1E, 0x41));
        let mouse_action = Action::mouse(MouseAction::ButtonClick {
            button: super::super::MouseButton::Left,
        });
        let window_action = Action::window(WindowAction::Center);
        let launch_action = Action::launch("notepad.exe");

        assert!(matches!(key_action, Action::Key(_)));
        assert!(matches!(mouse_action, Action::Mouse(_)));
        assert!(matches!(window_action, Action::Window(_)));
        assert!(matches!(launch_action, Action::Launch(_)));
    }

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

    // ==================== Action from_input_event Tests ====================

    #[test]
    fn test_action_from_input_event_key_pressed() {
        let key_event =
            super::super::KeyEvent::new(0x1E, 0x41, super::super::KeyState::Pressed);
        let event = super::super::InputEvent::Key(key_event);

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

    #[test]
    fn test_action_from_input_event_key_released() {
        let key_event =
            super::super::KeyEvent::new(0x1E, 0x41, super::super::KeyState::Released);
        let event = super::super::InputEvent::Key(key_event);

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

    #[test]
    fn test_action_from_input_event_mouse_move() {
        let mouse_event =
            super::super::MouseEvent::new(super::super::MouseEventType::Move, 100, 200);
        let event = super::super::InputEvent::Mouse(mouse_event);

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

    #[test]
    fn test_action_from_input_event_mouse_button_down() {
        let mouse_event = super::super::MouseEvent::new(
            super::super::MouseEventType::ButtonDown(super::super::MouseButton::Left),
            0,
            0,
        );
        let event = super::super::InputEvent::Mouse(mouse_event);

        let action = Action::from_input_event(&event);
        assert!(action.is_some());

        if let Some(Action::Mouse(MouseAction::ButtonDown { button })) = action {
            assert_eq!(button, super::super::MouseButton::Left);
        } else {
            panic!("Expected Mouse ButtonDown action");
        }
    }

    #[test]
    fn test_action_from_input_event_mouse_button_up() {
        let mouse_event = super::super::MouseEvent::new(
            super::super::MouseEventType::ButtonUp(super::super::MouseButton::Right),
            0,
            0,
        );
        let event = super::super::InputEvent::Mouse(mouse_event);

        let action = Action::from_input_event(&event);
        assert!(action.is_some());

        if let Some(Action::Mouse(MouseAction::ButtonUp { button })) = action {
            assert_eq!(button, super::super::MouseButton::Right);
        } else {
            panic!("Expected Mouse ButtonUp action");
        }
    }

    #[test]
    fn test_action_from_input_event_mouse_wheel() {
        let mouse_event = super::super::MouseEvent::new(
            super::super::MouseEventType::Wheel(120),
            0,
            0,
        );
        let event = super::super::InputEvent::Mouse(mouse_event);

        let action = Action::from_input_event(&event);
        assert!(action.is_some());

        if let Some(Action::Mouse(MouseAction::Wheel { delta })) = action {
            assert_eq!(delta, 120);
        } else {
            panic!("Expected Mouse Wheel action");
        }
    }
}
