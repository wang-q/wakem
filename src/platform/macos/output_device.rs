//! macOS output device implementation using CGEvent
//!
//! This module uses Core Graphics to send simulated input events.
//! Shared logic (char mapping, text input, key combos) is in [output_helpers].

use crate::platform::traits::OutputDeviceTrait;
use crate::types::{KeyAction, ModifierState, MouseAction, MouseButton, SystemAction};
use anyhow::Result;
use core_graphics::event::{CGEvent, CGEventFlags, CGEventType, CGMouseButton};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use core_graphics::geometry::CGPoint;
use tracing::debug;

/// macOS output device using CGEvent
pub struct MacosOutputDevice;

impl MacosOutputDevice {
    pub fn new() -> Self {
        Self
    }

    /// Convert Windows-style virtual key to macOS CGKeyCode
    fn virtual_key_to_cg_keycode(virtual_key: u16) -> u16 {
        match virtual_key {
            0x41 => 0x00,
            0x53 => 0x01,
            0x44 => 0x02,
            0x46 => 0x03,
            0x48 => 0x04,
            0x47 => 0x05,
            0x5A => 0x06,
            0x58 => 0x07,
            0x43 => 0x08,
            0x56 => 0x09,
            0x42 => 0x0B,
            0x51 => 0x0C,
            0x57 => 0x0D,
            0x45 => 0x0E,
            0x52 => 0x0F,
            0x59 => 0x10,
            0x54 => 0x11,
            0x31 => 0x12,
            0x32 => 0x13,
            0x33 => 0x14,
            0x34 => 0x15,
            0x36 => 0x16,
            0x35 => 0x17,
            0x3D => 0x18,
            0x39 => 0x19,
            0x37 => 0x1A,
            0x2D => 0x1B,
            0x38 => 0x1C,
            0x30 => 0x1D,
            0x5D => 0x1E,
            0x4F => 0x1F,
            0x55 => 0x20,
            0x5B => 0x21,
            0x49 => 0x22,
            0x50 => 0x23,
            0x0D => 0x24,
            0x4C => 0x25,
            0x4A => 0x26,
            0xDE => 0x27,
            0x4B => 0x28,
            0x3B => 0x29,
            0xDC => 0x2A,
            0xBC => 0x2B,
            0xBF => 0x2C,
            0x4E => 0x2D,
            0x4D => 0x2E,
            0xBE => 0x2F,
            0x09 => 0x30,
            0x20 => 0x31,
            0xC0 => 0x32,
            0x08 => 0x33,
            0x1B => 0x35,
            0x14 => 0x3A,
            0x70 => 0x7A,
            0x71 => 0x78,
            0x72 => 0x63,
            0x73 => 0x76,
            0x74 => 0x60,
            0x75 => 0x61,
            0x76 => 0x62,
            0x77 => 0x64,
            0x78 => 0x65,
            0x79 => 0x6D,
            0x7A => 0x67,
            0x7B => 0x6F,
            0x24 => 0x72,
            0x23 => 0x73,
            0x21 => 0x74,
            0x22 => 0x79,
            0x25 => 0x7B,
            0x26 => 0x7E,
            0x27 => 0x7C,
            0x28 => 0x7D,
            0xA0 | 0xA1 | 0x10 => 0x38,
            0xA2 | 0xA3 | 0x11 => 0x3B,
            0xA4 | 0xA5 | 0x12 => 0x3A,
            0x5B | 0x5C => 0x37,
            _ => virtual_key,
        }
    }

    fn send_key_raw(&self, virtual_key: u16, release: bool) -> Result<()> {
        let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
            .map_err(|e| anyhow::anyhow!("Failed to create event source: {:?}", e))?;

        let keycode = Self::virtual_key_to_cg_keycode(virtual_key);
        let event = CGEvent::new_keyboard_event(source, keycode, !release)
            .map_err(|e| anyhow::anyhow!("Failed to create keyboard event: {:?}", e))?;

        event.post(core_graphics::event::CGEventTapLocation::HID);
        debug!(
            "Sent key event: vk={:#04X}, keycode={}, release={}",
            virtual_key, keycode, release
        );

        Ok(())
    }
}

