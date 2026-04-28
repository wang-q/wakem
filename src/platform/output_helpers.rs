//! Platform-agnostic output device helper functions
//!
//! Contains shared logic used by all platform output device implementations,
//! including cross-platform modifier key virtual key codes.

/// Cross-platform modifier key virtual key codes.
///
/// These constants use Windows virtual key code values as the canonical
/// representation. On macOS, [MacosOutputDevice] converts them to
/// CGKeyCode values via [virtual_key_to_keycode].
///
/// | Constant  | Windows VK     | macOS Keycode |
/// |-----------|----------------|---------------|
/// | SHIFT     | VK_SHIFT (0x10)| 56            |
/// | CONTROL   | VK_CONTROL (0x11)| 59          |
/// | ALT       | VK_MENU (0x12) | 58            |
/// | META/WIN  | VK_LWIN (0x5B) | 55            |
pub mod modifier_vk {
    pub const SHIFT: u16 = 0x10;
    pub const CONTROL: u16 = 0x11;
    pub const ALT: u16 = 0x12;
    pub const META: u16 = 0x5B;
}

/// Convert ASCII character to virtual key code
///
/// Supports basic US QWERTY layout characters:
/// - Letters: a-z, A-Z
/// - Digits: 0-9
/// - Space, Tab, Enter
///
/// # Limitations
///
/// This function only supports basic ASCII characters. Unicode characters,
/// special symbols (like `@`, `#`, `$`), and non-US keyboard layouts are
/// not supported and will return `None`.
///
/// For full keyboard layout support, consider using a dedicated keyboard
/// layout library like `keyboard-types` or platform-specific APIs.
pub fn char_to_vk(ch: char) -> Option<u16> {
    match ch {
        'a'..='z' => Some(ch as u16 - 'a' as u16 + 0x41),
        'A'..='Z' => Some(ch as u16 - 'A' as u16 + 0x41),
        '0'..='9' => Some(ch as u16 - '0' as u16 + 0x30),
        ' ' => Some(0x20),
        '\t' => Some(0x09),
        '\r' | '\n' => Some(0x0D),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_char_to_vk_lowercase() {
        assert_eq!(char_to_vk('a'), Some(0x41));
        assert_eq!(char_to_vk('b'), Some(0x42));
        assert_eq!(char_to_vk('m'), Some(0x4D));
        assert_eq!(char_to_vk('z'), Some(0x5A));
    }

    #[test]
    fn test_char_to_vk_uppercase() {
        assert_eq!(char_to_vk('A'), Some(0x41));
        assert_eq!(char_to_vk('Z'), Some(0x5A));
    }

    #[test]
    fn test_char_to_vk_digits() {
        assert_eq!(char_to_vk('0'), Some(0x30));
        assert_eq!(char_to_vk('5'), Some(0x35));
        assert_eq!(char_to_vk('9'), Some(0x39));
    }

    #[test]
    fn test_char_to_vk_special() {
        assert_eq!(char_to_vk(' '), Some(0x20));
        assert_eq!(char_to_vk('\t'), Some(0x09));
        assert_eq!(char_to_vk('\n'), Some(0x0D));
        assert_eq!(char_to_vk('\r'), Some(0x0D));
    }

    #[test]
    fn test_char_to_vk_unsupported() {
        assert_eq!(char_to_vk('@'), None);
        assert_eq!(char_to_vk('#'), None);
        assert_eq!(char_to_vk('中'), None);
        assert_eq!(char_to_vk('é'), None);
    }
}
