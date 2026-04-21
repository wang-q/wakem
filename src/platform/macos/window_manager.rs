//! macOS window manager implementation

use crate::platform::macos::window_api::MacosWindowApi;
use crate::platform::traits::{
    MonitorInfo, WindowApiTrait, WindowId, WindowInfo, WindowManagerTrait,
};
use anyhow::Result;

/// macOS window manager
pub struct MacosWindowManager {
    api: MacosWindowApi,
}

impl MacosWindowManager {
    /// Create a new macOS window manager
    pub fn new() -> Self {
        Self {
            api: MacosWindowApi::new(),
        }
    }

    /// Create with custom API (for testing)
    pub fn with_api(api: MacosWindowApi) -> Self {
        Self { api }
    }
}

impl Default for MacosWindowManager {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowManagerTrait for MacosWindowManager {
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

    fn set_opacity(&self, window: WindowId, opacity: u8) -> Result<()> {
        self.api.set_opacity(window, opacity)
    }

    fn get_monitors(&self) -> Vec<MonitorInfo> {
        self.api.get_monitors()
    }

    fn move_to_monitor(&self, window: WindowId, monitor_index: usize) -> Result<()> {
        self.api.move_to_monitor(window, monitor_index)
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
}

/// Additional macOS-specific window operations
impl MacosWindowManager {
    /// Move window to center of screen
    pub fn move_to_center(&self, window: WindowId) -> Result<()> {
        let info = self.get_window_info(window)?;
        let monitors = self.get_monitors();

        if let Some(monitor) = monitors.first() {
            let new_x = monitor.x + (monitor.width - info.width) / 2;
            let new_y = monitor.y + (monitor.height - info.height) / 2;
            self.set_window_pos(window, new_x, new_y, info.width, info.height)?;
        }

        Ok(())
    }

    /// Set window to half screen
    pub fn set_half_screen(
        &self,
        window: WindowId,
        edge: crate::types::Edge,
    ) -> Result<()> {
        let info = self.get_window_info(window)?;
        let monitors = self.get_monitors();

        if let Some(monitor) = monitors.first() {
            let (new_x, new_y, new_width, new_height) = match edge {
                crate::types::Edge::Left => {
                    (monitor.x, monitor.y, monitor.width / 2, monitor.height)
                }
                crate::types::Edge::Right => {
                    let width = monitor.width / 2;
                    (
                        monitor.x + monitor.width - width,
                        monitor.y,
                        width,
                        monitor.height,
                    )
                }
                crate::types::Edge::Top => {
                    (monitor.x, monitor.y, monitor.width, monitor.height / 2)
                }
                crate::types::Edge::Bottom => {
                    let height = monitor.height / 2;
                    (
                        monitor.x,
                        monitor.y + monitor.height - height,
                        monitor.width,
                        height,
                    )
                }
            };

            self.set_window_pos(window, new_x, new_y, new_width, new_height)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macos_window_manager_creation() {
        let manager = MacosWindowManager::new();
        drop(manager);
    }
}
