//! Cross-platform mock implementations for testing
//!
//! This module provides mock implementations of platform-specific traits
//! that can be used for testing on any platform.

use crate::types::{
    InputEvent, KeyEvent, KeyState, ModifierState, MouseButton, MouseEvent,
    MouseEventType,
};
use anyhow::Result;
use std::cell::RefCell;
use std::collections::VecDeque;

/// Mock input device for testing
pub struct MockInputDevice {
    events: RefCell<VecDeque<InputEvent>>,
    running: RefCell<bool>,
    modifier_state: RefCell<ModifierState>,
    captured_events: RefCell<Vec<InputEvent>>,
}

impl MockInputDevice {
    /// Create a new mock input device
    pub fn new() -> Self {
        Self {
            events: RefCell::new(VecDeque::new()),
            running: RefCell::new(false),
            modifier_state: RefCell::new(ModifierState::default()),
            captured_events: RefCell::new(Vec::new()),
        }
    }

    /// Inject a key press event
    pub fn inject_key_press(&self, scan_code: u16, virtual_key: u16) {
        let event = KeyEvent::new(scan_code, virtual_key, KeyState::Pressed);
        self.events.borrow_mut().push_back(InputEvent::Key(event));
    }

    /// Inject a key release event
    pub fn inject_key_release(&self, scan_code: u16, virtual_key: u16) {
        let event = KeyEvent::new(scan_code, virtual_key, KeyState::Released);
        self.events.borrow_mut().push_back(InputEvent::Key(event));
    }

    /// Inject a mouse move event
    pub fn inject_mouse_move(&self, x: i32, y: i32) {
        let event = MouseEvent::new(MouseEventType::Move, x, y);
        self.events.borrow_mut().push_back(InputEvent::Mouse(event));
    }

    /// Inject a mouse button down event
    pub fn inject_mouse_button_down(&self, button: MouseButton, x: i32, y: i32) {
        let event = MouseEvent::new(MouseEventType::ButtonDown(button), x, y);
        self.events.borrow_mut().push_back(InputEvent::Mouse(event));
    }

    /// Inject a mouse button up event
    pub fn inject_mouse_button_up(&self, button: MouseButton, x: i32, y: i32) {
        let event = MouseEvent::new(MouseEventType::ButtonUp(button), x, y);
        self.events.borrow_mut().push_back(InputEvent::Mouse(event));
    }

    /// Inject a wheel event
    pub fn inject_wheel(&self, delta: i32, x: i32, y: i32) {
        let event = MouseEvent::new(MouseEventType::Wheel(delta), x, y);
        self.events.borrow_mut().push_back(InputEvent::Mouse(event));
    }

    /// Inject a horizontal wheel event
    pub fn inject_hwheel(&self, delta: i32, x: i32, y: i32) {
        let event = MouseEvent::new(MouseEventType::HWheel(delta), x, y);
        self.events.borrow_mut().push_back(InputEvent::Mouse(event));
    }

    /// Inject an arbitrary event
    pub fn inject_event(&self, event: InputEvent) {
        self.events.borrow_mut().push_back(event);
    }

    /// Get all captured events
    pub fn get_captured_events(&self) -> Vec<InputEvent> {
        self.captured_events.borrow().clone()
    }

    /// Clear captured events
    pub fn clear_captured(&self) {
        self.captured_events.borrow_mut().clear();
    }

    /// Get the number of pending events
    pub fn pending_count(&self) -> usize {
        self.events.borrow().len()
    }

    /// Clear all pending events
    pub fn clear(&self) {
        self.events.borrow_mut().clear();
    }

    /// Set modifier key state
    pub fn set_modifier_state(&self, state: ModifierState) {
        *self.modifier_state.borrow_mut() = state;
    }

    /// Get current modifier key state
    pub fn get_modifier_state(&self) -> ModifierState {
        *self.modifier_state.borrow()
    }
}

impl Default for MockInputDevice {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for input device operations (platform-agnostic)
pub trait InputDeviceOps {
    /// Register the device
    fn register(&mut self) -> Result<()>;
    /// Unregister the device
    fn unregister(&mut self);
    /// Poll for events (non-blocking)
    fn poll_event(&mut self) -> Option<InputEvent>;
    /// Check if the device is running
    fn is_running(&self) -> bool;
    /// Stop the device
    fn stop(&mut self);
}

impl InputDeviceOps for MockInputDevice {
    fn register(&mut self) -> Result<()> {
        *self.running.borrow_mut() = true;
        Ok(())
    }

