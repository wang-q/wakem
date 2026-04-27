use std::fmt;
use std::num::NonZeroU16;

/// Scan code (hardware-related keyboard identifier)
///
/// Using newtype wrapper to ensure type safety:
/// - Distinguish between "has value" and "no value" (0 means invalid)
/// - Prevent confusion with other u16 types
/// - Provide semantic construction methods
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanCode(NonZeroU16);

impl ScanCode {
    /// Create a new scan code
    ///
    /// # Parameters
    /// * `code` - Scan code value (must be > 0)
    ///
    /// # Returns
    /// * `Some(ScanCode)` - If code > 0
    /// * `None` - If code == 0 (invalid)
    #[allow(dead_code)]
    pub fn new(code: u16) -> Option<Self> {
        NonZeroU16::new(code).map(ScanCode)
    }

    /// Create scan code, 0 value is treated as None
    ///
    /// This is the most commonly used construction method for handling inputs that may be 0
    #[allow(dead_code)]
    pub fn from_option(code: u16) -> Option<Self> {
        Self::new(code)
    }

    /// Get raw value
    #[allow(dead_code)]
    pub fn value(&self) -> u16 {
        self.0.get()
    }

    /// Check if valid (always returns true since 0 cannot be created)
    #[allow(dead_code)]
    pub fn is_valid(&self) -> bool {
        true // NonZeroU16 guarantees non-zero
    }
}

impl fmt::Display for ScanCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:04X}", self.value())
    }
}

/// Virtual key code (Windows VK_* identifier)
///
/// Characteristics:
/// - 0 means invalid/not specified
/// - Non-zero values represent valid virtual key codes
/// - Provides constant definitions for common keys
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VirtualKey(u16);

// === Modifier Key Scan Codes ===

/// Scan code for Ctrl key
pub const SCAN_CODE_CTRL: u16 = 0x1D;
/// Scan code for Shift key
pub const SCAN_CODE_SHIFT: u16 = 0x2A;
/// Scan code for Alt key
pub const SCAN_CODE_ALT: u16 = 0x38;
/// Scan code for Meta/Win key
pub const SCAN_CODE_META: u16 = 0x5B;

impl VirtualKey {
    /// Create virtual key code
    ///
    /// # Parameters
    /// * `key` - Virtual key code value (0 means invalid)
    pub fn new(key: u16) -> Self {
        VirtualKey(key)
    }

    /// Get raw value
    pub fn value(&self) -> u16 {
        self.0
    }

    /// Check if valid (non-zero)
    pub fn is_valid(&self) -> bool {
        self.0 != 0
    }

    /// Check if modifier key
    pub fn is_modifier(&self) -> bool {
        matches!(
            self.0,
            0x10 | 0xA0 | 0xA1 | // Shift
            0x11 | 0xA2 | 0xA3 | // Ctrl
            0x12 | 0xA4 | 0xA5 | // Alt
            0x5B | 0x5C // Win
        )
    }

