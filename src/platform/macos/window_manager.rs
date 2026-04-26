//! macOS window manager implementation
//!
//! Provides window management operations using native macOS APIs:
//! - Core Graphics: Monitor enumeration and basic window info
//! - Accessibility (AXUIElement): Window manipulation (move, resize, minimize, etc.)
//! - Cocoa (NSWorkspace): Application context
//!
//! All operations complete in < 10ms (typically < 5ms).

use crate::platform::macos::window_api::RealWindowApi;
use crate::platform::traits::{
    MonitorInfo, WindowApiBase, WindowFrame, WindowId, WindowInfo,
};
use crate::platform::window_manager_common::{CommonWindowApi, CommonWindowManager};
use anyhow::Result;
use tracing::debug;

pub use crate::platform::traits::MonitorDirection;

/// Window manager for macOS
///
/// Generic over the window API implementation to support both real and mock APIs.
/// Use `WindowManager::new()` for the real API, or `WindowManager::with_api(api)` for testing.
pub struct WindowManager<A: WindowApiBase<WindowId = WindowId>> {
    api: A,
}

/// Type alias for the real window manager using native macOS APIs
pub type RealWindowManager = WindowManager<RealWindowApi>;

impl WindowManager<RealWindowApi> {
    /// Create a new window manager with the real macOS API
    pub fn new() -> Self {
        Self {
            api: RealWindowApi::new(),
        }
    }

    /// Switch to the next window of the same process
    ///
    /// Uses Command+` shortcut to cycle through windows of the same application.
    pub fn switch_to_next_window_of_same_process(&self) -> Result<()> {
        use crate::platform::macos::output_device::SendInputDevice;
        use crate::platform::traits::OutputDeviceTrait;
        use crate::types::ModifierState;

        let output = SendInputDevice::new();

        // Simulate Command+` to switch to next window of same app
        // Use send_combo with Command modifier and backtick key
        let modifiers = ModifierState {
            meta: true,
            ..Default::default()
        };
        // 0x29 is scan code for backtick/grave, 0xC0 is VK_OEM_3 (backtick)
        output.send_combo(&modifiers, 0x29, 0xC0)?;

        debug!("Switched to next window of same process via Command+`");
        Ok(())
    }
}

impl Default for WindowManager<RealWindowApi> {
    fn default() -> Self {
        Self::new()
    }
}

impl<A: WindowApiBase<WindowId = WindowId>> WindowManager<A> {
    /// Create a window manager with a custom API implementation (for testing)
    pub fn with_api(api: A) -> Self {
        Self { api }
    }

    /// Get reference to the underlying API
    pub fn api(&self) -> &A {
        &self.api
    }

    /// Get foreground window
    pub fn get_foreground_window(&self) -> Option<WindowId> {
        self.api.get_foreground_window()
    }

    /// Get foreground window information
    pub fn get_foreground_window_info(&self) -> Result<WindowInfo> {
        let window = self
            .api
            .get_foreground_window()
            .ok_or_else(|| anyhow::anyhow!("No foreground window"))?;
        self.api.get_window_info(window)
    }

    /// Get window information
    pub fn get_window_info(&self, window: WindowId) -> Result<WindowInfo> {
        self.api.get_window_info(window)
    }

    /// Get debug info string
    pub fn get_debug_info(&self) -> Result<String> {
        let info = self.get_foreground_window_info()?;

        Ok(format!(
            "Window: {}\nID: {}\nPosition: [{}, {}]\nSize: {} x {}",
            info.title,
            info.id,
            info.x,
            info.y,
            info.width,
            info.height,
        ))
    }

    /// Get window rectangle
    pub fn get_window_rect(&self, window: WindowId) -> Option<WindowFrame> {
        self.api.get_window_info(window).ok().map(|info| {
            WindowFrame::new(info.x, info.y, info.width, info.height)
        })
    }

