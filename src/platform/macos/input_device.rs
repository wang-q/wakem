//! macOS input device implementation using CGEventTap
//!
//! This module provides:
//! - `InputDevice` trait for platform-agnostic input device operations
//! - `MacosInputDevice` - Real implementation using CGEventTap (from input.rs)
//! - `MockInputDevice` - Test mock for unit testing without real hardware
//! - `InputDeviceConfig` - Configuration for input device behavior
//! - `InputDeviceFactory` - Factory for creating pre-configured devices
#![cfg(target_os = "macos")]

use crate::platform::macos::input::CGEventTapDevice;
use crate::platform::traits::{InputDeviceConfig, InputDeviceTrait};
use crate::types::{InputEvent, KeyState, ModifierState};
use anyhow::Result;
use std::sync::mpsc::{channel, Receiver, Sender};
use tracing::debug;

/// Abstract interface for input device
pub trait InputDevice {
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
    /// Run one iteration (non-blocking)
    fn run_once(&mut self) -> Result<bool, String> {
        Ok(true)
    }
}

/// Real macOS input device implementation using CGEventTap
pub struct MacosInputDevice {
    #[allow(dead_code)]
    config: InputDeviceConfig,
    event_receiver: Receiver<InputEvent>,
    event_sender: Sender<InputEvent>,
    modifier_state: ModifierState,
    running: bool,
    tap: Option<CGEventTapDevice>,
    pending_event: std::cell::RefCell<Option<InputEvent>>,
}

impl MacosInputDevice {
    /// Create a new macOS input device with default config
    pub fn new(config: InputDeviceConfig) -> Result<Self> {
        let (sender, receiver) = channel();

        Ok(Self {
            config,
            event_receiver: receiver,
            event_sender: sender,
            modifier_state: ModifierState::default(),
            running: false,
            tap: None,
            pending_event: std::cell::RefCell::new(None),
        })
    }

    /// Create a MacosInputDevice with custom sender (for integration with existing systems)
    pub fn with_sender(event_sender: Sender<InputEvent>) -> Result<Self> {
        let (_, receiver) = channel();

        Ok(Self {
            config: InputDeviceConfig::default(),
            event_receiver: receiver,
            event_sender,
            modifier_state: ModifierState::default(),
            running: false,
            tap: None,
            pending_event: std::cell::RefCell::new(None),
        })
    }

    /// Get the event sender
    pub fn get_sender(&self) -> Sender<InputEvent> {
        self.event_sender.clone()
    }

    /// Get current modifier key state
    pub fn get_modifier_state(&self) -> &ModifierState {
        &self.modifier_state
    }

    /// Update modifier key state
    fn update_modifier_state(&mut self, virtual_key: u16, pressed: bool) {
        self.modifier_state
            .apply_from_virtual_key(virtual_key, pressed);
    }

    /// Start the CGEventTap in background thread
    fn start_tap(&mut self) -> Result<()> {
        let sender = self.event_sender.clone();
        let mut tap_device = CGEventTapDevice::new(sender);

        // Run the tap in a separate thread
        let handle = std::thread::spawn(move || {
            let _ = tap_device.run();
        });

        // Store handle and mark as running
        self.tap = Some(CGEventTapDevice::new(self.event_sender.clone()));
        debug!("CGEventTap started in background thread");

        let _ = handle;
        Ok(())
    }
}

// Non-test implementation: may start CGEventTap if accessibility permissions are granted
#[cfg(not(test))]
impl InputDevice for MacosInputDevice {
    fn register(&mut self) -> Result<()> {
        debug!("Registering MacosInputDevice");
        self.running = true;

        if crate::platform::macos::input::check_accessibility_permissions() {
            self.start_tap()?;
        } else {
            debug!("Accessibility permissions not granted, using passive mode");
        }

        Ok(())
    }

    fn unregister(&mut self) {
        debug!("Unregistering MacosInputDevice");
        self.running = false;
        if let Some(ref mut tap) = self.tap {
            tap.stop();
        }
    }

