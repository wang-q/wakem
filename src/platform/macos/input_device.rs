//! macOS input device implementation using CGEventTap
//!
//! Wraps the low-level CGEventTap device from [crate::platform::macos::input]
//! behind the [InputDeviceTrait] interface. Uses the generic [InputDevice]
//! from [input_device_common] to share code with Windows implementation.

use crate::platform::input_device_common::{InputDevice, PlatformInputDevice};
use crate::platform::traits::InputDeviceTrait;
use crate::types::InputEvent;
use anyhow::Result;
use std::sync::mpsc::Sender;
use tracing::debug;

/// macOS CGEventTap device type alias
pub type RawInputDevice = InputDevice<CGEventTapInner>;

/// Inner CGEventTap device from the low-level module
pub struct CGEventTapInner {
    tap: crate::platform::macos::input::CGEventTapDevice,
}

impl PlatformInputDevice for CGEventTapInner {
    fn create(sender: Sender<InputEvent>) -> Result<Self> {
        let tap = crate::platform::macos::input::CGEventTapDevice::new(sender);
        Ok(Self { tap })
    }

    fn register(&mut self) -> Result<()> {
        self.tap.run()
    }

    fn run_once(&mut self) -> Result<bool> {
        Ok(true)
    }

    fn stop(&mut self) {
        self.tap.stop();
    }
}

unsafe impl Send for CGEventTapInner {}

impl InputDeviceTrait for InputDevice<CGEventTapInner> {
    fn register(&mut self) -> Result<()> {
        debug!("Registering CGEventTap device");

        #[cfg(not(test))]
        if crate::platform::macos::input::check_accessibility_permissions() {
            self.register_inner()?;
            debug!("CGEventTap started");
        } else {
            debug!("Accessibility permissions not granted, using passive mode");
            self.base.running = true;
        }

        #[cfg(test)]
        {
            debug!("[TEST MODE] Registering CGEventTap device (disabled)");
            self.base.running = true;
        }

        Ok(())
    }

    fn unregister(&mut self) {
        debug!("Unregistering CGEventTap device");
        self.base.running = false;
        if let Some(mut inner) = self.inner.take() {
            inner.stop();
        }
    }

    fn poll_event(&mut self) -> Option<InputEvent> {
        self.poll_event_inner()
    }

    fn is_running(&self) -> bool {
        self.base.is_running()
    }

    fn stop(&mut self) {
        self.stop_inner();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::traits::InputDeviceConfig;

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
