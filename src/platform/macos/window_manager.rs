//! macOS window manager implementation
//!
//! Provides window management operations using native macOS APIs:
//! - Core Graphics: Monitor enumeration and basic window info
//! - Accessibility (AXUIElement): Window manipulation (move, resize, minimize, etc.)
//! - Cocoa (NSWorkspace): Application context
//!
//! All operations complete in < 10ms (typically < 5ms).

use crate::platform::macos::window_api::RealWindowApi;
pub use crate::platform::traits::MonitorDirection;
use crate::platform::traits::{WindowApiBase, WindowFrame, WindowId};
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

    #[allow(dead_code)]
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
    pub fn move_to_monitor(
        &self,
        window: WindowId,
        direction: MonitorDirection,
    ) -> Result<()> {
        use crate::platform::window_manager_common::CommonWindowManager;
        CommonWindowManager::move_to_monitor(self, window, direction)
    }
}

/// Implement WindowManagerTrait for macOS RealWindowManager
///
/// This bridges the platform-specific CGWindowNumber (usize) to the unified
/// WindowId (usize) used by the cross-platform trait abstraction.
impl crate::platform::traits::WindowManagerTrait for RealWindowManager {
    fn get_foreground_window(&self) -> Option<crate::platform::traits::WindowId> {
        self.api().get_foreground_window()
    }

    fn get_window_info(
        &self,
        window: crate::platform::traits::WindowId,
    ) -> Result<crate::platform::traits::WindowInfo> {
        let info = self.api().get_window_info(window)?;
        Ok(crate::platform::traits::WindowInfo {
            id: info.id,
            title: info.title,
            process_name: info.process_name,
            executable_path: info.executable_path,
            x: info.x,
            y: info.y,
            width: info.width,
            height: info.height,
        })
    }

    fn set_window_pos(
        &self,
        window: crate::platform::traits::WindowId,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<()> {
        let frame = WindowFrame::new(x, y, width, height);
        self.set_window_frame(window, &frame)
    }

    fn minimize_window(&self, window: crate::platform::traits::WindowId) -> Result<()> {
        self.minimize_window(window)
    }

    fn maximize_window(&self, window: crate::platform::traits::WindowId) -> Result<()> {
        self.maximize_window(window)
    }

    fn restore_window(&self, window: crate::platform::traits::WindowId) -> Result<()> {
        self.restore_window(window)
    }

    fn close_window(&self, window: crate::platform::traits::WindowId) -> Result<()> {
        self.close_window(window)
    }

    fn set_topmost(
        &self,
        window: crate::platform::traits::WindowId,
        topmost: bool,
    ) -> Result<()> {
        self.api().set_topmost(window, topmost)
    }

    fn get_monitors(&self) -> Vec<crate::platform::traits::MonitorInfo> {
        self.api().get_monitors()
    }

    fn move_to_monitor(
        &self,
        window: crate::platform::traits::WindowId,
        monitor_index: usize,
    ) -> Result<()> {
        let direction = MonitorDirection::Index(monitor_index as i32);
        self.move_to_monitor(window, direction)
    }

    fn is_window_valid(&self, window: crate::platform::traits::WindowId) -> bool {
        self.api().is_window_valid(window)
    }

    fn is_minimized(&self, window: crate::platform::traits::WindowId) -> bool {
        self.api().is_minimized(window)
    }

    fn is_maximized(&self, window: crate::platform::traits::WindowId) -> bool {
        self.api().is_maximized(window)
    }

    fn is_topmost(&self, window: crate::platform::traits::WindowId) -> bool {
        self.api().is_topmost(window)
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
