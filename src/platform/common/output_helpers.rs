//! Output device helper functions for internal key code mapping
//!
//! Contains shared logic used by the [OutputDevice](crate::platform::traits::OutputDevice)
//! trait's default implementations.
//!
//! ## Internal VK Convention
//!
//! All functions in this module return **Windows virtual key codes** as the
//! universal internal key identifier. This is a deliberate design choice:
//! the system uses Windows VK codes as a common key representation regardless
//! of platform. Platform-specific output devices convert these to native codes
//! at send time (e.g., macOS CGKeyCode via `virtual_key_to_keycode()`).

/// Convert ASCII character to internal virtual key code
///
/// Returns internal VK values as per the internal VK convention.
/// macOS output devices convert these via `virtual_key_to_keycode()`.
///
/// Supports basic US QWERTY layout characters:
/// - Letters: a-z, A-Z
/// - Digits: 0-9
/// - Space, Tab, Enter
pub fn char_to_internal_vk(ch: char) -> Option<u16> {
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
    fn test_char_to_internal_vk_lowercase() {
        assert_eq!(char_to_internal_vk('a'), Some(0x41));
        assert_eq!(char_to_internal_vk('b'), Some(0x42));
        assert_eq!(char_to_internal_vk('m'), Some(0x4D));
        assert_eq!(char_to_internal_vk('z'), Some(0x5A));
    }

    #[test]
    fn test_char_to_internal_vk_uppercase() {
        assert_eq!(char_to_internal_vk('A'), Some(0x41));
        assert_eq!(char_to_internal_vk('Z'), Some(0x5A));
    }

    #[test]
    fn test_char_to_internal_vk_digits() {
        assert_eq!(char_to_internal_vk('0'), Some(0x30));
        assert_eq!(char_to_internal_vk('5'), Some(0x35));
        assert_eq!(char_to_internal_vk('9'), Some(0x39));
    }

    #[test]
    fn test_char_to_internal_vk_special() {
        assert_eq!(char_to_internal_vk(' '), Some(0x20));
        assert_eq!(char_to_internal_vk('\t'), Some(0x09));
        assert_eq!(char_to_internal_vk('\n'), Some(0x0D));
        assert_eq!(char_to_internal_vk('\r'), Some(0x0D));
    }

    #[test]
    fn test_char_to_internal_vk_unsupported() {
        assert_eq!(char_to_internal_vk('@'), None);
        assert_eq!(char_to_internal_vk('#'), None);
        assert_eq!(char_to_internal_vk('中'), None);
        assert_eq!(char_to_internal_vk('é'), None);
    }
}
