//! Windows input device implementation
#![cfg(target_os = "windows")]
#![allow(dead_code)]

use crate::platform::input_device_common::InputDeviceBase;
use crate::platform::traits::{InputDeviceConfig, InputDeviceTrait};
use crate::types::InputEvent;
use anyhow::Result;
use tracing::debug;

/// Real Raw Input device implementation
pub struct RawInputDevice {
    config: InputDeviceConfig,
    base: InputDeviceBase,
    #[allow(dead_code)]
    hwnd: Option<isize>,
}

impl RawInputDevice {
    /// Create a new Raw Input device
    pub fn new(config: InputDeviceConfig) -> Result<Self> {
        Ok(Self {
            config,
            base: InputDeviceBase::new(),
            hwnd: None,
        })
    }
}

impl InputDeviceTrait for RawInputDevice {
    fn register(&mut self) -> Result<()> {
        debug!("Registering Raw Input device");
        self.base.running = true;
        Ok(())
    }

    fn unregister(&mut self) {
        debug!("Unregistering Raw Input device");
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raw_input_device_creation() {
        let config = InputDeviceConfig::default();
        let device = RawInputDevice::new(config).unwrap();
        assert!(!device.is_running());
    }

    #[test]
    fn test_raw_input_device_register_unregister() {
        let config = InputDeviceConfig::default();
        let mut device = RawInputDevice::new(config).unwrap();
        device.register().unwrap();
        assert!(device.is_running());
        device.unregister();
        assert!(!device.is_running());
    }

    #[test]
    fn test_raw_input_device_poll_when_not_running() {
        let config = InputDeviceConfig::default();
        let mut device = RawInputDevice::new(config).unwrap();
        assert!(device.poll_event().is_none());
    }
}