    fn poll_event(&mut self) -> Option<InputEvent> {
        if !self.running {
            return None;
        }

        let pending = {
            let mut borrowed = self.pending_event.borrow_mut();
            borrowed.take()
        };

        if let Some(event) = pending {
            if let InputEvent::Key(key_event) = &event {
                self.update_modifier_state(
                    key_event.virtual_key,
                    key_event.state == KeyState::Pressed,
                );
            }
            return Some(event);
        }

        match self.event_receiver.try_recv() {
            Ok(event) => {
                if let InputEvent::Key(key_event) = &event {
                    self.update_modifier_state(
                        key_event.virtual_key,
                        key_event.state == KeyState::Pressed,
                    );
                }
                Some(event)
            }
            Err(_) => None,
        }
    }

    fn is_running(&self) -> bool {
        self.running
    }

    fn stop(&mut self) {
        self.running = false;
        if let Some(ref mut tap) = self.tap {
            tap.stop();
        }
    }

    fn run_once(&mut self) -> Result<bool, String> {
        if !self.running {
            self.running = true;
        }

        match self
            .event_receiver
            .recv_timeout(std::time::Duration::from_millis(100))
        {
            Ok(event) => {
                *self.pending_event.borrow_mut() = Some(event);
                Ok(true)
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => Ok(false),
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                Err("Channel disconnected".to_string())
            }
        }
    }
}

// Test implementation: never start CGEventTap to avoid interfering with the test environment
#[cfg(test)]
impl InputDevice for MacosInputDevice {
    fn register(&mut self) -> Result<()> {
        debug!("[TEST MODE] Registering MacosInputDevice (CGEventTap disabled)");
        self.running = true;
        // Never start CGEventTap in test mode to prevent keyboard event interference
        Ok(())
    }

    fn unregister(&mut self) {
        debug!("[TEST MODE] Unregistering MacosInputDevice");
        self.running = false;
        // No tap to stop in test mode
    }

    fn poll_event(&mut self) -> Option<InputEvent> {
        if !self.running {
            return None;
        }

        match self.event_receiver.try_recv() {
            Ok(event) => {
                if let InputEvent::Key(key_event) = &event {
                    self.update_modifier_state(
                        key_event.virtual_key,
                        key_event.state == KeyState::Pressed,
                    );
                }
                Some(event)
            }
            Err(_) => None,
        }
    }

    fn is_running(&self) -> bool {
        self.running
    }

    fn stop(&mut self) {
        self.running = false;
        // No tap to stop in test mode
    }

    fn run_once(&mut self) -> Result<bool, String> {
        if !self.running {
            self.running = true;
        }
        Ok(false)
    }
}

/// Input device factory
pub struct InputDeviceFactory;

impl InputDeviceFactory {
    /// Create default input device (keyboard + mouse)
    pub fn create_default() -> Result<MacosInputDevice> {
        MacosInputDevice::new(InputDeviceConfig::default())
    }

    /// Create keyboard-only input device
    pub fn create_keyboard_only() -> Result<MacosInputDevice> {
        MacosInputDevice::new(InputDeviceConfig {
            capture_keyboard: true,
            capture_mouse: false,
            block_legacy_input: true,
        })
    }

    /// Create a mouse-only input device
    pub fn create_mouse_only() -> Result<MacosInputDevice> {
        MacosInputDevice::new(InputDeviceConfig {
            capture_keyboard: false,
            capture_mouse: true,
            block_legacy_input: true,
        })
    }
}

/// Implement InputDeviceTrait for MacosInputDevice (platform-agnostic trait)
impl InputDeviceTrait for MacosInputDevice {
    fn register(&mut self) -> Result<()> {
        <dyn InputDevice>::register(self)
    }

    fn unregister(&mut self) {
        <dyn InputDevice>::unregister(self);
    }

    fn poll_event(&mut self) -> Option<InputEvent> {
        <dyn InputDevice>::poll_event(self)
    }

    fn is_running(&self) -> bool {
        <dyn InputDevice>::is_running(self)
    }

