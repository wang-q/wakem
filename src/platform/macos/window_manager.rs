//! macOS window manager implementation
//!
//! Provides comprehensive window management operations on macOS,
//! including half-screen, centering, ratio control, and multi-monitor support.

use crate::platform::macos::window_api::RealWindowApi;
use crate::platform::traits::{
    MonitorInfo, WindowApiBase, WindowFrame, WindowId, WindowInfo,
};
use anyhow::Result;
use tracing::debug;

use crate::platform::window_manager_common::CommonWindowApi;

// Re-export MonitorDirection for consistency with Windows platform
pub use crate::platform::traits::MonitorDirection;

/// macOS window manager using WindowApiBase trait
pub struct WindowManager<A: WindowApiBase<WindowId = WindowId>> {
    api: A,
}

/// Type alias for window manager using real macOS API
pub type RealWindowManager = WindowManager<RealWindowApi>;

impl WindowManager<RealWindowApi> {
    /// Create a window manager using real macOS API
    pub fn new() -> Self {
        Self {
            api: RealWindowApi::new(),
        }
    }
}

impl Default for WindowManager<RealWindowApi> {
    fn default() -> Self {
        Self::new()
    }
}

impl<A: WindowApiBase<WindowId = WindowId>> WindowManager<A> {
    /// Create a window manager with specified API implementation
    #[allow(dead_code)]
    pub fn with_api(api: A) -> Self {
        Self { api }
    }

    /// Get API reference (for testing)
    #[allow(dead_code)]
    pub fn api(&self) -> &A {
        &self.api
    }

    /// Get foreground window information
    #[allow(dead_code)]
    pub fn get_foreground_window_info(&self) -> Result<WindowInfo> {
        let window = self
            .api
            .get_foreground_window()
            .ok_or_else(|| anyhow::anyhow!("No foreground window"))?;
        self.api.get_window_info(window)
    }

    /// Get specified window information
    pub fn get_window_info(&self, window: WindowId) -> Result<WindowInfo> {
        self.api.get_window_info(window)
    }

    /// Get debug info string
    #[allow(dead_code)]
    pub fn get_debug_info(&self) -> Result<String> {
        let info = self.get_foreground_window_info()?;

        Ok(format!(
            "Window: {}\nID: {}\nPosition: [{}, {}]\nSize: {} x {}",
            info.title, info.id, info.x, info.y, info.width, info.height
        ))
    }

    /// Minimize window
    pub fn minimize_window(&self, window: WindowId) -> Result<()> {
        self.api.minimize_window(window)
    }

    /// Maximize window
    pub fn maximize_window(&self, window: WindowId) -> Result<()> {
        self.api.maximize_window(window)
    }

    /// Restore window
    pub fn restore_window(&self, window: WindowId) -> Result<()> {
        self.api.restore_window(window)
    }

    /// Close window
    pub fn close_window(&self, window: WindowId) -> Result<()> {
        self.api.close_window(window)
    }
}

impl WindowManager<RealWindowApi> {
    /// Set window position and size (with ensure restored for RealWindowApi)
    #[allow(dead_code)]
    pub fn set_window_frame(&self, window: WindowId, frame: &WindowFrame) -> Result<()> {
        self.api.ensure_window_restored(window)?;
        self.api
            .set_window_pos(window, frame.x, frame.y, frame.width, frame.height)?;

        debug!(
            "Window moved to: x={}, y={}, width={}, height={}",
            frame.x, frame.y, frame.width, frame.height
        );

        Ok(())
    }
}

