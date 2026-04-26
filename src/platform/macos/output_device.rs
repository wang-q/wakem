//! macOS output device implementation using CGEvent
//!
//! This module uses Core Graphics to send simulated input events.
//! Shared logic (char mapping, text input, key combos) is in [output_helpers].

// Allow dead code - this module is under development for macOS output support
#![allow(dead_code)]

use crate::platform::traits::OutputDeviceTrait;
use crate::types::MouseButton;
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
        crate::platform::macos::input::virtual_key_to_keycode(virtual_key)
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
crate::impl_test_output_device!(MacosOutputDevice);