    /// Get modifier key name (if it's a modifier key)
    pub fn modifier_name(&self) -> Option<&'static str> {
        match self.0 {
            0x10 | 0xA0 | 0xA1 => Some("Shift"),
            0x11 | 0xA2 | 0xA3 => Some("Control"),
            0x12 | 0xA4 | 0xA5 => Some("Alt"),
            0x5B | 0x5C => Some("Meta"),
            _ => None,
        }
    }

    // === Common Key Constants ===

    /// Backspace (VK_BACK = 0x08)
    pub const BACKSPACE: Self = VirtualKey(0x08);
    /// Tab (VK_TAB = 0x09)
    pub const TAB: Self = VirtualKey(0x09);
    /// Enter (VK_RETURN = 0x0D)
    pub const ENTER: Self = VirtualKey(0x0D);
    /// Escape (VK_ESCAPE = 0x1B)
    pub const ESCAPE: Self = VirtualKey(0x1B);
    /// Space (VK_SPACE = 0x20)
    pub const SPACE: Self = VirtualKey(0x20);
    /// CapsLock (VK_CAPITAL = 0x14)
    pub const CAPSLOCK: Self = VirtualKey(0x14);

    // === Letter Keys ===
    pub const A: Self = VirtualKey(0x41);
    pub const B: Self = VirtualKey(0x42);
    pub const C: Self = VirtualKey(0x43);
    pub const D: Self = VirtualKey(0x44);
    pub const E: Self = VirtualKey(0x45);
    pub const F: Self = VirtualKey(0x46);
    pub const G: Self = VirtualKey(0x47);
    pub const H: Self = VirtualKey(0x48);
    pub const I: Self = VirtualKey(0x49);
    pub const J: Self = VirtualKey(0x4A);
    pub const K: Self = VirtualKey(0x4B);
    pub const L: Self = VirtualKey(0x4C);
    pub const M: Self = VirtualKey(0x4D);
    pub const N: Self = VirtualKey(0x4E);
    pub const O: Self = VirtualKey(0x4F);
    pub const P: Self = VirtualKey(0x50);
    pub const Q: Self = VirtualKey(0x51);
    pub const R: Self = VirtualKey(0x52);
    pub const S: Self = VirtualKey(0x53);
    pub const T: Self = VirtualKey(0x54);
    pub const U: Self = VirtualKey(0x55);
    pub const V: Self = VirtualKey(0x56);
    pub const W: Self = VirtualKey(0x57);
    pub const X: Self = VirtualKey(0x58);
    pub const Y: Self = VirtualKey(0x59);
    pub const Z: Self = VirtualKey(0x5A);

    // === Number Keys ===
    pub const KEY_0: Self = VirtualKey(0x30);
    pub const KEY_1: Self = VirtualKey(0x31);
    pub const KEY_2: Self = VirtualKey(0x32);
    pub const KEY_3: Self = VirtualKey(0x33);
    pub const KEY_4: Self = VirtualKey(0x34);
    pub const KEY_5: Self = VirtualKey(0x35);
    pub const KEY_6: Self = VirtualKey(0x36);
    pub const KEY_7: Self = VirtualKey(0x37);
    pub const KEY_8: Self = VirtualKey(0x38);
    pub const KEY_9: Self = VirtualKey(0x39);

    // === Function Keys ===
    pub const F1: Self = VirtualKey(0x70);
    pub const F2: Self = VirtualKey(0x71);
    pub const F3: Self = VirtualKey(0x72);
    pub const F4: Self = VirtualKey(0x73);
    pub const F5: Self = VirtualKey(0x74);
    pub const F6: Self = VirtualKey(0x75);
    pub const F7: Self = VirtualKey(0x76);
    pub const F8: Self = VirtualKey(0x77);
    pub const F9: Self = VirtualKey(0x78);
    pub const F10: Self = VirtualKey(0x79);
    pub const F11: Self = VirtualKey(0x7A);
    pub const F12: Self = VirtualKey(0x7B);

    // === Modifier Keys ===
    pub const SHIFT: Self = VirtualKey(0x10);
    pub const CONTROL: Self = VirtualKey(0x11);
    pub const ALT: Self = VirtualKey(0x12);
    pub const META: Self = VirtualKey(0x5B);

    // === Arrow Keys ===
    pub const LEFT: Self = VirtualKey(0x25);
    pub const UP: Self = VirtualKey(0x26);
    pub const RIGHT: Self = VirtualKey(0x27);
    pub const DOWN: Self = VirtualKey(0x28);
}

impl fmt::Display for VirtualKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(name) = self.modifier_name() {
            write!(f, "{}", name)
        } else if self.is_valid() {
            write!(f, "VK_0x{:02X}", self.value())
        } else {
            write!(f, "Invalid")
        }
    }
}

impl From<u16> for VirtualKey {
    fn from(value: u16) -> Self {
        VirtualKey::new(value)
    }
}

impl From<VirtualKey> for u16 {
    fn from(vk: VirtualKey) -> u16 {
        vk.value()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_code_creation() {
        assert!(ScanCode::new(0).is_none());
        assert!(ScanCode::new(1).is_some());
        assert_eq!(ScanCode::new(0x3A).unwrap().value(), 0x3A);
    }

    #[test]
    fn test_scan_code_display() {
        let sc = ScanCode::new(0x3A).unwrap();
        assert_eq!(format!("{}", sc), "0x003A");
    }

    #[test]
    fn test_virtual_key_validity() {
        assert!(!VirtualKey::new(0).is_valid());
        assert!(VirtualKey::new(0x41).is_valid());
    }

    #[test]
    fn test_virtual_key_modifiers() {
        assert!(VirtualKey::SHIFT.is_modifier());
        assert!(VirtualKey::CONTROL.is_modifier());
        assert!(VirtualKey::ALT.is_modifier());
        assert!(!VirtualKey::A.is_modifier());
    }

    #[test]
    fn test_virtual_key_constants() {
        assert_eq!(VirtualKey::A.value(), 0x41);
        assert_eq!(VirtualKey::ENTER.value(), 0x0D);
        assert_eq!(VirtualKey::F1.value(), 0x70);
    }

    #[test]
    fn test_conversion() {
        let vk = VirtualKey::from(0x42u16);
        let value: u16 = vk.into();
        assert_eq!(value, 0x42);
    }
}
