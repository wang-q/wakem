//! Windows input device implementation
#![cfg(target_os = "windows")]

use crate::platform::common::input_device::{
    InputDevice as GenericInputDevice, PlatformInputDevice,
};
use crate::platform::traits::InputDevice;
use crate::platform::types::InputDeviceConfig;
use crate::types::InputEvent;
use anyhow::Result;
use std::sync::mpsc::Sender;
use tracing::debug;

pub struct RawInputDeviceInner {
    device: crate::platform::windows::input::RawInputDevice,
}

impl PlatformInputDevice for RawInputDeviceInner {
    fn create(sender: Sender<InputEvent>) -> Result<Self> {
        let device = crate::platform::windows::input::RawInputDevice::new(sender)?;
        Ok(Self { device })
    }

    fn run_once(&mut self) -> Result<bool> {
        self.device.run_once()
    }

    fn stop(&mut self) {
        self.device.stop();
    }
}

// SAFETY: RawInputDeviceInner contains an HWND which is not Send by default.
// However, the device is only used on the thread that created it.
unsafe impl Send for RawInputDeviceInner {}

pub struct RawInputDevice {
    device: GenericInputDevice<RawInputDeviceInner>,
}

impl RawInputDevice {
    pub fn new(config: InputDeviceConfig) -> Result<Self> {
        let device = GenericInputDevice::new(config)?;
        Ok(Self { device })
    }

    pub fn with_sender(sender: Sender<InputEvent>) -> Result<Self> {
        let device = GenericInputDevice::with_sender(sender)?;
        Ok(Self { device })
    }

    pub fn run_once(&mut self) -> Result<bool> {
        if let Some(ref mut inner) = self.device.inner {
            inner.run_once()
        } else {
            std::thread::sleep(std::time::Duration::from_millis(1));
            Ok(true)
        }
    }
}

impl InputDevice for RawInputDevice {
    fn register(&mut self) -> Result<()> {
        debug!("Registering Raw Input device");
        self.device.base.running = true;
        let sender = self.device.base.sender();
        let inner = RawInputDeviceInner::create(sender)?;
        self.device.inner = Some(inner);
        Ok(())
    }

    fn unregister(&mut self) {
        debug!("Unregistering Raw Input device");
        self.device.base.running = false;
        if let Some(ref mut inner) = self.device.inner.take() {
            inner.stop();
        }
    }

    fn poll_event(&mut self) -> Option<InputEvent> {
        if !self.device.base.running {
            return None;
        }

        if let Some(ref mut inner) = self.device.inner {
            let _ = inner.run_once();
        }

        self.device.base.try_recv_event()
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
