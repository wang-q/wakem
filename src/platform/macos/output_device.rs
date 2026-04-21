//! macOS output device implementation using CGEvent
//!
//! This module uses Core Graphics to send simulated input events.

use crate::platform::traits::OutputDeviceTrait;
use crate::types::{KeyAction, ModifierState, MouseAction, MouseButton, SystemAction};
use anyhow::Result;
use core_graphics::event::{CGEvent, CGEventFlags, CGEventType, CGMouseButton};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use core_graphics::geometry::CGPoint;
use tracing::debug;

/// macOS output device using CGEvent
#[derive(Clone)]
pub struct MacosOutputDevice;

impl MacosOutputDevice {
    /// Create a new macOS output device
    pub fn new() -> Self {
        Self
    }

    /// Convert virtual key code to CGKeyCode
    fn virtual_key_to_cg_keycode(&self, virtual_key: u16) -> u16 {
        // macOS CGKeyCode values for common keys
        // Reference: https://developer.apple.com/library/archive/documentation/Carbon/Reference/QuartzEventServicesRef/
        match virtual_key {
            // Letters
            0x41 => 0x00, // A
            0x53 => 0x01, // S
            0x44 => 0x02, // D
            0x46 => 0x03, // F
            0x48 => 0x04, // H
            0x47 => 0x05, // G
            0x5A => 0x06, // Z
            0x58 => 0x07, // X
            0x43 => 0x08, // C
            0x56 => 0x09, // V
            0x42 => 0x0B, // B
            0x51 => 0x0C, // Q
            0x57 => 0x0D, // W
            0x45 => 0x0E, // E
            0x52 => 0x0F, // R
            0x59 => 0x10, // Y
            0x54 => 0x11, // T
            0x31 => 0x12, // 1
            0x32 => 0x13, // 2
            0x33 => 0x14, // 3
            0x34 => 0x15, // 4
            0x36 => 0x16, // 6
            0x35 => 0x17, // 5
            0x3D => 0x18, // =
            0x39 => 0x19, // 9
            0x37 => 0x1A, // 7
            0x2D => 0x1B, // -
            0x38 => 0x1C, // 8
            0x30 => 0x1D, // 0
            0x5D => 0x1E, // ]
            0x4F => 0x1F, // O
            0x55 => 0x20, // U
            0x5B => 0x21, // [
            0x49 => 0x22, // I
            0x50 => 0x23, // P
            0x0D => 0x24, // Return
            0x4C => 0x25, // L
            0x4A => 0x26, // J
            0xDE => 0x27, // '
            0x4B => 0x28, // K
            0x3B => 0x29, // ;
            0xDC => 0x2A, // \
            0xBC => 0x2B, // ,
            0xBF => 0x2C, // /
            0x4E => 0x2D, // N
            0x4D => 0x2E, // M
            0xBE => 0x2F, // .
            0x09 => 0x30, // Tab
            0x20 => 0x31, // Space
            0xC0 => 0x32, // `
            0x08 => 0x33, // Backspace
            0x1B => 0x35, // Escape
            0x14 => 0x3A, // Caps Lock
            // Function keys
            0x70 => 0x7A, // F1
            0x71 => 0x78, // F2
            0x72 => 0x63, // F3
            0x73 => 0x76, // F4
            0x74 => 0x60, // F5
            0x75 => 0x61, // F6
            0x76 => 0x62, // F7
            0x77 => 0x64, // F8
            0x78 => 0x65, // F9
            0x79 => 0x6D, // F10
            0x7A => 0x67, // F11
            0x7B => 0x6F, // F12
            // Navigation
            0x24 => 0x72, // Home
            0x23 => 0x73, // End
            0x21 => 0x74, // Page Up
            0x22 => 0x79, // Page Down
            0x25 => 0x7B, // Left Arrow
            0x26 => 0x7E, // Up Arrow
            0x27 => 0x7C, // Right Arrow
            0x28 => 0x7D, // Down Arrow
            // Modifiers
            0xA0 | 0xA1 | 0x10 => 0x38, // Shift
            0xA2 | 0xA3 | 0x11 => 0x3B, // Control
            0xA4 | 0xA5 | 0x12 => 0x3A, // Option/Alt
            0x5B | 0x5C => 0x37,        // Command/Win
            // Default: return as-is
            _ => virtual_key,
        }
    }

