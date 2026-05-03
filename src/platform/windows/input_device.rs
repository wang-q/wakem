//! Windows input device implementation
#![cfg(target_os = "windows")]

use crate::platform::traits::InputDevice;
use crate::platform::types::InputDeviceConfig;
use crate::types::InputEvent;
use anyhow::Result;
use std::sync::mpsc::{channel, Receiver, Sender};
use tracing::debug;

/// Windows Raw Input device
pub struct RawInputDevice {
    inner: Option<crate::platform::windows::input::RawInputDevice>,
    receiver: Receiver<InputEvent>,
    sender: Sender<InputEvent>,
    running: bool,
}

impl RawInputDevice {
    /// Create a new input device with default configuration
    pub fn new(_config: InputDeviceConfig) -> Result<Self> {
        let (sender, receiver) = channel();
        Ok(Self {
            inner: None,
            receiver,
            sender,
            running: false,
        })
    }

    /// Create a new input device with a sender
    pub fn with_sender(sender: Sender<InputEvent>) -> Result<Self> {
        let (_, receiver) = channel();
        Ok(Self {
            inner: None,
            receiver,
            sender,
            running: false,
        })
    }

    /// Run one iteration of the input processing loop
    /// Returns Ok(true) if should continue, Ok(false) if shutdown requested
    pub fn run_once(&mut self) -> Result<bool> {
        if let Some(ref mut inner) = self.inner {
            inner.run_once()
        } else {
            // If not registered, just sleep briefly to avoid busy loop
            std::thread::sleep(std::time::Duration::from_millis(1));
            Ok(true)
        }
    }
}

impl InputDevice for RawInputDevice {
    fn register(&mut self) -> Result<()> {
        debug!("Registering Raw Input device");

        let inner =
            crate::platform::windows::input::RawInputDevice::new(self.sender.clone())?;
        self.inner = Some(inner);
        self.running = true;

        Ok(())
    }

    fn unregister(&mut self) {
        debug!("Unregistering Raw Input device");
        self.running = false;
        if let Some(mut inner) = self.inner.take() {
            inner.stop();
        }
    }

    fn poll_event(&mut self) -> Option<InputEvent> {
        if !self.running {
            return None;
        }

        if let Some(ref mut inner) = self.inner {
            let _ = inner.run_once();
        }

        self.receiver.try_recv().ok()
    }

    fn is_running(&self) -> bool {
        self.running
    }

    fn stop(&mut self) {
        self.running = false;
        if let Some(mut inner) = self.inner.take() {
            inner.stop();
        }
    }
}

// SAFETY: RawInputDevice contains an HWND which is not Send by default.
// However, the device is only used on the thread that created it.
unsafe impl Send for RawInputDevice {}

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
