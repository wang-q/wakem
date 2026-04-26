//! macOS input device implementation using CGEventTap
//!
//! This module provides a macOS input device using CGEventTap.
//! Uses the generic [InputDevice] from [input_device_common] to share code
//! with the Windows implementation.
#![cfg(target_os = "macos")]

use crate::platform::input_device_common::{
    InputDevice, InputDeviceBase, PlatformInputDevice,
};
use crate::platform::traits::{InputDeviceConfig, InputDeviceTrait};
use crate::types::{InputEvent, ModifierState};
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

impl InputDevice<CGEventTapInner> {
    /// Get current modifier key state
    pub fn get_modifier_state(&self) -> &ModifierState {
        &self.base.modifier_state
    }

    /// Wait for an event with timeout
    pub fn wait_for_event(&mut self, timeout_ms: u64) -> Result<bool, String> {
        if !self.base.running {
            self.base.running = true;
        }

        #[cfg(not(test))]
        {
            match self
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
}

impl InputDeviceTrait for InputDevice<CGEventTapInner> {
    fn register(&mut self) -> Result<()> {
        debug!("Registering MacosInputDevice");
        self.base.running = true;

        #[cfg(not(test))]
        if crate::platform::macos::input::check_accessibility_permissions() {
            let sender = self.base.sender();
            let inner = CGEventTapInner::create(sender)?;
            inner.tap.run()?;
            self.inner = Some(inner);
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
        self.base.running = false;
        if let Some(ref mut inner) = self.inner.take() {
            inner.stop();
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
        if let Some(ref mut inner) = self.inner.take() {
            inner.stop();
        }
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
        let device = MacosInputDevice::new(config).unwrap();
        assert!(!device.is_running());
    }

    #[test]
    fn test_macos_input_device_with_sender() {
        let (tx, _rx) = std::sync::mpsc::channel();
        let device = MacosInputDevice::with_sender(tx).unwrap();
        assert!(!device.is_running());
    }
}
