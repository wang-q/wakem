//! macOS window manager implementation
//!
//! Provides window management operations on macOS.
#![cfg(target_os = "macos")]

use crate::platform::macos::window_api::RealMacosWindowApi;
use crate::platform::traits::{
    MonitorInfo, WindowId, WindowInfo, WindowManager as WindowManagerTrait,
};
use crate::platform::types::WindowFrame;
use anyhow::Result;
use tracing::debug;

/// Monitor direction (for moving between displays)
#[derive(Debug, Clone, Copy)]
pub enum MonitorDirection {
    Next,
    Prev,
    Index(i32),
}

/// macOS window manager
#[derive(Clone)]
pub struct WindowManager {
    api: RealMacosWindowApi,
}

impl WindowManager {
    pub fn new() -> Self {
        Self {
            api: RealMacosWindowApi::new(),
        }
    }

    pub fn api(&self) -> &RealMacosWindowApi {
        &self.api
    }

    /// Switch to next window of same application
    pub fn switch_to_next_window_of_same_process(&self) -> Result<()> {
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

impl WindowManagerTrait for WindowManager {
    fn get_foreground_window(&self) -> Option<WindowId> {
        self.api.get_foreground_window()
    }

    fn get_window_info(&self, window: WindowId) -> Result<WindowInfo> {
        self.api.get_window_info(window)
    }

    fn set_window_pos(
        &self,
        window: WindowId,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<()> {
        self.api.set_window_pos(window, x, y, width, height)
    }

    fn minimize_window(&self, window: WindowId) -> Result<()> {
        self.api.minimize_window(window)
    }

    fn maximize_window(&self, window: WindowId) -> Result<()> {
        self.api.maximize_window(window)
    }

    fn restore_window(&self, window: WindowId) -> Result<()> {
        self.api.restore_window(window)
    }

    fn close_window(&self, window: WindowId) -> Result<()> {
        self.api.close_window(window)
    }

    fn set_topmost(&self, window: WindowId, topmost: bool) -> Result<()> {
        self.api.set_topmost(window, topmost)
    }

    fn is_topmost(&self, window: WindowId) -> bool {
        self.api.is_topmost(window)
    }

    fn is_window_valid(&self, window: WindowId) -> bool {
        self.api.is_window_valid(window)
    }

    fn is_minimized(&self, window: WindowId) -> bool {
        self.api.is_minimized(window)
    }

    fn is_maximized(&self, window: WindowId) -> bool {
        self.api.is_maximized(window)
    }

    fn get_monitors(&self) -> Vec<MonitorInfo> {
        self.api.get_monitors()
    }

    fn move_to_monitor(&self, window: WindowId, monitor_index: usize) -> Result<()> {
        self.api.move_to_monitor(window, monitor_index)
    }

    fn switch_to_next_window_of_same_process(&self) -> Result<()> {
        self.switch_to_next_window_of_same_process()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::macos::window_api::MockMacosWindowApi;

    #[test]
    fn test_window_manager_creation() {
        let mock = MockMacosWindowApi::new();
        let _wm = WindowManager { api: mock };
    }
}