// Implement CommonWindowApi for WindowManager to use common window manager logic
impl<A: WindowApiBase<WindowId = WindowId> + 'static> CommonWindowApi for WindowManager<A> {
    type WindowId = WindowId;
    type WindowInfo = WindowInfo;

    fn api(&self) -> &dyn std::any::Any {
        self
    }

    fn get_window_info(&self, window: Self::WindowId) -> Result<Self::WindowInfo> {
        self.api.get_window_info(window)
    }

    fn set_window_pos(
        &self,
        window: Self::WindowId,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<()> {
        self.api.set_window_pos(window, x, y, width, height)
    }

    fn get_monitors(&self) -> Vec<MonitorInfo> {
        self.api.get_monitors()
    }

    fn is_window_valid(&self, window: Self::WindowId) -> bool {
        self.api.is_window_valid(window)
    }

    fn is_maximized(&self, window: Self::WindowId) -> bool {
        self.api.is_maximized(window)
    }

    fn is_topmost(&self, window: Self::WindowId) -> bool {
        self.api.is_topmost(window)
    }

    fn set_topmost(&self, window: Self::WindowId, topmost: bool) -> Result<()> {
        self.api.set_topmost(window, topmost)
    }
}

/// Features requiring real macOS API (cross-monitor movement, window switching, etc.)
impl RealWindowManager {
    /// Move window to another monitor
    pub fn move_to_monitor(
        &self,
        window: WindowId,
        direction: MonitorDirection,
    ) -> Result<()> {
        use crate::platform::window_manager_common::CommonWindowManager;
        CommonWindowManager::move_to_monitor(self, window, direction)
    }

    /// Switch to next window of same application
    #[cfg(not(test))]
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