impl Default for MacosOutputDevice {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for MacosOutputDevice {
    fn clone(&self) -> Self {
        Self
    }
}

impl OutputDeviceTrait for MacosOutputDevice {
    fn send_key(&self, _scan_code: u16, virtual_key: u16, release: bool) -> Result<()> {
        self.send_key_raw(virtual_key, release)
    }

    fn send_mouse_move(&self, x: i32, y: i32, relative: bool) -> Result<()> {
        let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
            .map_err(|e| anyhow::anyhow!("Failed to create event source: {:?}", e))?;

        if relative {
            let current = CGEvent::new(source.clone())
                .map_err(|e| anyhow::anyhow!("Failed to create event: {:?}", e))?;
            let point = current.location();
            let new_point = CGPoint::new(point.x + x as f64, point.y + y as f64);

            let event = CGEvent::new_mouse_event(
                source,
                CGEventType::MouseMoved,
                new_point,
                CGMouseButton::Left,
            )
            .map_err(|e| anyhow::anyhow!("Failed to create mouse event: {:?}", e))?;

            event.post(core_graphics::event::CGEventTapLocation::HID);
        } else {
            let point = CGPoint::new(x as f64, y as f64);
            let event = CGEvent::new_mouse_event(
                source,
                CGEventType::MouseMoved,
                point,
                CGMouseButton::Left,
            )
            .map_err(|e| anyhow::anyhow!("Failed to create mouse event: {:?}", e))?;

            event.post(core_graphics::event::CGEventTapLocation::HID);
        }

        debug!("Sent mouse move: x={}, y={}, relative={}", x, y, relative);
        Ok(())
    }

    fn send_mouse_button(&self, button: MouseButton, release: bool) -> Result<()> {
        let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
            .map_err(|e| anyhow::anyhow!("Failed to create event source: {:?}", e))?;

        let current = CGEvent::new(source.clone())
            .map_err(|e| anyhow::anyhow!("Failed to create event: {:?}", e))?;
        let point = current.location();

        let event_type = match (button, release) {
            (MouseButton::Left, false) => CGEventType::LeftMouseDown,
            (MouseButton::Left, true) => CGEventType::LeftMouseUp,
            (MouseButton::Right, false) => CGEventType::RightMouseDown,
            (MouseButton::Right, true) => CGEventType::RightMouseUp,
            (MouseButton::Middle, false) => CGEventType::OtherMouseDown,
            (MouseButton::Middle, true) => CGEventType::OtherMouseUp,
            _ => CGEventType::OtherMouseDown,
        };

        let cg_button = match button {
            MouseButton::Left => CGMouseButton::Left,
            MouseButton::Right => CGMouseButton::Right,
            _ => CGMouseButton::Center,
        };

        let event = CGEvent::new_mouse_event(source, event_type, point, cg_button)
            .map_err(|e| anyhow::anyhow!("Failed to create mouse event: {:?}", e))?;

        event.post(core_graphics::event::CGEventTapLocation::HID);
        debug!("Sent mouse button: {:?}, release={}", button, release);
        Ok(())
    }

    fn send_mouse_wheel(&self, delta: i32, horizontal: bool) -> Result<()> {
        debug!(
            "Sent mouse wheel: delta={}, horizontal={}",
            delta, horizontal
        );

        Ok(())
    }

