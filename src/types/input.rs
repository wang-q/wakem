use super::{now, DeviceType, KeyState, ModifierState, Timestamp};
use serde::{Deserialize, Serialize};

/// Keyboard event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyEvent {
    /// Scan code (hardware related)
    pub scan_code: u16,
    /// Virtual key code (Windows VK_*)
    pub virtual_key: u16,
    /// Key state
    pub state: KeyState,
    /// Modifier key state
    pub modifiers: ModifierState,
    /// Device type
    pub device_type: DeviceType,
    /// Timestamp
    pub timestamp: Timestamp,
    /// Whether from physical device (not simulated input)
    pub is_injected: bool,
}

impl KeyEvent {
    pub fn new(scan_code: u16, virtual_key: u16, state: KeyState) -> Self {
        Self {
            scan_code,
            virtual_key,
            state,
            modifiers: ModifierState::default(),
            device_type: DeviceType::Keyboard,
            timestamp: now(),
            is_injected: false,
        }
    }

    /// Set modifier key state (for building events)
    pub fn with_modifiers(mut self, modifiers: ModifierState) -> Self {
        self.modifiers = modifiers;
        self
    }

    /// Mark as injected event (for simulated input)
    pub fn injected(mut self) -> Self {
        self.is_injected = true;
        self
    }

    /// Check if is modifier key
    pub fn is_modifier(&self) -> bool {
        matches!(
            self.virtual_key,
            0x10 | 0xA0 | 0xA1 | // Shift
            0x11 | 0xA2 | 0xA3 | // Ctrl
            0x12 | 0xA4 | 0xA5 | // Alt
            0x5B | 0x5C // Win
        )
    }

    /// Get modifier key identifier (if is modifier key)
    pub fn modifier_identifier(&self) -> Option<&'static str> {
        match self.virtual_key {
            0x10 | 0xA0 | 0xA1 => Some("Shift"),
            0x11 | 0xA2 | 0xA3 => Some("Control"),
            0x12 | 0xA4 | 0xA5 => Some("Alt"),
            0x5B | 0x5C => Some("Meta"),
            _ => None,
        }
    }
}

/// Mouse button
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    X1, // Side button 1
    X2, // Side button 2
}

/// Mouse event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MouseEvent {
    /// Event type
    pub event_type: MouseEventType,
    /// X coordinate (screen coordinates)
    pub x: i32,
    /// Y coordinate (screen coordinates)
    pub y: i32,
    /// Modifier key state
    pub modifiers: ModifierState,
    /// Timestamp
    pub timestamp: Timestamp,
    /// Whether from physical device
    pub is_injected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MouseEventType {
    /// Mouse move
    Move,
    /// Button press
    ButtonDown(MouseButton),
    /// Button release
    ButtonUp(MouseButton),
    /// Scroll wheel (positive up, negative down)
    Wheel(i32),
    /// Horizontal scroll (positive right, negative left)
    HWheel(i32),
}

impl MouseEvent {
    pub fn new(event_type: MouseEventType, x: i32, y: i32) -> Self {
        Self {
            event_type,
            x,
            y,
            modifiers: ModifierState::default(),
            timestamp: now(),
            is_injected: false,
        }
    }

    /// Set modifier key state (for building events)
    pub fn with_modifiers(mut self, modifiers: ModifierState) -> Self {
        self.modifiers = modifiers;
        self
    }

    /// Mark as injected event (for simulated input)
    pub fn injected(mut self) -> Self {
        self.is_injected = true;
        self
    }

    /// Check if is button press event
    pub fn is_button_down(&self, button: MouseButton) -> bool {
        matches!(&self.event_type, MouseEventType::ButtonDown(b) if *b == button)
    }

    /// Check if is button release event
    pub fn is_button_up(&self, button: MouseButton) -> bool {
        matches!(&self.event_type, MouseEventType::ButtonUp(b) if *b == button)
    }
}

/// Input event (keyboard or mouse)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputEvent {
    Key(KeyEvent),
    Mouse(MouseEvent),
}

impl InputEvent {
    /// Get event timestamp
    pub fn timestamp(&self) -> Timestamp {
        match self {
            InputEvent::Key(e) => e.timestamp,
            InputEvent::Mouse(e) => e.timestamp,
        }
    }

    pub fn is_injected(&self) -> bool {
        match self {
            InputEvent::Key(e) => e.is_injected,
            InputEvent::Mouse(e) => e.is_injected,
        }
    }

    /// Get event type name (for logging)
    pub fn event_type_name(&self) -> &'static str {
        match self {
            InputEvent::Key(_) => "key",
            InputEvent::Mouse(_) => "mouse",
        }
    }
}
