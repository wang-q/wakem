//! Windows input device implementation
//!
//! Wraps the low-level Raw Input device from [crate::platform::windows::input]
//! behind the [InputDeviceTrait] interface. Events captured by the underlying
//! Raw Input window are forwarded through an mpsc channel so that
//! [InputDeviceTrait::poll_event] can consume them.
#![cfg(target_os = "windows")]

use crate::platform::input_device_common::InputDeviceBase;
use crate::platform::traits::{InputDeviceConfig, InputDeviceTrait};
use crate::types::InputEvent;
use anyhow::Result;
use std::sync::mpsc::Sender;
use tracing::debug;

pub struct RawInputDevice {
    #[allow(dead_code)]
    config: InputDeviceConfig,
    base: InputDeviceBase,
    #[allow(dead_code)]
    inner: Option<crate::platform::windows::input::RawInputDevice>,
}

// SAFETY: RawInputDevice contains an HWND which is not Send by default.
// However, the RawInputDevice is only used on the thread that created it
// (the message window is thread-affine). The InputDeviceTrait requires Send
// for storage in cross-thread containers, but actual event processing
// always happens on the creating thread.
unsafe impl Send for RawInputDevice {}

#[allow(dead_code)]
impl RawInputDevice {
    pub fn new(config: InputDeviceConfig) -> Result<Self> {
        let base = InputDeviceBase::new();
        Ok(Self {
            config,
            base,
            inner: None,
        })
    }

    pub fn with_sender(event_sender: Sender<InputEvent>) -> Result<Self> {
        let base = InputDeviceBase::with_sender(event_sender.clone());
        Ok(Self {
            config: InputDeviceConfig::default(),
            base,
            inner: None,
        })
    }

    pub fn get_sender(&self) -> Sender<InputEvent> {
        self.base.sender()
    }
}

impl InputDeviceTrait for RawInputDevice {
    fn register(&mut self) -> Result<()> {
        debug!("Registering Raw Input device");

        let sender = self.base.sender();
        let inner = crate::platform::windows::input::RawInputDevice::new(sender)?;
        self.inner = Some(inner);

        self.base.running = true;
        Ok(())
    }

    fn unregister(&mut self) {
        debug!("Unregistering Raw Input device");
        self.base.running = false;
        if let Some(mut inner) = self.inner.take() {
            inner.stop();
        }
    }

    fn poll_event(&mut self) -> Option<InputEvent> {
        if !self.base.running {
            return None;
        }

        if let Some(ref mut inner) = self.inner {
            let _ = inner.run_once();
        }

        self.base.try_recv_event()
    }

    fn is_running(&self) -> bool {
        self.base.is_running()
    }

    fn stop(&mut self) {
        self.base.stop();
        if let Some(mut inner) = self.inner.take() {
            inner.stop();
        }
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
