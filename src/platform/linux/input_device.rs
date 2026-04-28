//! Linux input device (placeholder)

use crate::platform::traits::InputDeviceTrait;
use anyhow::Result;

pub struct LinuxInputDevice;

impl LinuxInputDevice {
    pub fn new(_config: crate::platform::traits::InputDeviceConfig) -> Result<Self> {
        Ok(Self)
    }

    pub fn with_sender(
        _sender: std::sync::mpsc::Sender<crate::types::InputEvent>,
    ) -> Result<Self> {
        Ok(Self)
    }
}

impl InputDeviceTrait for LinuxInputDevice {
    fn register(&mut self) -> Result<()> {
        Err(anyhow::anyhow!(
            "Linux input device not yet implemented. Wayland/EVDEV support required."
        ))
    }

    fn unregister(&mut self) {}

    fn poll_event(&mut self) -> Option<crate::types::InputEvent> {
        None
    }

    fn is_running(&self) -> bool {
        false
    }

    fn stop(&mut self) {}
}