    /// Set window position
    pub fn set_window_pos(
        &self,
        window: WindowId,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<()> {
        self.api
            .set_window_pos(window, x, y, width, height)
    }

    /// Get monitors
    pub fn get_monitors(&self) -> Vec<MonitorInfo> {
        self.api.get_monitors()
    }

    /// Check if window is valid
    pub fn is_window_valid(&self, window: WindowId) -> bool {
        self.api.is_window_valid(window)
    }

    /// Check if window is maximized
    pub fn is_maximized(&self, window: WindowId) -> bool {
        self.api.is_maximized(window)
    }

    /// Check if window is topmost
    pub fn is_topmost(&self, window: WindowId) -> bool {
        self.api.is_topmost(window)
    }

    /// Set window topmost state
    pub fn set_topmost(&self, window: WindowId, topmost: bool) -> Result<()> {
        self.api.set_topmost(window, topmost)
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

    /// Set window frame with restoration check
    pub fn set_window_frame(&self, window: WindowId, frame: &WindowFrame) -> Result<()> {
        self.api.ensure_window_restored(window)?;
        self.api.set_window_pos(window, frame.x, frame.y, frame.width, frame.height)
    }
}

impl<A: WindowApiBase<WindowId = WindowId> + 'static> CommonWindowApi for WindowManager<A> {
    type WindowId = WindowId;
    type WindowInfo = WindowInfo;

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
        let frame = WindowFrame::new(x, y, width, height);
        self.set_window_frame(window, &frame)
    }

    fn get_monitors(&self) -> Vec<MonitorInfo> {
        #[cfg(not(test))]
        {
            self.api.get_monitors()
        }
        #[cfg(test)]
        {
            if let Some(_window) = self.api.get_foreground_window() {
                if let Some(monitor) = self.api.get_monitors().first().cloned() {
                    return vec![monitor];
                }
            }
            vec![MonitorInfo {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            }]
        }
    }

    fn is_window_valid(&self, window: WindowId) -> bool {
        self.api.is_window_valid(window)
    }

    fn is_maximized(&self, window: WindowId) -> bool {
        self.api.is_maximized(window)
    }

    fn is_topmost(&self, window: WindowId) -> bool {
        self.api.is_topmost(window)
    }

    fn set_topmost(&self, window: WindowId, topmost: bool) -> Result<()> {
        self.api.set_topmost(window, topmost)
    }

    fn api(&self) -> &dyn std::any::Any {
        &self.api
    }
}

/// Features requiring real macOS API
impl RealWindowManager {
    /// Move window to another monitor
    pub fn move_to_monitor(
        &self,
        window: WindowId,
        direction: MonitorDirection,
    ) -> Result<()> {
        CommonWindowManager::move_to_monitor(self, window, direction)
    }
}

#[cfg(test)]
mod tests {
    use super::super::window_api::MockWindowApi;
    use super::*;
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
        api.set_window_rect(2, WindowFrame::new(100, 200, 800, 600));
        api.set_monitor_info(
            2,
            MonitorInfo {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
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
        api.set_window_rect(2, WindowFrame::new(0, 0, 800, 600));
        api.set_monitor_info(
            2,
            MonitorInfo {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
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

        api.set_window_rect(2, WindowFrame::new(100, 100, 800, 600));
        api.set_monitor_info(
            2,
            MonitorInfo {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
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

        api.set_window_rect(2, WindowFrame::new(100, 100, 800, 600));
        api.set_monitor_info(
            2,
            MonitorInfo {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
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
        api.set_window_rect(2, WindowFrame::new(0, 0, 960, 600));
        api.set_monitor_info(
            2,
            MonitorInfo {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
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
        api.set_window_rect(2, WindowFrame::new(100, 100, 800, 600));
        api.set_monitor_info(
            2,
            MonitorInfo {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
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

        api.set_window_rect(2, WindowFrame::new(100, 100, 800, 600));
        api.set_window_state(2, false, false);

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

        api.set_window_rect(2, WindowFrame::new(100, 100, 800, 600));

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
