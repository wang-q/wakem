//! Common input device factory and utilities
//!
//! This module provides platform-agnostic input device factory implementations
//! that can be used across all platforms.

use crate::platform::traits::InputDeviceConfig;

/// Input device factory for creating configured input devices
///
/// This is a platform-agnostic factory that works with any platform's
/// input device implementation.
pub struct InputDeviceFactory;

impl InputDeviceFactory {
    /// Create default input device configuration
    pub fn default_config() -> InputDeviceConfig {
        InputDeviceConfig::default()
    }

    /// Create keyboard-only input device configuration
    pub fn keyboard_only_config() -> InputDeviceConfig {
        InputDeviceConfig {
            capture_keyboard: true,
            capture_mouse: false,
            block_legacy_input: true,
        }
    }

    /// Create mouse-only input device configuration
    pub fn mouse_only_config() -> InputDeviceConfig {
        InputDeviceConfig {
            capture_keyboard: false,
            capture_mouse: true,
            block_legacy_input: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = InputDeviceFactory::default_config();
        assert!(config.capture_keyboard);
        assert!(config.capture_mouse);
        assert!(!config.block_legacy_input);
    }

    #[test]
    fn test_keyboard_only_config() {
        let config = InputDeviceFactory::keyboard_only_config();
        assert!(config.capture_keyboard);
        assert!(!config.capture_mouse);
        assert!(config.block_legacy_input);
    }

    #[test]
    fn test_mouse_only_config() {
        let config = InputDeviceFactory::mouse_only_config();
        assert!(!config.capture_keyboard);
        assert!(config.capture_mouse);
        assert!(config.block_legacy_input);
    }
}
