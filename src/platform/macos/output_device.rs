//! macOS output device implementation using CGEvent
//!
//! This module uses Core Graphics to send simulated input events.
//! Shared logic (char mapping, text input, key combos) is in [output_helpers].
#![cfg(target_os = "macos")]

use crate::platform::traits::OutputDeviceTrait;
use crate::types::{MouseButton, SystemAction};
use anyhow::Result;
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
            0x5C => 0x37,
            _ => virtual_key,
        }
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

// Non-test implementation: sends real events to the system
#[cfg(not(test))]
impl OutputDeviceTrait for MacosOutputDevice {
    fn send_key(&self, _scan_code: u16, virtual_key: u16, release: bool) -> Result<()> {
        use core_graphics::event::{CGEvent, CGEventTapLocation};
        use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

        let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
            .map_err(|e| anyhow::anyhow!("Failed to create event source: {:?}", e))?;

        let keycode = Self::virtual_key_to_cg_keycode(virtual_key);
        let event = CGEvent::new_keyboard_event(source, keycode, !release)
            .map_err(|e| anyhow::anyhow!("Failed to create keyboard event: {:?}", e))?;

        event.post(CGEventTapLocation::HID);
        debug!(
            "Sent key event: vk={:#04X}, keycode={}, release={}",
            virtual_key, keycode, release
        );

        Ok(())
    }

    fn send_mouse_move(&self, x: i32, y: i32, relative: bool) -> Result<()> {
        use core_graphics::event::{
            CGEvent, CGEventTapLocation, CGEventType, CGMouseButton,
        };
        use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
        use core_graphics::geometry::CGPoint;

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

            event.post(CGEventTapLocation::HID);
        } else {
            let point = CGPoint::new(x as f64, y as f64);
            let event = CGEvent::new_mouse_event(
                source,
                CGEventType::MouseMoved,
                point,
                CGMouseButton::Left,
            )
            .map_err(|e| anyhow::anyhow!("Failed to create mouse event: {:?}", e))?;

            event.post(CGEventTapLocation::HID);
        }

        debug!("Sent mouse move: x={}, y={}, relative={}", x, y, relative);
        Ok(())
    }

    fn send_mouse_button(&self, button: MouseButton, release: bool) -> Result<()> {
        use core_graphics::event::{
            CGEvent, CGEventTapLocation, CGEventType, CGMouseButton,
        };
        use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

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

        event.post(CGEventTapLocation::HID);
        debug!("Sent mouse button: {:?}, release={}", button, release);
        Ok(())
    }

    fn send_mouse_wheel(&self, delta: i32, horizontal: bool) -> Result<()> {
        use core_graphics::event::{
            CGEvent, CGEventTapLocation, CGEventType, CGMouseButton,
        };
        use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

        let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
            .map_err(|e| anyhow::anyhow!("Failed to create event source: {:?}", e))?;

        // Get current mouse position for scroll event
        let current = CGEvent::new(source.clone())
            .map_err(|e| anyhow::anyhow!("Failed to create event: {:?}", e))?;
        let point = current.location();

        // On macOS, scroll events are created using mouse events with scroll types
        // CGEventType::ScrollWheel for vertical, we use NSEvent for horizontal
        let event_type = if horizontal {
            // For horizontal scroll, we simulate Shift+ScrollWheel
            // First press Shift
            let shift_event = CGEvent::new_keyboard_event(source.clone(), 0x38, true)
                .map_err(|e| {
                    anyhow::anyhow!("Failed to create shift down event: {:?}", e)
                })?;
            shift_event.post(CGEventTapLocation::HID);

            // Then scroll
            let scroll_event = CGEvent::new_mouse_event(
                source.clone(),
                CGEventType::ScrollWheel,
                point,
                CGMouseButton::Left,
            )
            .map_err(|e| anyhow::anyhow!("Failed to create scroll event: {:?}", e))?;
            scroll_event.set_integer_value_field(
                core_graphics::event::EventField::SCROLL_WHEEL_EVENT_DELTA_AXIS_1,
                delta as i64,
            );
            scroll_event.post(CGEventTapLocation::HID);

            // Release Shift
            let shift_up = CGEvent::new_keyboard_event(source.clone(), 0x38, false)
                .map_err(|e| {
                    anyhow::anyhow!("Failed to create shift up event: {:?}", e)
                })?;
            shift_up.post(CGEventTapLocation::HID);

            debug!("Sent horizontal mouse wheel: delta={}", delta);
            return Ok(());
        } else {
            CGEventType::ScrollWheel
        };

        let event =
            CGEvent::new_mouse_event(source, event_type, point, CGMouseButton::Left)
                .map_err(|e| {
                    anyhow::anyhow!("Failed to create scroll event: {:?}", e)
                })?;

        // Set scroll delta
        event.set_integer_value_field(
            core_graphics::event::EventField::SCROLL_WHEEL_EVENT_DELTA_AXIS_1,
            delta as i64,
        );

        event.post(CGEventTapLocation::HID);
        debug!(
            "Sent mouse wheel: delta={}, horizontal={}",
            delta, horizontal
        );
        Ok(())
    }

    fn send_system_action(&self, action: &SystemAction) -> Result<()> {
        use crate::platform::macos::native_api::{core_audio, display};

        match action {
            SystemAction::VolumeUp => {
                core_audio::volume_up(0.1)
                    .map_err(|e| anyhow::anyhow!("Volume up failed: {}", e))?;
            }
            SystemAction::VolumeDown => {
                core_audio::volume_down(0.1)
                    .map_err(|e| anyhow::anyhow!("Volume down failed: {}", e))?;
            }
            SystemAction::VolumeMute => {
                core_audio::toggle_mute()
                    .map_err(|e| anyhow::anyhow!("Toggle mute failed: {}", e))?;
            }
            SystemAction::BrightnessUp => {
                display::brightness_up(0.1)
                    .map_err(|e| anyhow::anyhow!("Brightness up failed: {}", e))?;
            }
            SystemAction::BrightnessDown => {
                display::brightness_down(0.1)
                    .map_err(|e| anyhow::anyhow!("Brightness down failed: {}", e))?;
            }
        }

        debug!("System action executed: {:?}", action);
        Ok(())
    }
}

