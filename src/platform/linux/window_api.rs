//! Linux window event hook (placeholder)

use crate::platform::traits::WindowEventHookTrait;
use anyhow::Result;

pub struct LinuxWindowEventHook;

impl LinuxWindowEventHook {
    pub fn new(
        _sender: std::sync::mpsc::Sender<crate::platform::traits::PlatformWindowEvent>,
    ) -> Self {
        Self
    }

    pub fn start_with_shutdown(
        &mut self,
        _shutdown_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) -> Result<()> {
        Err(anyhow::anyhow!(
            "Linux window event hook not yet implemented. Wayland toplevel events required."
        ))
    }

    pub fn stop(&mut self) {}

    pub fn shutdown_flag(&self) -> std::sync::Arc<std::sync::atomic::AtomicBool> {
        std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true))
    }
}
