//! macOS window manager implementation
//!
//! Provides comprehensive window management operations on macOS,
//! including half-screen, centering, ratio control, and multi-monitor support.

use crate::platform::macos::window_api::RealWindowApi;
use crate::platform::traits::{
    MonitorInfo, WindowApiBase, WindowFrame, WindowId, WindowInfo, WindowManagerTrait,
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

impl<A: WindowApiBase<WindowId = WindowId>> WindowManager<A> {
    pub fn new(api: A) -> Self {
        Self { api }
    }

    pub fn api(&self) -> &A {
        &self.api
    }
}

impl WindowManager<RealWindowApi> {
    pub fn new_real() -> Self {
        Self {
            api: RealWindowApi::new(),
        }
    }
}

impl<A: WindowApiBase<WindowId = WindowId> + Default> Default for WindowManager<A> {
    fn default() -> Self {
        Self::new(A::default())
    }
}

impl<A: WindowApiBase<WindowId = WindowId> + Send + Sync> WindowManagerTrait for WindowManager<A> {
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

impl<A: WindowApiBase<WindowId = WindowId>> WindowManager<A> {
    /// Get foreground window information
    pub fn get_foreground_window_info(&self) -> Result<WindowInfo> {
        let window = self
            .api
            .get_foreground_window()
            .ok_or_else(|| anyhow::anyhow!("No foreground window"))?;
        self.api.get_window_info(window)
    }

    /// Get debug info string
    pub fn get_debug_info(&self) -> Result<String> {
        let info = self.get_foreground_window_info()?;

        Ok(format!(
            "Window: {}\nID: {}\nPosition: [{}, {}]\nSize: {} x {}",
            info.title, info.id, info.x, info.y, info.width, info.height
        ))
    }

    /// Set window frame (convenience method)
    pub fn set_window_frame(&self, window: WindowId, frame: &WindowFrame) -> Result<()> {
        self.api
            .set_window_pos(window, frame.x, frame.y, frame.width, frame.height)
    }

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

pub type RealWindowManager = WindowManager<RealWindowApi>;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::macos::window_api::MockWindowApi;
    use crate::types::{Alignment, Edge};

    #[test]
    fn test_window_manager_creation() {
        let mock = MockWindowApi::new();
        let mgr = WindowManager::<MockWindowApi>::new(mock);
        drop(mgr);
    }

    #[test]
    fn test_get_foreground_window() {
        let mock = MockWindowApi::new();
        let mgr = WindowManager::<MockWindowApi>::new(mock);
        assert_eq!(mgr.get_foreground_window(), Some(1));
    }

    #[test]
    fn test_move_to_center() {
        let mock = MockWindowApi::new();
        let mgr = WindowManager::<MockWindowApi>::new(mock);
        CommonWindowApi::move_to_center(&mgr, 1).unwrap();

        let info = mgr.api.get_window_info(1).unwrap();
        assert_eq!(info.x, 560);
        assert_eq!(info.y, 240);
    }

    #[test]
    fn test_set_half_screen_left() {
        let mock = MockWindowApi::new();
        let mgr = WindowManager::<MockWindowApi>::new(mock);
        CommonWindowApi::set_half_screen(&mgr, 1, Edge::Left).unwrap();

        let info = mgr.api.get_window_info(1).unwrap();
        assert_eq!(info.x, 0);
        assert_eq!(info.y, 0);
        assert_eq!(info.width, 960);
        assert_eq!(info.height, 1080);
    }

    #[test]
    fn test_set_half_screen_right() {
        let mock = MockWindowApi::new();
        let mgr = WindowManager::<MockWindowApi>::new(mock);
        CommonWindowApi::set_half_screen(&mgr, 1, Edge::Right).unwrap();

        let info = mgr.api.get_window_info(1).unwrap();
        assert_eq!(info.width, 960);
        assert_eq!(info.height, 1080);
        assert_eq!(info.x, 960);
    }

    #[test]
    fn test_set_half_screen_top() {
        let mock = MockWindowApi::new();
        let mgr = WindowManager::<MockWindowApi>::new(mock);
        CommonWindowApi::set_half_screen(&mgr, 1, Edge::Top).unwrap();

        let info = mgr.api.get_window_info(1).unwrap();
        assert_eq!(info.width, 1920);
        assert_eq!(info.height, 540);
        assert_eq!(info.x, 0);
        assert_eq!(info.y, 0);
    }

    #[test]
    fn test_set_half_screen_bottom() {
        let mock = MockWindowApi::new();
        let mgr = WindowManager::<MockWindowApi>::new(mock);
        CommonWindowApi::set_half_screen(&mgr, 1, Edge::Bottom).unwrap();

        let info = mgr.api.get_window_info(1).unwrap();
        assert_eq!(info.width, 1920);
        assert_eq!(info.height, 540);
        assert_eq!(info.y, 540);
    }

    #[test]
    fn test_loop_width() {
        let mock = MockWindowApi::new();
        let mgr = WindowManager::<MockWindowApi>::new(mock);

        CommonWindowApi::loop_width(&mgr, 1, Alignment::Left).unwrap();
        let info = mgr.api.get_window_info(1).unwrap();
        assert!(info.width > 800);
    }

    #[test]
    fn test_loop_height() {
        let mock = MockWindowApi::new();
        let mgr = WindowManager::<MockWindowApi>::new(mock);

        CommonWindowApi::loop_height(&mgr, 1, Alignment::Top).unwrap();
        let info = mgr.api.get_window_info(1).unwrap();
        assert!(info.height > 600);
    }

    #[test]
    fn test_set_fixed_ratio() {
        let mock = MockWindowApi::new();
        let mgr = WindowManager::<MockWindowApi>::new(mock);

        CommonWindowApi::set_fixed_ratio(&mgr, 1, 16.0 / 9.0).unwrap();
        let info = mgr.api.get_window_info(1).unwrap();

        let ratio = info.width as f64 / info.height as f64;
        assert!((ratio - 16.0 / 9.0).abs() < 0.01);
    }

    #[test]
    fn test_toggle_topmost() {
        let mock = MockWindowApi::new();
        let mgr = WindowManager::<MockWindowApi>::new(mock);

        assert!(!mgr.api.is_maximized(1));

        CommonWindowApi::toggle_topmost(&mgr, 1).unwrap();
        assert!(mgr.api.is_window_valid(1));
    }

    #[test]
    fn test_snap_to_grid_2x2() {
        let mock = MockWindowApi::new();
        let mgr = WindowManager::<MockWindowApi>::new(mock);

        CommonWindowApi::snap_to_grid(&mgr, 1, 2, 2, 0, 0).unwrap();
        let info = mgr.api.get_window_info(1).unwrap();
        assert_eq!(info.x, 0);
        assert_eq!(info.y, 0);
        assert_eq!(info.width, 960);
        assert_eq!(info.height, 540);

        CommonWindowApi::snap_to_grid(&mgr, 1, 2, 2, 1, 1).unwrap();
        let info = mgr.api.get_window_info(1).unwrap();
        assert_eq!(info.x, 960);
        assert_eq!(info.y, 540);
        assert_eq!(info.width, 960);
        assert_eq!(info.height, 540);
    }

    #[test]
    fn test_move_to_edge() {
        let mock = MockWindowApi::new();
        let mgr = WindowManager::<MockWindowApi>::new(mock);

        CommonWindowApi::move_to_edge(&mgr, 1, Edge::Left).unwrap();
        let info = mgr.api.get_window_info(1).unwrap();
        assert_eq!(info.x, 0);
        assert_eq!(info.y, 100);

        CommonWindowApi::move_to_edge(&mgr, 1, Edge::Right).unwrap();
        let info = mgr.api.get_window_info(1).unwrap();
        assert_eq!(info.x, 1120);

        CommonWindowApi::move_to_edge(&mgr, 1, Edge::Top).unwrap();
        let info = mgr.api.get_window_info(1).unwrap();
        assert_eq!(info.y, 0);

        CommonWindowApi::move_to_edge(&mgr, 1, Edge::Bottom).unwrap();
        let info = mgr.api.get_window_info(1).unwrap();
        assert_eq!(info.y, 480);
    }

    #[test]
    fn test_resize_from_corner() {
        let mock = MockWindowApi::new();
        let mgr = WindowManager::<MockWindowApi>::new(mock);

        CommonWindowApi::resize_from_corner(&mgr, 1, 200, 150, Edge::Right).unwrap();
        let info = mgr.api.get_window_info(1).unwrap();
        assert_eq!(info.width, 1000);
        assert_eq!(info.height, 750);
    }

    #[test]
    fn test_switch_same_process() {
        let mock = MockWindowApi::new();
        let mgr = WindowManager::<MockWindowApi>::new(mock);
        mgr.switch_to_next_window_of_same_process().unwrap();
    }
}