    /// Convert modifiers to CGEventFlags
    fn modifiers_to_cg_flags(&self, modifiers: &ModifierState) -> CGEventFlags {
        let mut flags = CGEventFlags::empty();
        if modifiers.shift {
            flags |= CGEventFlags::CGEventFlagShift;
        }
        if modifiers.ctrl {
            flags |= CGEventFlags::CGEventFlagControl;
        }
        if modifiers.alt {
            flags |= CGEventFlags::CGEventFlagAlternate;
        }
        if modifiers.meta {
            flags |= CGEventFlags::CGEventFlagCommand;
        }
        flags
    }

    /// Get CGEventType for mouse button
    fn get_mouse_event_type(&self, button: MouseButton, release: bool) -> CGEventType {
        match (button, release) {
            (MouseButton::Left, false) => CGEventType::LeftMouseDown,
            (MouseButton::Left, true) => CGEventType::LeftMouseUp,
            (MouseButton::Right, false) => CGEventType::RightMouseDown,
            (MouseButton::Right, true) => CGEventType::RightMouseUp,
            (MouseButton::Middle, false) => CGEventType::OtherMouseDown,
            (MouseButton::Middle, true) => CGEventType::OtherMouseUp,
            _ => CGEventType::OtherMouseDown,
        }
    }

    /// Get mouse button number for CGEvent
    fn get_mouse_button_number(&self, button: MouseButton) -> i64 {
        match button {
            MouseButton::Left => 0,
            MouseButton::Right => 1,
            MouseButton::Middle => 2,
            MouseButton::X1 => 3,
            MouseButton::X2 => 4,
        }
    }
}

impl Default for MacosOutputDevice {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputDeviceTrait for MacosOutputDevice {
    fn send_key_action(&self, action: &KeyAction) -> Result<()> {
        match action {
            KeyAction::Press {
                scan_code,
                virtual_key,
            } => self.send_key(*scan_code, *virtual_key, false),
            KeyAction::Release {
                scan_code,
                virtual_key,
            } => self.send_key(*scan_code, *virtual_key, true),
            KeyAction::Click {
                scan_code,
                virtual_key,
            } => {
                self.send_key(*scan_code, *virtual_key, false)?;
                self.send_key(*scan_code, *virtual_key, true)
            }
            KeyAction::TypeText(text) => self.send_text(text),
            KeyAction::Combo { modifiers, key } => {
                self.send_combo(modifiers, key.0, key.1)
            }
            KeyAction::None => Ok(()),
        }
    }

    fn send_key(&self, _scan_code: u16, virtual_key: u16, release: bool) -> Result<()> {
        let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
            .map_err(|e| anyhow::anyhow!("Failed to create event source: {:?}", e))?;

        let keycode = self.virtual_key_to_cg_keycode(virtual_key);
        let event = CGEvent::new_keyboard_event(source, keycode, !release)
            .map_err(|e| anyhow::anyhow!("Failed to create keyboard event: {:?}", e))?;

        event.post(core_graphics::event::CGEventTapLocation::HID);
        debug!(
            "Sent key event: vk={:#04X}, keycode={}, release={}",
            virtual_key, keycode, release
        );

        Ok(())
    }

    fn send_mouse_action(&self, action: &MouseAction) -> Result<()> {
        match action {
            MouseAction::Move { x, y, relative } => {
                self.send_mouse_move(*x, *y, *relative)
            }
            MouseAction::ButtonDown { button } => self.send_mouse_button(*button, false),
            MouseAction::ButtonUp { button } => self.send_mouse_button(*button, true),
            MouseAction::ButtonClick { button } => {
                self.send_mouse_button(*button, false)?;
                self.send_mouse_button(*button, true)
            }
            MouseAction::Wheel { delta } => self.send_mouse_wheel(*delta, false),
            MouseAction::HWheel { delta } => self.send_mouse_wheel(*delta, true),
            MouseAction::None => Ok(()),
        }
    }

