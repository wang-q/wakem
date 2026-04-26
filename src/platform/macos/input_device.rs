//! macOS input device implementation using CGEventTap
//!
//! This module provides a macOS input device using CGEventTap.
//! Uses the generic [InputDevice] from [input_device_common] to share code
//! with the Windows implementation.
#![cfg(target_os = "macos")]

use crate::platform::input_device_common::{InputDevice, PlatformInputDevice};
use crate::platform::traits::{InputDeviceConfig, InputDeviceTrait};
use crate::types::{InputEvent, KeyState, ModifierState};
use anyhow::Result;
use std::sync::mpsc::Sender;
use tracing::debug;

/// macOS CGEventTap device type alias
pub type MacosInputDevice = InputDevice<CGEventTapInner>;

/// Inner CGEventTap device
pub struct CGEventTapInner {
    tap: crate::platform::macos::input::CGEventTapDevice,
}

impl PlatformInputDevice for CGEventTapInner {
    fn create(sender: Sender<InputEvent>) -> Result<Self> {
        let tap = crate::platform::macos::input::CGEventTapDevice::new(sender);
        Ok(Self { tap })
    }

    fn run_once(&mut self) -> Result<bool> {
        // CGEventTap runs in its own thread, so this is a no-op
        Ok(true)
    }

    fn stop(&mut self) {
        self.tap.stop();
    }
}

/// macOS-specific input device extension
///
/// This struct wraps the generic InputDevice and adds macOS-specific
/// functionality like wait_for_event and pending event handling.
pub struct MacosInputDeviceExt {
    device: InputDevice<CGEventTapInner>,
    pending_event: std::cell::RefCell<Option<InputEvent>>,
}

impl MacosInputDeviceExt {
    /// Create a new macOS input device with default config
    pub fn new(config: InputDeviceConfig) -> Result<Self> {
        let device = InputDevice::new(config)?;
        Ok(Self {
            device,
            pending_event: std::cell::RefCell::new(None),
        })
    }

    /// Create a macOS input device with custom sender
    pub fn with_sender(event_sender: Sender<InputEvent>) -> Result<Self> {
        let device = InputDevice::with_sender(event_sender)?;
        Ok(Self {
            device,
            pending_event: std::cell::RefCell::new(None),
        })
    }

    /// Get the event sender
    pub fn get_sender(&self) -> Sender<InputEvent> {
        self.device.get_sender()
    }

    /// Get current modifier key state
    pub fn get_modifier_state(&self) -> &ModifierState {
        &self.device.base.modifier_state
    }

    /// Wait for an event with timeout
    pub fn wait_for_event(&mut self, timeout_ms: u64) -> Result<bool, String> {
        if !self.device.base.running {
            self.device.base.running = true;
        }

        #[cfg(not(test))]
        {
            match self
                .device
                .base
                .event_receiver
                .recv_timeout(std::time::Duration::from_millis(timeout_ms))
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

        #[cfg(test)]
        Ok(false)
    }

    /// Run one iteration of the input processing loop
    pub fn run_once(&mut self) -> Result<bool> {
        if let Some(ref mut inner) = self.device.inner {
            inner.run_once()
        } else {
            Ok(true)
        }
    }

    /// Poll event with pending event support
    fn poll_event_with_pending(&mut self) -> Option<InputEvent> {
        if !self.device.base.running {
            return None;
        }

        let pending = {
            let mut borrowed = self.pending_event.borrow_mut();
            borrowed.take()
        };

        if let Some(event) = pending {
            if let InputEvent::Key(key_event) = &event {
                self.device.base.update_modifier_state(
                    key_event.virtual_key,
                    key_event.state == KeyState::Pressed,
                );
            }
            return Some(event);
        }

        self.device.base.try_recv_event()
    }
}

impl InputDeviceTrait for MacosInputDeviceExt {
    fn register(&mut self) -> Result<()> {
        debug!("Registering MacosInputDevice");
        self.device.base.running = true;

        #[cfg(not(test))]
        if crate::platform::macos::input::check_accessibility_permissions() {
            let sender = self.device.base.sender();
            let mut inner = CGEventTapInner::create(sender)?;
            inner.tap.run()?;
            self.device.inner = Some(inner);
            debug!("CGEventTap started");
        } else {
            debug!("Accessibility permissions not granted, using passive mode");
        }

        #[cfg(test)]
        debug!("[TEST MODE] Registering MacosInputDevice (CGEventTap disabled)");

        Ok(())
    }

    fn unregister(&mut self) {
        debug!("Unregistering MacosInputDevice");
        self.device.base.running = false;
        if let Some(ref mut inner) = self.device.inner.take() {
            inner.stop();
        }
    }

    fn poll_event(&mut self) -> Option<InputEvent> {
        self.poll_event_with_pending()
    }

    fn is_running(&self) -> bool {
        self.device.is_running()
    }

    fn stop(&mut self) {
        self.device.stop();
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

    #[test]
    fn test_macos_input_device_creation() {
        let config = InputDeviceConfig::default();
        let device = MacosInputDeviceExt::new(config).unwrap();
        assert!(!device.is_running());
    }

    #[test]
    fn test_macos_input_device_with_sender() {
        let (tx, _rx) = std::sync::mpsc::channel();
        let device = MacosInputDeviceExt::with_sender(tx).unwrap();
        assert!(!device.is_running());
    }
}
