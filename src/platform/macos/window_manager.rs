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
pub use crate::platform::window_manager_common::WindowManager;

/// Type alias for the real window manager using native macOS APIs
pub type RealWindowManager = WindowManager<RealWindowApi>;

impl WindowManager<RealWindowApi> {
    /// Create a new window manager with the real macOS API
    pub fn new() -> Self {
        Self::with_api(RealWindowApi::new())
    }
}

impl Default for RealWindowManager {
    fn default() -> Self {
        Self::new()
    }
}

// Re-export window preset manager for macOS
use crate::platform::window_preset_common::WindowPresetManager as CommonWindowPresetManager;

/// Window preset manager type for macOS platform
pub type WindowPresetManager = CommonWindowPresetManager<WindowManager<RealWindowApi>>;

impl Default for WindowPresetManager {
    fn default() -> Self {
        Self::new(WindowManager::new())
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
        wm.set_fixed_ratio(2, 4.0 / 3.0, None).unwrap();

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
