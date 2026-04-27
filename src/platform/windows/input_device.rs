//! Windows input device implementation
//!
//! Wraps the low-level Raw Input device from [crate::platform::windows::input]
//! behind the [InputDeviceTrait] interface. Uses the generic [InputDevice]
//! from [input_device_common] to share code with macOS implementation.
#![cfg(target_os = "windows")]

use crate::platform::input_device_common::{InputDevice, PlatformInputDevice};
use crate::platform::traits::InputDeviceTrait;
use crate::types::InputEvent;
use anyhow::Result;
use std::sync::mpsc::Sender;
use tracing::debug;

/// Windows Raw Input device type alias
pub type RawInputDevice = InputDevice<RawInputInner>;

/// Inner Raw Input device from the low-level module
pub struct RawInputInner {
    inner: crate::platform::windows::input::RawInputDevice,
}

impl PlatformInputDevice for RawInputInner {
    fn create(sender: Sender<InputEvent>) -> Result<Self> {
        let inner = crate::platform::windows::input::RawInputDevice::new(sender)?;
        Ok(Self { inner })
    }

    fn register(&mut self) -> Result<()> {
        // Windows Raw Input device is already registered in ::new()
        // Just mark as running
        Ok(())
    }

    fn run_once(&mut self) -> Result<bool> {
        self.inner.run_once()
    }

    fn stop(&mut self) {
        self.inner.stop();
    }
}

// SAFETY: RawInputInner contains an HWND which is not Send by default.
// However, the device is only used on the thread that created it.
unsafe impl Send for RawInputInner {}

impl InputDeviceTrait for InputDevice<RawInputInner> {
    fn register(&mut self) -> Result<()> {
        debug!("Registering Raw Input device");
        self.register_inner()
    }

    fn unregister(&mut self) {
        debug!("Unregistering Raw Input device");
        self.unregister_inner();
    }

    fn poll_event(&mut self) -> Option<InputEvent> {
        self.poll_event_inner()
    }

    fn is_running(&self) -> bool {
        self.is_running_inner()
    }

    fn stop(&mut self) {
        self.stop_inner();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::traits::InputDeviceConfig;
    use crate::platform::traits::InputDeviceTrait;

    #[test]
    fn test_raw_input_device_creation() {
        let config = InputDeviceConfig::default();
        let device = RawInputDevice::new(config).unwrap();
        assert!(!device.is_running());
    }

    #[test]
    fn test_raw_input_device_with_sender() {
        let (tx, _rx) = std::sync::mpsc::channel();
        let device = RawInputDevice::with_sender(tx).unwrap();
        assert!(!device.is_running());
    }

    #[test]
    fn test_raw_input_device_poll_when_not_running() {
        let config = InputDeviceConfig::default();
        let mut device = RawInputDevice::new(config).unwrap();
        assert!(device.poll_event().is_none());
    }
}
