pub mod action;
pub mod input;
pub mod key_codes;
pub mod key_map;
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

// Re-export modifier key constants from key_codes module for backward compatibility
pub use key_codes::{
    VK_ALT, VK_CONTROL, VK_LALT, VK_LCONTROL, VK_LMETA, VK_LSHIFT, VK_RALT, VK_RCONTROL,
    VK_RMETA, VK_RSHIFT, VK_SHIFT,
};
// Note: VK_META is defined in key_codes module but not re-exported here
// to avoid unused import warnings. It can be accessed via `wakem::types::key_codes::VK_META`

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
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
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

    /// Apply modifier state from a virtual key event (sets on press, clears on release)
    pub fn apply_from_virtual_key(&mut self, key: u16, pressed: bool) -> bool {
        match key {
            VK_SHIFT | VK_LSHIFT | VK_RSHIFT => self.shift = pressed,
            VK_CONTROL | VK_LCONTROL | VK_RCONTROL => self.ctrl = pressed,
            VK_ALT | VK_LALT | VK_RALT => self.alt = pressed,
            VK_LMETA | VK_RMETA => self.meta = pressed,
            _ => return false,
        }
        true
    }
}

/// Timestamp (milliseconds)
pub type Timestamp = u64;

/// Get current timestamp in milliseconds since UNIX epoch.
/// Returns 0 if system clock is before UNIX epoch (should never happen on normal systems).
pub fn now() -> Timestamp {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test ModifierState default values (all false)
    #[test]
    fn test_modifier_state_default_values() {
        let modifiers = ModifierState::default();
        assert!(!modifiers.shift);
        assert!(!modifiers.ctrl);
        assert!(!modifiers.alt);
        assert!(!modifiers.meta);
    }

    /// Test ModifierState apply_from_virtual_key (press and release)
    #[test]
    fn test_modifier_state_apply_from_vk() {
        let mut state = ModifierState::default();

        // Press Ctrl
        assert!(state.apply_from_virtual_key(0x11, true));
        assert!(state.ctrl);
        assert!(!state.shift);

        // Press Shift
        assert!(state.apply_from_virtual_key(0x10, true));
        assert!(state.ctrl);
        assert!(state.shift);

        // Release Ctrl
        assert!(state.apply_from_virtual_key(0x11, false));
        assert!(!state.ctrl);
        assert!(state.shift);

        // Release Shift
        assert!(state.apply_from_virtual_key(0x10, false));
        assert!(!state.ctrl);
        assert!(!state.shift);

        // Non-modifier key returns false
        assert!(!state.apply_from_virtual_key(0x41, true));
    }

    /// Test ModifierState new()
    #[test]
    fn test_modifier_state_new() {
        let state = ModifierState::new();
        assert!(!state.shift);
        assert!(!state.ctrl);
        assert!(!state.alt);
        assert!(!state.meta);
    }

    /// Test ModifierState with multiple modifiers
    #[test]
    fn test_modifier_state_multiple() {
        let mut full = ModifierState::new();
        full.ctrl = true;
        full.shift = true;
        full.alt = true;
        full.meta = true;
        assert!(full.ctrl && full.shift && full.alt && full.meta);

        let mut partial = ModifierState::new();
        partial.ctrl = true;
        assert!(partial.ctrl);
    }

    /// Test timestamp function
    #[test]
    fn test_timestamp_alt() {
        let ts1 = now();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let ts2 = now();

        assert!(ts2 >= ts1);
        assert!(ts2 - ts1 >= 10);
    }

    /// Test DeviceType enum
    #[test]
    fn test_device_type_enum_alt() {
        let keyboard = DeviceType::Keyboard;
        let mouse = DeviceType::Mouse;

        assert!(matches!(keyboard, DeviceType::Keyboard));
        assert!(matches!(mouse, DeviceType::Mouse));
    }

    /// Test KeyState enum
    #[test]
    fn test_key_state_enum_alt() {
        let pressed = KeyState::Pressed;
        let released = KeyState::Released;

        assert!(matches!(pressed, KeyState::Pressed));
        assert!(matches!(released, KeyState::Released));
    }
}
