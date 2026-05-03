//! macOS window manager implementation
//!
//! Provides window management operations on macOS.
#![cfg(target_os = "macos")]

use crate::platform::macos::window_api::RealMacosWindowApi;
use crate::platform::traits::WindowSwitching;
use anyhow::Result;
use tracing::debug;

pub type WindowManager =
    crate::platform::common::window_manager::WindowManager<RealMacosWindowApi>;

impl WindowManager {
    pub fn new() -> Self {
        Self::with_api(RealMacosWindowApi::new())
    }

    fn switch_to_next_window_of_same_process_inner(&self) -> Result<()> {
        use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation};
        use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

        let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
            .map_err(|e| anyhow::anyhow!("Failed to create event source: {:?}", e))?;

        let key_down = CGEvent::new_keyboard_event(source.clone(), 50, true)
            .map_err(|e| anyhow::anyhow!("Failed to create key down event: {:?}", e))?;
        key_down.set_flags(CGEventFlags::CGEventFlagCommand);
        key_down.post(CGEventTapLocation::HID);

        let key_up = CGEvent::new_keyboard_event(source, 50, false)
            .map_err(|e| anyhow::anyhow!("Failed to create key up event: {:?}", e))?;
        key_up.set_flags(CGEventFlags::CGEventFlagCommand);
        key_up.post(CGEventTapLocation::HID);

        debug!("Switched to next window of same process (using CGEvent)");
        Ok(())
    }
}

impl Default for WindowManager {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowSwitching for WindowManager {
    fn switch_to_next_window_of_same_process(&self) -> Result<()> {
        self.switch_to_next_window_of_same_process_inner()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::macos::window_api::MockMacosWindowApi;

    #[test]
    fn test_window_manager_creation() {
        let _wm = WindowManager::with_api(MockMacosWindowApi::new());
    }
}