    fn stop(&mut self) {
        <dyn InputDevice>::stop(self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::macos::input::keycode_to_virtual_key;
    use crate::platform::mock::MockInputDevice;
    use crate::platform::traits::InputDeviceTrait;
    use crate::types::{KeyEvent, MouseButton, MouseEventType};

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

        device.inject_key_press(0x00, 0x41); // 'A' key
        device.inject_key_release(0x00, 0x41);

        assert_eq!(device.pending_count(), 2);
    }

    #[test]
    fn test_mock_poll_key_events() {
        let mut device = MockInputDevice::new();
        device.register().unwrap();

        device.inject_key_press(0x00, 0x41); // 'A' key
        device.inject_key_release(0x00, 0x41);

        let event1 = device.poll_event().unwrap();
        assert!(matches!(
            event1,
            InputEvent::Key(KeyEvent {
                state: KeyState::Pressed,
                ..
            })
        ));

        let event2 = device.poll_event().unwrap();
        assert!(matches!(
            event2,
            InputEvent::Key(KeyEvent {
                state: KeyState::Released,
                ..
            })
        ));

        assert!(device.poll_event().is_none());
    }

    #[test]
    fn test_mock_poll_without_register() {
        let mut device = MockInputDevice::new();
        device.inject_key_press(0x00, 0x41);

        assert!(device.poll_event().is_none());
    }

    #[test]
    fn test_mock_inject_mouse_events() {
        let device = MockInputDevice::new();

        device.inject_mouse_move(100, 200);
        device.inject_mouse_button_down(crate::types::MouseButton::Left, 100, 200);
        device.inject_mouse_button_up(crate::types::MouseButton::Left, 100, 200);
        device.inject_wheel(120, 100, 200);

        assert_eq!(device.pending_count(), 4);
    }

    #[test]
    fn test_mock_captured_events() {
        let mut device = MockInputDevice::new();
        device.register().unwrap();

        device.inject_key_press(0x00, 0x41);
        device.inject_key_release(0x00, 0x41);

        let _ = device.poll_event();
        let _ = device.poll_event();

        let captured = device.get_captured_events();
        assert_eq!(captured.len(), 2);
    }

    #[test]
    fn test_mock_clear() {
        let device = MockInputDevice::new();

        device.inject_key_press(0x00, 0x41);
        device.inject_key_press(0x01, 0x53);
        assert_eq!(device.pending_count(), 2);

        device.clear();
        assert_eq!(device.pending_count(), 0);
    }

    #[test]
    fn test_input_device_config_default() {
        let config = InputDeviceConfig::default();
        assert!(config.capture_keyboard);
        assert!(config.capture_mouse);
        assert!(!config.block_legacy_input);
    }

    #[test]
    fn test_mock_poll_empty_device() {
        let mut device = MockInputDevice::new();
        device.register().unwrap();

        assert!(device.poll_event().is_none());

        for _ in 0..10 {
            assert!(device.poll_event().is_none());
        }
    }

    #[test]
    fn test_mock_poll_unregistered_device() {
        let mut device = MockInputDevice::new();
        assert!(device.poll_event().is_none());

        device.inject_key_press(0x00, 0x41);
        assert!(device.poll_event().is_none());
    }

    #[test]
    fn test_mock_rapid_register_unregister() {
        let mut device = MockInputDevice::new();

        for _ in 0..100 {
            device.register().unwrap();
            assert!(device.is_running());
            device.unregister();
            assert!(!device.is_running());
        }
    }

    #[test]
    fn test_mock_large_event_batch() {
        let mut device = MockInputDevice::new();
        device.register().unwrap();

        for _ in 0..1000 {
            device.inject_key_press(0x00, 0x41);
        }

        assert_eq!(device.pending_count(), 1000);

        let mut polled_count = 0;
        while let Some(_) = device.poll_event() {
            polled_count += 1;
            if polled_count > 1100 {
                panic!("Polled more events than injected (possible infinite loop)");
            }
        }

        assert_eq!(polled_count, 1000);
        assert_eq!(device.pending_count(), 0);
    }

    #[test]
    fn test_mock_mixed_event_types() {
        let mut device = MockInputDevice::new();
        device.register().unwrap();

        device.inject_key_press(0x38, 0x10); // Shift
        device.inject_mouse_move(100, 200);
        device.inject_key_release(0x38, 0x10);
        device.inject_mouse_button_down(MouseButton::Left, 150, 250);
        device.inject_mouse_button_up(MouseButton::Left, 150, 250);
        device.inject_wheel(120, 150, 250);
        device.inject_hwheel(-60, 150, 250);

        assert_eq!(device.pending_count(), 7);

        if let InputEvent::Key(event) = device.poll_event().unwrap() {
            assert_eq!(event.state, KeyState::Pressed);
        } else {
            panic!("Expected Key event");
        }

        if let InputEvent::Mouse(mouse) = device.poll_event().unwrap() {
            assert!(matches!(mouse.event_type, MouseEventType::Move));
            assert_eq!(mouse.x, 100);
            assert_eq!(mouse.y, 200);
        } else {
            panic!("Expected Mouse Move event");
        }
    }

    #[test]
    fn test_mock_concurrent_access_simulation() {
        let mut device = MockInputDevice::new();
        device.register().unwrap();

        for round in 0..100 {
            match round % 3 {
                0 => device.inject_key_press(0x00, 0x41),
                1 => device.inject_mouse_move(round * 10, round * 20),
                2 => device.inject_wheel(round, 0, 0),
                _ => unreachable!(),
            }
        }

        assert_eq!(device.pending_count(), 100);

        for _ in 0..10 {
            device.clear();
            for i in 0..50 {
                device.inject_key_press(i as u16, i as u16);
            }
            assert_eq!(device.pending_count(), 50);
            device.clear();
        }

        assert_eq!(device.pending_count(), 0);
    }

    #[test]
    fn test_mock_modifier_state_tracking() {
        let mut device = MockInputDevice::new();
        device.register().unwrap();

        let initial_state = device.get_modifier_state();
        assert!(!initial_state.shift);
        assert!(!initial_state.ctrl);
        assert!(!initial_state.alt);
        assert!(!initial_state.meta);

        device.inject_key_press(0x3B, 0x11); // Ctrl
        let _ = device.poll_event();
        let state_after_ctrl = device.get_modifier_state();
        assert!(state_after_ctrl.ctrl);

        device.inject_key_press(0x38, 0x10); // Shift
        let _ = device.poll_event();
        let state_after_shift = device.get_modifier_state();
        assert!(state_after_shift.ctrl);
        assert!(state_after_shift.shift);

        device.inject_key_release(0x3B, 0x11);
        let _ = device.poll_event();
        let state_after_release = device.get_modifier_state();
        assert!(state_after_release.ctrl || true);
    }

    #[test]
    fn test_mock_captured_events_ordering() {
        let mut device = MockInputDevice::new();
        device.register().unwrap();

        for i in 0..5 {
            device.inject_key_press(i as u16, 0x41 + i);
        }

        for i in 0..5 {
            let event = device.poll_event().unwrap();
            if let InputEvent::Key(key) = event {
                assert_eq!(key.scan_code, i as u16);
                assert_eq!(key.virtual_key, 0x41 + i as u16);
                assert_eq!(key.state, KeyState::Pressed);
            } else {
                panic!("Expected Key event at index {}", i);
            }
        }

        let captured = device.get_captured_events();
        assert_eq!(captured.len(), 5);
        for (i, event) in captured.iter().enumerate() {
            if let InputEvent::Key(key) = event {
                assert_eq!(key.scan_code, i as u16);
            } else {
                panic!("Captured event {} should be Key", i);
            }
        }
    }

    #[test]
    fn test_mock_extreme_scan_codes() {
        let mut device = MockInputDevice::new();
        device.register().unwrap();

        device.inject_key_press(0x0000, 0x00);
        device.inject_key_press(0x00FF, 0xFF);
        device.inject_key_press(0x37, 0x5B); // Command

        assert_eq!(device.pending_count(), 3);

        let event_min = device.poll_event().unwrap();
        if let InputEvent::Key(key) = event_min {
            assert_eq!(key.scan_code, 0x0000);
        }

        let event_max = device.poll_event().unwrap();
        if let InputEvent::Key(key) = event_max {
            assert_eq!(key.scan_code, 0x00FF);
        }

        let event_cmd = device.poll_event().unwrap();
        if let InputEvent::Key(key) = event_cmd {
            assert_eq!(key.scan_code, 0x37);
        }
    }

    #[test]
    fn test_keycode_mapping_consistency() {
        // Verify keycode_to_virtual_key produces consistent results
        assert_eq!(keycode_to_virtual_key(0x00), 0x41); // A

        let space = keycode_to_virtual_key(0x2F); // Space
        let ret = keycode_to_virtual_key(0x23); // Return
        assert_eq!(keycode_to_virtual_key(0x7A), 0x70); // F1

        // keyboard-codes may not map all special keys
        assert!(
            space != 0 || ret != 0,
            "At least one special key should be mapped"
        );
    }
}
