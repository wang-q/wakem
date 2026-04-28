//! Linux output device (placeholder)

use crate::platform::traits::OutputDeviceTrait;
use anyhow::Result;

pub struct LinuxOutputDevice;

impl LinuxOutputDevice {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LinuxOutputDevice {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputDeviceTrait for LinuxOutputDevice {
    fn send_key(&self, _scan_code: u16, _virtual_key: u16, _release: bool) -> Result<()> {
        Err(anyhow::anyhow!(
            "Linux output device not yet implemented. Wayland virtual input required."
        ))
    }

    fn send_mouse_move(&self, _x: i32, _y: i32, _relative: bool) -> Result<()> {
        Err(anyhow::anyhow!(
            "Linux output device not yet implemented. Wayland virtual input required."
        ))
    }

    fn send_mouse_button(
        &self,
        _button: crate::types::MouseButton,
        _release: bool,
    ) -> Result<()> {
        Err(anyhow::anyhow!(
            "Linux output device not yet implemented. Wayland virtual input required."
        ))
    }

    fn send_mouse_wheel(&self, _delta: i32, _horizontal: bool) -> Result<()> {
        Err(anyhow::anyhow!(
            "Linux output device not yet implemented. Wayland virtual input required."
        ))
    }
}
