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
    /// Set transparency
    SetOpacity { opacity: u8 },
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

/// System control action
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SystemAction {
    /// Increase volume
    VolumeUp,
    /// Decrease volume
    VolumeDown,
    /// Toggle mute
    VolumeMute,
    /// Increase brightness
    BrightnessUp,
    /// Decrease brightness
    BrightnessDown,
}

/// All possible action types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Action {
    Key(KeyAction),
    Mouse(MouseAction),
    Window(WindowAction),
    Launch(LaunchAction),
    System(SystemAction),
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

    /// Check if no operation
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
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
