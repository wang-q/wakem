pub mod action;
pub mod input;
pub mod key_codes;
pub mod layer;
pub mod macros;
pub mod mapping;

pub use action::*;
pub use input::*;
pub use key_codes::*;
pub use layer::*;
pub use macros::*;
pub use mapping::*;

use serde::{Deserialize, Serialize};

/// Device type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceType {
    Keyboard,
    Mouse,
}

/// Key state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyState {
    Pressed,
    Released,
}

/// Modifier key state
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct ModifierState {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool, // Windows key / Command key
}

impl ModifierState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if no modifier keys are pressed
    pub fn is_empty(&self) -> bool {
        !self.shift && !self.ctrl && !self.alt && !self.meta
    }

    /// Create modifier key state from virtual key code
    pub fn from_virtual_key(key: u16, pressed: bool) -> Option<(Self, bool)> {
        let mut state = Self::new();
        match key {
            0x10 | 0xA0 | 0xA1 => state.shift = pressed, // VK_SHIFT, VK_LSHIFT, VK_RSHIFT
            0x11 | 0xA2 | 0xA3 => state.ctrl = pressed, // VK_CONTROL, VK_LCONTROL, VK_RCONTROL
            0x12 | 0xA4 | 0xA5 => state.alt = pressed,  // VK_MENU, VK_LMENU, VK_RMENU
            0x5B | 0x5C => state.meta = pressed,        // VK_LWIN, VK_RWIN
            _ => return None,
        }
        Some((state, pressed))
    }

    /// Merge another modifier key state
    pub fn merge(&mut self, other: &ModifierState) {
        self.shift |= other.shift;
        self.ctrl |= other.ctrl;
        self.alt |= other.alt;
        self.meta |= other.meta;
    }
}

/// Timestamp (milliseconds)
pub type Timestamp = u64;

/// Get current timestamp
pub fn now() -> Timestamp {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