// Test implementation: no-op to prevent interfering with the test environment
#[cfg(test)]
impl OutputDeviceTrait for MacosOutputDevice {
    fn send_key(&self, _scan_code: u16, virtual_key: u16, release: bool) -> Result<()> {
        // Ensure this is never called in production - this is a safety check
        if std::env::var("WAKEM_ALLOW_REAL_OUTPUT").is_ok() {
            panic!("Real output attempted in test mode! Set WAKEM_ALLOW_REAL_OUTPUT to allow.");
        }
        debug!(
            "[TEST MODE] Mock key event: vk={:#04X}, release={}",
            virtual_key, release
        );
        Ok(())
    }

    fn send_mouse_move(&self, x: i32, y: i32, relative: bool) -> Result<()> {
        debug!(
            "[TEST MODE] Mock mouse move: x={}, y={}, relative={}",
            x, y, relative
        );
        Ok(())
    }

    fn send_mouse_button(&self, button: MouseButton, release: bool) -> Result<()> {
        debug!(
            "[TEST MODE] Mock mouse button: {:?}, release={}",
            button, release
        );
        Ok(())
    }

    fn send_mouse_wheel(&self, delta: i32, horizontal: bool) -> Result<()> {
        debug!(
            "[TEST MODE] Mock mouse wheel: delta={}, horizontal={}",
            delta, horizontal
        );
        Ok(())
    }

    fn send_system_action(&self, action: &SystemAction) -> Result<()> {
        debug!("[TEST MODE] Mock system action: {:?}", action);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::platform::mock::{MockOutputDevice, MockOutputEvent};
    // Note: char_to_vk tests are in platform::output_helpers module

    // --- Device lifecycle (no side effects) ---

    #[test]
    fn test_macos_output_device_creation() {
        let device = MockOutputDevice::new();
        let _cloned = device.clone();
    }

    #[test]
    fn test_macos_output_device_default() {
        let _device = MockOutputDevice::default();
    }
}

/// Re-export MockOutputDevice and MockOutputEvent from platform::mock
#[cfg(test)]
pub use crate::platform::mock::{MockOutputDevice, MockOutputEvent};