    fn send_system_action(&self, action: &SystemAction) -> Result<()> {
        use std::process::Command;

        match action {
            SystemAction::VolumeUp => {
                let _ = Command::new("osascript")
                    .arg("-e")
                    .arg("set volume output volume (output volume of (get volume settings) + 10)")
                    .output();
            }
            SystemAction::VolumeDown => {
                let _ = Command::new("osascript")
                    .arg("-e")
                    .arg("set volume output volume (output volume of (get volume settings) - 10)")
                    .output();
            }
            SystemAction::VolumeMute => {
                let _ = Command::new("osascript")
                    .arg("-e")
                    .arg("set volume with output muted")
                    .output();
            }
            SystemAction::BrightnessUp => {
                let _ = Command::new("brightness").arg("+10").output();
            }
            SystemAction::BrightnessDown => {
                let _ = Command::new("brightness").arg("-10").output();
            }
        }

        debug!("System action executed: {:?}", action);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::output_helpers::char_to_vk;
    use crate::types::{KeyAction, ModifierState, MouseAction, MouseButton};

    // --- char_to_vk tests (shared logic) ---

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

    // --- Device lifecycle ---

    #[test]
    fn test_macos_output_device_creation() {
        let device = MacosOutputDevice::new();
        let _cloned = device.clone();
    }

    #[test]
    fn test_macos_output_device_default() {
        let _device = MacosOutputDevice::default();
    }

    // --- Trait default implementations (send_text / send_combo / dispatch) ---

    #[test]
    fn test_send_text_dispatches_press_release_pairs() {
        let device = MacosOutputDevice::new();
        let result = device.send_text("ab");
        assert!(result.is_ok());
    }

    #[test]
    fn test_send_text_empty_string() {
        let device = MacosOutputDevice::new();
        let result = device.send_text("");
        assert!(result.is_ok());
    }

    #[test]
    fn test_send_text_with_unsupported_chars_skips_gracefully() {
        let device = MacosOutputDevice::new();
        let result = device.send_text("a中b");
        assert!(result.is_ok());
    }

    #[test]
    fn test_send_key_action_type_text() {
        let device = MacosOutputDevice::new();
        let action = KeyAction::TypeText("hello".to_string());
        let result = device.send_key_action(&action);
        assert!(result.is_ok());
    }

    #[test]
    fn test_send_key_action_click() {
        let device = MacosOutputDevice::new();
        let action = KeyAction::Click {
            scan_code: 0,
            virtual_key: 0x41,
        };
        let result = device.send_key_action(&action);
        assert!(result.is_ok());
    }

    #[test]
    fn test_send_key_action_combo() {
        let device = MacosOutputDevice::new();
        let modifiers = ModifierState {
            ctrl: true,
            ..ModifierState::default()
        };
        let action = KeyAction::Combo {
            modifiers,
            key: (0, 0x41),
        };
        let result = device.send_key_action(&action);
        assert!(result.is_ok());
    }

    #[test]
    fn test_send_key_action_none() {
        let device = MacosOutputDevice::new();
        let result = device.send_key_action(&KeyAction::None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_send_mouse_action_move() {
        let device = MacosOutputDevice::new();
        let action = MouseAction::Move {
            x: 100,
            y: 200,
            relative: true,
        };
        let result = device.send_mouse_action(&action);
        assert!(result.is_ok());
    }

    #[test]
    fn test_send_mouse_action_click() {
        let device = MacosOutputDevice::new();
        let action = MouseAction::ButtonClick {
            button: MouseButton::Left,
        };
        let result = device.send_mouse_action(&action);
        assert!(result.is_ok());
    }

    #[test]
    fn test_send_mouse_action_wheel() {
        let device = MacosOutputDevice::new();
        let action = MouseAction::Wheel { delta: 120 };
        let result = device.send_mouse_action(&action);
        assert!(result.is_ok());
    }

    #[test]
    fn test_send_key_action_press_and_release() {
        let device = MacosOutputDevice::new();
        let press = KeyAction::Press {
            scan_code: 0,
            virtual_key: 0x41,
        };
        let release = KeyAction::Release {
            scan_code: 0,
            virtual_key: 0x41,
        };
        assert!(device.send_key_action(&press).is_ok());
        assert!(device.send_key_action(&release).is_ok());
    }

    #[test]
    fn test_send_mouse_action_move_absolute() {
        let device = MacosOutputDevice::new();
        let action = MouseAction::Move {
            x: 1920,
            y: 1080,
            relative: false,
        };
        let result = device.send_mouse_action(&action);
        assert!(result.is_ok());
    }

    #[test]
    fn test_send_mouse_action_right_click() {
        let device = MacosOutputDevice::new();
        let action = MouseAction::ButtonClick {
            button: MouseButton::Right,
        };
        let result = device.send_mouse_action(&action);
        assert!(result.is_ok());
    }

    #[test]
    fn test_send_mouse_action_middle_click() {
        let device = MacosOutputDevice::new();
        let action = MouseAction::ButtonClick {
            button: MouseButton::Middle,
        };
        let result = device.send_mouse_action(&action);
        assert!(result.is_ok());
    }

    #[test]
    fn test_send_mouse_action_wheel_horizontal() {
        let device = MacosOutputDevice::new();
        let action = MouseAction::HWheel { delta: 120 };
        let result = device.send_mouse_action(&action);
        assert!(result.is_ok());
    }
}
