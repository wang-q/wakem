pub mod action;
pub mod context;
pub mod input;
pub mod key_codes;
pub mod layer;
pub mod macros;
pub mod mapping;

pub use action::*;
pub use context::*;
pub use input::*;
#[allow(unused_imports)]
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

    /// Check if no modifier keys are pressed
    pub fn is_empty(&self) -> bool {
        !self.shift && !self.ctrl && !self.alt && !self.meta
    }

    /// Create modifier key state from virtual key code
    ///
    /// Returns `Some((modifier_state, pressed))` if the key is a modifier,
    /// where `modifier_state` has only the relevant modifier bit set,
    /// and `pressed` echoes back the input parameter for convenience.
    /// Returns `None` for non-modifier keys.
    pub fn from_virtual_key(key: u16, pressed: bool) -> Option<(Self, bool)> {
        let mut state = Self::new();
        match key {
            0x10 | 0xA0 | 0xA1 => state.shift = pressed,
            0x11 | 0xA2 | 0xA3 => state.ctrl = pressed,
            0x12 | 0xA4 | 0xA5 => state.alt = pressed,
            0x5B | 0x5C => state.meta = pressed,
            _ => return None,
        }
        Some((state, pressed))
    }

    /// Check if a virtual key code is a modifier key and return which modifier
    ///
    /// Returns `Some(Self)` with the relevant modifier bit set to true,
    /// or `None` for non-modifier keys.
    pub fn from_virtual_key_only(key: u16) -> Option<Self> {
        let mut state = Self::new();
        match key {
            0x10 | 0xA0 | 0xA1 => state.shift = true,
            0x11 | 0xA2 | 0xA3 => state.ctrl = true,
            0x12 | 0xA4 | 0xA5 => state.alt = true,
            0x5B | 0x5C => state.meta = true,
            _ => return None,
        }
        Some(state)
    }

    /// Merge another modifier key state (OR logic, only sets bits)
    pub fn merge(&mut self, other: &ModifierState) {
        self.shift |= other.shift;
        self.ctrl |= other.ctrl;
        self.alt |= other.alt;
        self.meta |= other.meta;
    }

    /// Apply modifier state from a virtual key event (sets on press, clears on release)
    pub fn apply_from_virtual_key(&mut self, key: u16, pressed: bool) -> bool {
        match key {
            0x10 | 0xA0 | 0xA1 => self.shift = pressed,
            0x11 | 0xA2 | 0xA3 => self.ctrl = pressed,
            0x12 | 0xA4 | 0xA5 => self.alt = pressed,
            0x5B | 0x5C => self.meta = pressed,
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

    /// Test ModifierState::is_empty()
    #[test]
    fn test_modifier_state_is_empty_alt() {
        let mut modifiers = ModifierState::default();
        assert!(modifiers.is_empty());

        // After setting a modifier, it's no longer empty
        modifiers.ctrl = true;
        assert!(!modifiers.is_empty());
    }

    /// Test ModifierState from virtual key - Shift
    #[test]
    fn test_modifier_state_from_vk_shift_alt() {
        let (state, pressed) = ModifierState::from_virtual_key(0x10, true).unwrap();
        assert!(state.shift);
        assert!(pressed);

        let (state, _) = ModifierState::from_virtual_key(0xA0, true).unwrap();
        assert!(state.shift); // LSHIFT

        let (state, _) = ModifierState::from_virtual_key(0xA1, true).unwrap();
        assert!(state.shift); // RSHIFT

        // Release state
        let (_, pressed) = ModifierState::from_virtual_key(0x10, false).unwrap();
        assert!(!pressed);
    }

    /// Test ModifierState from virtual key - Control
    #[test]
    fn test_modifier_state_from_vk_ctrl_alt() {
        let (state, _) = ModifierState::from_virtual_key(0x11, true).unwrap();
        assert!(state.ctrl);

        let (state, _) = ModifierState::from_virtual_key(0xA2, true).unwrap();
        assert!(state.ctrl); // LCONTROL

        let (state, _) = ModifierState::from_virtual_key(0xA3, true).unwrap();
        assert!(state.ctrl); // RCONTROL
    }

    /// Test ModifierState from virtual key - Alt
    #[test]
    fn test_modifier_state_from_vk_alt_alt() {
        let (state, _) = ModifierState::from_virtual_key(0x12, true).unwrap();
        assert!(state.alt);

        let (state, _) = ModifierState::from_virtual_key(0xA4, true).unwrap();
        assert!(state.alt); // LMENU

        let (state, _) = ModifierState::from_virtual_key(0xA5, true).unwrap();
        assert!(state.alt); // RMENU
    }

    /// Test ModifierState from virtual key - Meta/Win
    #[test]
    fn test_modifier_state_from_vk_meta_alt() {
        let (state, _) = ModifierState::from_virtual_key(0x5B, true).unwrap();
        assert!(state.meta); // LWIN

        let (state, _) = ModifierState::from_virtual_key(0x5C, true).unwrap();
        assert!(state.meta); // RWIN
    }

    /// Test unknown virtual key returns None
    #[test]
    fn test_modifier_state_from_vk_unknown_alt() {
        let result = ModifierState::from_virtual_key(0x41, true); // 'A' key is not a modifier
        assert!(result.is_none());
    }

    /// Test ModifierState merge
    #[test]
    fn test_modifier_state_merge_alt() {
        let mut state1 = ModifierState::default();
        state1.ctrl = true;

        let mut state2 = ModifierState::default();
        state2.shift = true;
        state2.alt = true;

        state1.merge(&state2);

        assert!(state1.ctrl);
        assert!(state1.shift);
        assert!(state1.alt);
        assert!(!state1.meta);
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
        assert!(state.is_empty());
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
        assert!(!full.is_empty());

        let mut partial = ModifierState::new();
        partial.ctrl = true;
        assert!(!partial.is_empty());
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
