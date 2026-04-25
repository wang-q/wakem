//! macOS input device implementation using CGEventTap
//!
//! This module provides:
//! - `InputDevice` trait for platform-agnostic input device operations
//! - `MacosInputDevice` - Real implementation using CGEventTap (from input.rs)
//! - `InputDeviceConfig` - Configuration for input device behavior
//! - `InputDeviceFactory` - Factory for creating pre-configured devices
#![cfg(target_os = "macos")]

use crate::platform::input_device_common::InputDeviceBase;
use crate::platform::macos::input::CGEventTapDevice;
use crate::platform::traits::{InputDeviceConfig, InputDeviceTrait};
use crate::types::{InputEvent, KeyState, ModifierState};
use anyhow::Result;
use std::sync::mpsc::Sender;
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
    base: InputDeviceBase,
    tap: Option<CGEventTapDevice>,
    pending_event: std::cell::RefCell<Option<InputEvent>>,
}

impl MacosInputDevice {
    /// Create a new macOS input device with default config
    pub fn new(config: InputDeviceConfig) -> Result<Self> {
        Ok(Self {
            config,
            base: InputDeviceBase::new(),
            tap: None,
            pending_event: std::cell::RefCell::new(None),
        })
    }

    /// Create a MacosInputDevice with custom sender (for integration with existing systems)
    pub fn with_sender(event_sender: Sender<InputEvent>) -> Result<Self> {
        Ok(Self {
            config: InputDeviceConfig::default(),
            base: InputDeviceBase::with_sender(event_sender),
            tap: None,
            pending_event: std::cell::RefCell::new(None),
        })
    }

    /// Get the event sender
    pub fn get_sender(&self) -> Sender<InputEvent> {
        self.base.sender()
    }

    /// Get current modifier key state
    pub fn get_modifier_state(&self) -> &ModifierState {
        &self.base.modifier_state
    }

    /// Start the CGEventTap in background thread
    fn start_tap(&mut self) -> Result<()> {
        let sender = self.base.sender();
        let mut tap_device = CGEventTapDevice::new(sender);

        let handle = std::thread::spawn(move || {
            let _ = tap_device.run();
        });

        self.tap = Some(CGEventTapDevice::new(self.base.sender()));
        debug!("CGEventTap started in background thread");

        let _ = handle;
        Ok(())
    }

    /// Poll event with pending event support (macOS-specific)
    fn poll_event_with_pending(&mut self) -> Option<InputEvent> {
        if !self.base.running {
            return None;
        }

        let pending = {
            let mut borrowed = self.pending_event.borrow_mut();
            borrowed.take()
        };

        if let Some(event) = pending {
            if let InputEvent::Key(key_event) = &event {
                self.base.update_modifier_state(
                    key_event.virtual_key,
                    key_event.state == KeyState::Pressed,
                );
            }
            return Some(event);
        }

        self.base.try_recv_event()
    }
}

// Non-test implementation: may start CGEventTap if accessibility permissions are granted
#[cfg(not(test))]
impl InputDevice for MacosInputDevice {
    fn register(&mut self) -> Result<()> {
        debug!("Registering MacosInputDevice");
        self.base.running = true;

        if crate::platform::macos::input::check_accessibility_permissions() {
            self.start_tap()?;
        } else {
            debug!("Accessibility permissions not granted, using passive mode");
        }

        Ok(())
    }

    fn unregister(&mut self) {
        debug!("Unregistering MacosInputDevice");
        self.base.running = false;
        if let Some(ref mut tap) = self.tap {
            tap.stop();
        }
    }

    fn poll_event(&mut self) -> Option<InputEvent> {
        self.poll_event_with_pending()
    }

    fn is_running(&self) -> bool {
        self.base.is_running()
    }

    fn stop(&mut self) {
        self.base.stop();
        if let Some(ref mut tap) = self.tap {
            tap.stop();
        }
    }

    fn run_once(&mut self) -> Result<bool, String> {
        if !self.base.running {
            self.base.running = true;
        }

        match self
            .base
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
        self.base.running = true;
        Ok(())
    }

    fn unregister(&mut self) {
        debug!("[TEST MODE] Unregistering MacosInputDevice");
        self.base.running = false;
    }

    fn poll_event(&mut self) -> Option<InputEvent> {
        if !self.base.running {
            return None;
        }
        self.base.try_recv_event()
    }

    fn is_running(&self) -> bool {
        self.base.is_running()
    }

    fn stop(&mut self) {
        self.base.stop();
    }

    fn run_once(&mut self) -> Result<bool, String> {
        if !self.base.running {
            self.base.running = true;
        }
        Ok(false)
    }
}

/// Input device factory
pub use crate::platform::input_device_common::InputDeviceFactory;

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

    #[test]
    fn test_keycode_mapping_consistency() {
        assert_eq!(keycode_to_virtual_key(0x00), 0x41);

        let space = keycode_to_virtual_key(0x2F);
        let ret = keycode_to_virtual_key(0x23);
        assert_eq!(keycode_to_virtual_key(0x7A), 0x70);

        assert!(
            space != 0 || ret != 0,
            "At least one special key should be mapped"
        );
    }
}