    fn unregister(&mut self) {
        *self.running.borrow_mut() = false;
    }

    fn poll_event(&mut self) -> Option<InputEvent> {
        if !*self.running.borrow() {
            return None;
        }

        let event = self.events.borrow_mut().pop_front();

        if let Some(ref e) = event {
            // Record captured event
            self.captured_events.borrow_mut().push(e.clone());

            // Update modifier key state
            if let InputEvent::Key(key_event) = e {
                if let Some((modifier, _)) = ModifierState::from_virtual_key(
                    key_event.virtual_key,
                    key_event.state == KeyState::Pressed,
                ) {
                    self.modifier_state.borrow_mut().merge(&modifier);
                }
            }
        }

        event
    }

    fn is_running(&self) -> bool {
        *self.running.borrow()
    }

    fn stop(&mut self) {
        *self.running.borrow_mut() = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_input_device_creation() {
        let device = MockInputDevice::new();
        assert!(!device.is_running());
        assert_eq!(device.pending_count(), 0);
    }

    #[test]
    fn test_mock_input_device_register() {
        let mut device = MockInputDevice::new();
        assert!(!device.is_running());

        device.register().unwrap();
        assert!(device.is_running());

        device.unregister();
        assert!(!device.is_running());
    }

    #[test]
    fn test_mock_inject_key_events() {
        let device = MockInputDevice::new();

        device.inject_key_press(0x1E, 0x41); // 'A' key
        device.inject_key_release(0x1E, 0x41);

        assert_eq!(device.pending_count(), 2);
    }

    #[test]
    fn test_mock_poll_key_events() {
        let mut device = MockInputDevice::new();
        device.register().unwrap();

        device.inject_key_press(0x1E, 0x41);
        device.inject_key_release(0x1E, 0x41);

        // Poll the first event
        let event1 = device.poll_event().unwrap();
        assert!(matches!(
            event1,
            InputEvent::Key(KeyEvent {
                state: KeyState::Pressed,
                ..
            })
        ));

        // Poll second event
        let event2 = device.poll_event().unwrap();
        assert!(matches!(
            event2,
            InputEvent::Key(KeyEvent {
                state: KeyState::Released,
                ..
            })
        ));

        // No more events
        assert!(device.poll_event().is_none());
    }

    #[test]
    fn test_mock_poll_without_register() {
        let mut device = MockInputDevice::new();
        device.inject_key_press(0x1E, 0x41);

        // Should return None when not registered
        assert!(device.poll_event().is_none());
    }

    #[test]
    fn test_mock_inject_mouse_events() {
        let device = MockInputDevice::new();

        device.inject_mouse_move(100, 200);
        device.inject_mouse_button_down(MouseButton::Left, 100, 200);
        device.inject_mouse_button_up(MouseButton::Left, 100, 200);
        device.inject_wheel(120, 100, 200);

        assert_eq!(device.pending_count(), 4);
    }

    #[test]
    fn test_mock_captured_events() {
        let mut device = MockInputDevice::new();
        device.register().unwrap();

        device.inject_key_press(0x1E, 0x41);
        device.inject_key_release(0x1E, 0x41);

        // Poll all events
        let _ = device.poll_event();
        let _ = device.poll_event();

        // Check captured events
        let captured = device.get_captured_events();
        assert_eq!(captured.len(), 2);
    }

    #[test]
    fn test_mock_clear() {
        let device = MockInputDevice::new();

        device.inject_key_press(0x1E, 0x41);
        device.inject_key_press(0x30, 0x42);
        assert_eq!(device.pending_count(), 2);

        device.clear();
        assert_eq!(device.pending_count(), 0);
    }

    #[test]
    fn test_mock_large_event_batch() {
        let mut device = MockInputDevice::new();
        device.register().unwrap();

        // Inject large batch of events
        for _ in 0..1000 {
            device.inject_key_press(0x1E, 0x41); // 'A' key
        }

        assert_eq!(device.pending_count(), 1000);

        // Poll all events
        let mut polled_count = 0;
        while device.poll_event().is_some() {
            polled_count += 1;
            if polled_count > 1100 {
                panic!("Polled more events than injected (possible infinite loop)");
            }
        }

        assert_eq!(polled_count, 1000);
        assert_eq!(device.pending_count(), 0);
    }
}
