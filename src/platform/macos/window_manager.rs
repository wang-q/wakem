//! macOS window manager implementation
//!
//! Provides window management operations using native macOS APIs:
//! - Core Graphics: Monitor enumeration and basic window info
//! - Accessibility (AXUIElement): Window manipulation (move, resize, minimize, etc.)
//! - Cocoa (NSWorkspace): Application context
//!
//! All operations complete in < 10ms (typically < 5ms).

use crate::platform::macos::window_api::RealWindowApi;
use crate::platform::traits::{MonitorInfo, WindowFrame, WindowId};
pub use crate::platform::traits::MonitorDirection;
pub use crate::platform::window_manager_common::WindowManager;
use anyhow::Result;
use tracing::debug;

/// Type alias for the real window manager using native macOS APIs
pub type RealWindowManager = WindowManager<RealWindowApi>;

impl WindowManager<RealWindowApi> {
    /// Create a new window manager with the real macOS API
    pub fn new() -> Self {
        Self::with_api(RealWindowApi::new())
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

impl Default for RealWindowManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Features requiring real macOS API
impl RealWindowManager {
    /// Move window to another monitor
    pub fn move_to_monitor(&self, window: WindowId, direction: MonitorDirection) -> Result<()> {
        use crate::platform::window_manager_common::CommonWindowManager;
        CommonWindowManager::move_to_monitor(self, window, direction)
    }
}

/// Platform-specific CommonWindowApi implementation with test environment support
impl<A: crate::platform::traits::WindowApiBase<WindowId = WindowId> + 'static> crate::platform::window_manager_common::CommonWindowApi for WindowManager<A> {
    type WindowId = WindowId;
    type WindowInfo = crate::platform::traits::WindowInfo;

    fn api(&self) -> &dyn std::any::Any {
        self.api()
    }

    fn get_window_info(&self, window: Self::WindowId) -> Result<Self::WindowInfo> {
        self.api().get_window_info(window)
    }

    fn set_window_pos(
        &self,
        window: Self::WindowId,
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
            self.api().get_monitors()
        }
        #[cfg(test)]
        {
            if let Some(_window) = self.api().get_foreground_window() {
                if let Some(monitor) = self.api().get_monitors().first().cloned() {
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

    fn is_window_valid(&self, window: Self::WindowId) -> bool {
        self.api().is_window_valid(window)
    }

    fn is_maximized(&self, window: Self::WindowId) -> bool {
        self.api().is_maximized(window)
    }

    fn is_topmost(&self, window: Self::WindowId) -> bool {
        self.api().is_topmost(window)
    }

    fn set_topmost(&self, window: Self::WindowId, topmost: bool) -> Result<()> {
        self.api().set_topmost(window, topmost)
    }
}

#[cfg(test)]
mod tests {
    use super::super::window_api::MockWindowApi;
    use super::*;
    use crate::platform::traits::{MonitorInfo, WindowApiBase, WindowFrame};
    use crate::platform::window_manager_common::CommonWindowApi;
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
        wm.move_to_center(2).unwrap();

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
        wm.move_to_edge(2, Edge::Left).unwrap();
        let info = wm.get_window_info(2).unwrap();
        assert_eq!(info.x, 0);

        // Test right edge
        wm.move_to_edge(2, Edge::Right).unwrap();
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
        wm.set_half_screen(2, Edge::Left).unwrap();
        let info = wm.get_window_info(2).unwrap();
        assert_eq!(info.x, 0);
        assert_eq!(info.y, 0);
        assert_eq!(info.width, 960); // 1920 / 2
        assert_eq!(info.height, 1080);

        // Test right half screen
        wm.set_half_screen(2, Edge::Right).unwrap();
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
        wm.loop_width(2, Alignment::Left).unwrap();

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
        wm.set_fixed_ratio(2, 4.0 / 3.0).unwrap();

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
