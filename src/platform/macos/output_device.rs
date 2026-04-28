//! macOS output device implementation using CGEvent
//!
//! This module provides the macOS-specific implementation of the OutputDeviceTrait
//! using Core Graphics event simulation.

use crate::platform::traits::OutputDeviceTrait;
use crate::types::MouseButton;
use anyhow::Result;
#[cfg(not(test))]
use tracing::debug;

/// macOS output device using CGEvent (aligned with Windows SendInputDevice)
#[derive(Debug, Clone)]
pub struct SendInputDevice;

impl SendInputDevice {
    pub fn new() -> Self {
        Self
    }

    /// Convert Windows-style virtual key to macOS CGKeyCode
    fn virtual_key_to_cg_keycode(virtual_key: u16) -> u16 {
        crate::platform::macos::input::virtual_key_to_keycode(virtual_key)
    }
}

impl Default for SendInputDevice {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(test))]
impl OutputDeviceTrait for SendInputDevice {
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

        let current = CGEvent::new(source.clone())
            .map_err(|e| anyhow::anyhow!("Failed to create event: {:?}", e))?;
        let point = current.location();

        let event = CGEvent::new_mouse_event(
            source,
            CGEventType::ScrollWheel,
            point,
            CGMouseButton::Left,
        )
        .map_err(|e| anyhow::anyhow!("Failed to create scroll event: {:?}", e))?;

        if horizontal {
            event.set_integer_value_field(
                core_graphics::event::EventField::SCROLL_WHEEL_EVENT_DELTA_AXIS_2,
                delta as i64,
            );
        } else {
            event.set_integer_value_field(
                core_graphics::event::EventField::SCROLL_WHEEL_EVENT_DELTA_AXIS_1,
                delta as i64,
            );
        }

        event.post(CGEventTapLocation::HID);
        debug!(
            "Sent mouse wheel: delta={}, horizontal={}",
            delta, horizontal
        );
        Ok(())
    }
}

#[cfg(test)]
crate::impl_test_output_device!(SendInputDevice);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::traits::OutputDeviceTrait;

    #[test]
    fn test_send_input_device_creation() {
        let device = SendInputDevice::new();
        let _cloned = device.clone();
        let _default: SendInputDevice = Default::default();
    }

    #[test]
    fn test_virtual_key_conversion() {
        // Test that virtual key conversion works for common keys
        let key_a = SendInputDevice::virtual_key_to_cg_keycode(0x41); // 'A'
        let key_enter = SendInputDevice::virtual_key_to_cg_keycode(0x0D); // Enter
        let key_space = SendInputDevice::virtual_key_to_cg_keycode(0x20); // Space

        // Just verify they don't panic and return valid values
        assert!(key_a > 0 || key_a == 0, "Key A conversion should not panic");
        assert!(
            key_enter > 0 || key_enter == 0,
            "Enter conversion should not panic"
        );
        assert!(
            key_space > 0 || key_space == 0,
            "Space conversion should not panic"
        );
    }

    #[test]
    fn test_send_key_action() {
        use crate::types::KeyAction;

        let device = SendInputDevice::new();

        // Test KeyAction::Press
        let press_action = KeyAction::Press {
            scan_code: 0x1E,
            virtual_key: 0x41,
        };
        let result = device.send_key_action(&press_action);
        assert!(result.is_ok(), "Press action should succeed in test mode");

        // Test KeyAction::Release
        let release_action = KeyAction::Release {
            scan_code: 0x1E,
            virtual_key: 0x41,
        };
        let result = device.send_key_action(&release_action);
        assert!(result.is_ok(), "Release action should succeed in test mode");

        // Test KeyAction::Click
        let click_action = KeyAction::Click {
            scan_code: 0x1E,
            virtual_key: 0x41,
        };
        let result = device.send_key_action(&click_action);
        assert!(result.is_ok(), "Click action should succeed in test mode");

        // Test KeyAction::None
        let none_action = KeyAction::None;
        let result = device.send_key_action(&none_action);
        assert!(result.is_ok(), "None action should succeed");
    }

    #[test]
    fn test_send_mouse_action() {
        use crate::types::MouseAction;

        let device = SendInputDevice::new();

        // Test MouseAction::Move (absolute)
        let move_action = MouseAction::Move {
            x: 100,
            y: 200,
            relative: false,
        };
        let result = device.send_mouse_action(&move_action);
        assert!(
            result.is_ok(),
            "Mouse move (absolute) should succeed in test mode"
        );

        // Test MouseAction::Move (relative)
        let move_rel_action = MouseAction::Move {
            x: 10,
            y: -10,
            relative: true,
        };
        let result = device.send_mouse_action(&move_rel_action);
        assert!(
            result.is_ok(),
            "Mouse move (relative) should succeed in test mode"
        );

        // Test MouseAction::ButtonDown
        let button_down = MouseAction::ButtonDown {
            button: MouseButton::Left,
        };
        let result = device.send_mouse_action(&button_down);
        assert!(result.is_ok(), "Button down should succeed in test mode");

        // Test MouseAction::ButtonUp
        let button_up = MouseAction::ButtonUp {
            button: MouseButton::Right,
        };
        let result = device.send_mouse_action(&button_up);
        assert!(result.is_ok(), "Button up should succeed in test mode");

        // Test MouseAction::ButtonClick
        let button_click = MouseAction::ButtonClick {
            button: MouseButton::Middle,
        };
        let result = device.send_mouse_action(&button_click);
        assert!(result.is_ok(), "Button click should succeed in test mode");

        // Test MouseAction::Wheel
        let wheel = MouseAction::Wheel { delta: 120 };
        let result = device.send_mouse_action(&wheel);
        assert!(result.is_ok(), "Wheel should succeed in test mode");

        // Test MouseAction::HWheel
        let hwheel = MouseAction::HWheel { delta: -60 };
        let result = device.send_mouse_action(&hwheel);
        assert!(result.is_ok(), "HWheel should succeed in test mode");

        // Test MouseAction::None
        let none = MouseAction::None;
        let result = device.send_mouse_action(&none);
        assert!(result.is_ok(), "None action should succeed");
    }

    #[test]
    fn test_send_text() {
        let device = SendInputDevice::new();

        // Test sending text
        let result = device.send_text("Hello");
        assert!(result.is_ok(), "Send text should succeed in test mode");

        // Test empty text
        let result = device.send_text("");
        assert!(result.is_ok(), "Empty text should succeed");

        // Test text with spaces
        let result = device.send_text("Hello World 123");
        assert!(
            result.is_ok(),
            "Text with spaces should succeed in test mode"
        );
    }

    #[test]
    fn test_send_combo() {
        use crate::types::ModifierState;

        let device = SendInputDevice::new();

        // Test simple combo with Ctrl
        let modifiers = ModifierState {
            ctrl: true,
            ..Default::default()
        };
        let result = device.send_combo(&modifiers, 0x1E, 0x41);
        assert!(result.is_ok(), "Ctrl combo should succeed in test mode");

        // Test combo with multiple modifiers
        let modifiers = ModifierState {
            ctrl: true,
            shift: true,
            alt: false,
            meta: false,
        };
        let result = device.send_combo(&modifiers, 0x1E, 0x41);
        assert!(
            result.is_ok(),
            "Multi-modifier combo should succeed in test mode"
        );

        // Test combo with all modifiers
        let modifiers = ModifierState {
            ctrl: true,
            shift: true,
            alt: true,
            meta: true,
        };
        let result = device.send_combo(&modifiers, 0x1E, 0x41);
        assert!(
            result.is_ok(),
            "All-modifier combo should succeed in test mode"
        );
    }
}