    fn send_mouse_move(&self, x: i32, y: i32, relative: bool) -> Result<()> {
        let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
            .map_err(|e| anyhow::anyhow!("Failed to create event source: {:?}", e))?;

        if relative {
            // For relative movement, we need to get current position first
            let current = CGEvent::new(source.clone())
                .map_err(|e| anyhow::anyhow!("Failed to create event: {:?}", e))?;
            let point = current.location();
            let new_point = CGPoint::new(point.x + x as f64, point.y + y as f64);

            let event = CGEvent::new_mouse_event(
                source,
                CGEventType::MouseMoved,
                new_point,
                CGMouseButton::Left, // Button doesn't matter for move
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

        // Get current mouse position
        let current = CGEvent::new(source.clone())
            .map_err(|e| anyhow::anyhow!("Failed to create event: {:?}", e))?;
        let point = current.location();

        let event_type = self.get_mouse_event_type(button, release);
        let cg_button = match button {
            MouseButton::Left => CGMouseButton::Left,
            MouseButton::Right => CGMouseButton::Right,
            _ => CGMouseButton::Center,
        };

        let event = CGEvent::new_mouse_event(source, event_type, point, cg_button)
            .map_err(|e| anyhow::anyhow!("Failed to create mouse event: {:?}", e))?;

        // Note: MouseEventButtonNumber field is not directly available in core_graphics crate
        // The button is already set in new_mouse_event

        event.post(core_graphics::event::CGEventTapLocation::HID);
        debug!("Sent mouse button: {:?}, release={}", button, release);
        Ok(())
    }

    fn send_mouse_wheel(&self, _delta: i32, _horizontal: bool) -> Result<()> {
        // Mouse wheel simulation on macOS requires scroll wheel event creation
        // which is complex in core_graphics. For now, this is a placeholder.
        debug!("Mouse wheel not fully implemented on macOS");
        Ok(())
    }

    fn send_system_action(&self, action: &SystemAction) -> Result<()> {
        // System actions (volume, brightness, etc.) are handled differently on macOS
        // For now, we'll just log them as they may require special handling
        debug!("System action requested: {:?}", action);
        Ok(())
    }
}

impl MacosOutputDevice {
    /// Send text by typing each character
    fn send_text(&self, text: &str) -> Result<()> {
        // For now, we'll type each character as a separate key event
        // A more sophisticated implementation would use the current keyboard layout
        // to convert characters to key codes
        for ch in text.chars() {
            // Try to find a simple mapping for ASCII characters
            if let Some((vk, shift)) = Self::char_to_virtual_key(ch) {
                if shift {
                    // Press shift
                    self.send_key(0, 0x10, false)?; // Shift down
                }
                self.send_key(0, vk, false)?; // Key down
                self.send_key(0, vk, true)?; // Key up
                if shift {
                    // Release shift
                    self.send_key(0, 0x10, true)?; // Shift up
                }
            }
        }
        Ok(())
    }

    /// Send key combination
    fn send_combo(
        &self,
        modifiers: &ModifierState,
        _scan_code: u16,
        virtual_key: u16,
    ) -> Result<()> {
        // Press modifiers
        if modifiers.ctrl {
            self.send_key(0, 0x11, false)?; // Control down
        }
        if modifiers.alt {
            self.send_key(0, 0x12, false)?; // Alt down
        }
        if modifiers.shift {
            self.send_key(0, 0x10, false)?; // Shift down
        }
        if modifiers.meta {
            self.send_key(0, 0x5B, false)?; // Command down
        }

        // Press and release target key
        self.send_key(_scan_code, virtual_key, false)?;
        self.send_key(_scan_code, virtual_key, true)?;

        // Release modifiers (reverse order)
        if modifiers.meta {
            self.send_key(0, 0x5B, true)?; // Command up
        }
        if modifiers.shift {
            self.send_key(0, 0x10, true)?; // Shift up
        }
        if modifiers.alt {
            self.send_key(0, 0x12, true)?; // Alt up
        }
        if modifiers.ctrl {
            self.send_key(0, 0x11, true)?; // Control up
        }

        Ok(())
    }

    /// Convert character to virtual key code and shift state
    fn char_to_virtual_key(ch: char) -> Option<(u16, bool)> {
        match ch {
            'a'..='z' => Some((0x41 + (ch as u16 - 'a' as u16), false)),
            'A'..='Z' => Some((0x41 + (ch as u16 - 'A' as u16), true)),
            '0'..='9' => Some((0x30 + (ch as u16 - '0' as u16), false)),
            ' ' => Some((0x20, false)),
            '\n' => Some((0x0D, false)),
            '\t' => Some((0x09, false)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macos_output_device_creation() {
        let device = MacosOutputDevice::new();
        // Just verify it can be created
        let _ = device;
    }

    #[test]
    fn test_virtual_key_mapping() {
        let device = MacosOutputDevice::new();

        // Test some common keys
        assert_eq!(device.virtual_key_to_cg_keycode(0x41), 0x00); // A
        assert_eq!(device.virtual_key_to_cg_keycode(0x53), 0x01); // S
        assert_eq!(device.virtual_key_to_cg_keycode(0x20), 0x31); // Space
        assert_eq!(device.virtual_key_to_cg_keycode(0x0D), 0x24); // Return
    }
}