    #[cfg(test)]
    pub fn switch_to_next_window_of_same_process(&self) -> Result<()> {
        debug!("[TEST MODE] switch_to_next_window_of_same_process called");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::macos::window_api::MockWindowApi;
    use crate::types::{Alignment, Edge};

    #[test]
    fn test_window_manager_creation() {
        let api = MockWindowApi::new();
        let wm = WindowManager::with_api(api);

        // Verify creation success
        assert!(!wm.api().is_window_valid(0));
    }

    #[test]
    fn test_get_window_info() {
        let api = MockWindowApi::new();

        // Set test data
        api.add_window(
            2,
            WindowInfo {
                id: 2,
                title: "Test Window".to_string(),
                process_name: "TestApp".to_string(),
                executable_path: None,
                x: 100,
                y: 200,
                width: 800,
                height: 600,
            },
        );

        let wm = WindowManager::with_api(api);
        let info = wm.get_window_info(2).unwrap();

        assert_eq!(info.x, 100);
        assert_eq!(info.y, 200);
        assert_eq!(info.width, 800);
        assert_eq!(info.height, 600);
    }

    #[test]
    fn test_move_to_center() {
        let api = MockWindowApi::new();

        // Set test data - 800x600 window on 1920x1080 monitor
        api.add_window(
            2,
            WindowInfo {
                id: 2,
                title: "Test Window".to_string(),
                process_name: "TestApp".to_string(),
                executable_path: None,
                x: 0,
                y: 0,
                width: 800,
                height: 600,
            },
        );

        let wm = WindowManager::with_api(api);
        CommonWindowApi::move_to_center(&wm, 2).unwrap();

        // Verify window position (1920-800)/2 = 560, (1080-600)/2 = 240
        let info = wm.get_window_info(2).unwrap();
        assert_eq!(info.x, 560);
        assert_eq!(info.y, 240);
    }

    #[test]
    fn test_move_to_edge() {
        let api = MockWindowApi::new();

        api.add_window(
            2,
            WindowInfo {
                id: 2,
                title: "Test Window".to_string(),
                process_name: "TestApp".to_string(),
                executable_path: None,
                x: 100,
                y: 100,
                width: 800,
                height: 600,
            },
        );

        let wm = WindowManager::with_api(api);

        // Test left edge
        CommonWindowApi::move_to_edge(&wm, 2, Edge::Left).unwrap();
        let info = wm.get_window_info(2).unwrap();
        assert_eq!(info.x, 0);

        // Test right edge
        CommonWindowApi::move_to_edge(&wm, 2, Edge::Right).unwrap();
        let info = wm.get_window_info(2).unwrap();
        assert_eq!(info.x, 1920 - 800);
    }

    #[test]
    fn test_set_half_screen() {
        let api = MockWindowApi::new();

        api.add_window(
            2,
            WindowInfo {
                id: 2,
                title: "Test Window".to_string(),
                process_name: "TestApp".to_string(),
                executable_path: None,
                x: 100,
                y: 100,
                width: 800,
                height: 600,
            },
        );

        let wm = WindowManager::with_api(api);

        // Test left half screen
        CommonWindowApi::set_half_screen(&wm, 2, Edge::Left).unwrap();
        let info = wm.get_window_info(2).unwrap();
        assert_eq!(info.x, 0);
        assert_eq!(info.y, 0);
        assert_eq!(info.width, 960); // 1920 / 2
        assert_eq!(info.height, 1080);

        // Test right half screen
        CommonWindowApi::set_half_screen(&wm, 2, Edge::Right).unwrap();
        let info = wm.get_window_info(2).unwrap();
        assert_eq!(info.x, 960);
        assert_eq!(info.width, 960);
    }

    #[test]
    fn test_loop_width() {
        let api = MockWindowApi::new();

        // Set all data before creating WindowManager
        api.add_window(
            2,
            WindowInfo {
                id: 2,
                title: "Test Window".to_string(),
                process_name: "TestApp".to_string(),
                executable_path: None,
                x: 0,
                y: 0,
                width: 960,
                height: 600,
            },
        );

        let wm = WindowManager::with_api(api);

        // Test cycle from 50%
        CommonWindowApi::loop_width(&wm, 2, Alignment::Left).unwrap();

        let info = wm.get_window_info(2).unwrap();
        // 50% -> 40% = 768
        assert_eq!(info.width, 768);
    }

    #[test]
    fn test_set_fixed_ratio() {
        let api = MockWindowApi::new();

        // Set all data before creating WindowManager
        api.add_window(
            2,
            WindowInfo {
                id: 2,
                title: "Test Window".to_string(),
                process_name: "TestApp".to_string(),
                executable_path: None,
                x: 100,
                y: 100,
                width: 800,
                height: 600,
            },
        );

        let wm = WindowManager::with_api(api);

        // Test 4:3 ratio, 100% scale
        CommonWindowApi::set_fixed_ratio(&wm, 2, 4.0 / 3.0).unwrap();

        let info = wm.get_window_info(2).unwrap();
        // Based on smaller side 1080, 4:3 ratio, width = 1080 * 4/3 = 1440
        assert_eq!(info.width, 1440);
        assert_eq!(info.height, 1080);
    }

    #[test]
    fn test_window_state_operations() {
        let api = MockWindowApi::new();

        api.add_window(
            2,
            WindowInfo {
                id: 2,
                title: "Test Window".to_string(),
                process_name: "TestApp".to_string(),
                executable_path: None,
                x: 100,
                y: 100,
                width: 800,
                height: 600,
            },
        );

        let wm = WindowManager::with_api(api);

        // Test minimize
        wm.minimize_window(2).unwrap();
        assert!(wm.api().is_minimized(2));

        // Test restore
        wm.restore_window(2).unwrap();
        assert!(!wm.api().is_minimized(2));

        // Test maximize
        wm.maximize_window(2).unwrap();
        assert!(wm.api().is_maximized(2));
    }

    #[test]
    fn test_close_window() {
        let api = MockWindowApi::new();

        api.add_window(
            2,
            WindowInfo {
                id: 2,
                title: "Test Window".to_string(),
                process_name: "TestApp".to_string(),
                executable_path: None,
                x: 100,
                y: 100,
                width: 800,
                height: 600,
            },
        );

        let wm = WindowManager::with_api(api);
        assert!(wm.api().is_window_valid(2));

        wm.close_window(2).unwrap();

        // Window should be removed
        assert!(!wm.api().is_window_valid(2));
    }

    #[test]
    fn test_switch_same_process() {
        // switch_to_next_window_of_same_process is only available on RealWindowApi
        // This test just verifies the method exists on RealWindowManager
        let _wm = WindowManager::new();
        // Cannot call switch_to_next_window_of_same_process on MockWindowApi
        // as it requires real macOS API access
    }
}
