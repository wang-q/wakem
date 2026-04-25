//! macOS window manager implementation
//!
//! Provides comprehensive window management operations on macOS,
//! including half-screen, centering, ratio control, and multi-monitor support.
#![cfg(target_os = "macos")]

use crate::platform::macos::window_api::{MacosWindowApi, RealMacosWindowApi};
use crate::platform::traits::{
    MonitorInfo, WindowFrame, WindowId, WindowInfo, WindowManagerTrait,
};
use crate::types::Edge;
use anyhow::Result;
use tracing::debug;

// Import common window manager
use crate::platform::traits::WindowInfoProvider;
use crate::platform::window_manager_common::{CommonWindowApi, CommonWindowManager};
use crate::types::Alignment;

/// Backward-compatible alias for [WindowFrame]
pub type MacosWindowFrame = WindowFrame;

/// Edge direction for moving windows to screen edges.
///
/// Note: This is semantically different from [Windows MonitorDirection](crate::platform::windows::MonitorDirection)
/// which represents monitor switching (Next/Prev/Index). This type represents
/// directional edge snapping (Left/Right/Up/Down).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeDirection {
    Left,
    Right,
    Up,
    Down,
}

#[deprecated(
    since = "0.1.2",
    note = "Use EdgeDirection instead. MonitorDirection is semantically different on Windows (Next/Prev/Index)."
)]
pub type MonitorDirection = EdgeDirection;

/// Generic macOS window manager using MacosWindowApi trait
pub struct MacosWindowManager<A: MacosWindowApi> {
    api: A,
}

impl<A: MacosWindowApi> MacosWindowManager<A> {
    /// Create a new window manager with the given API implementation
    pub fn new(api: A) -> Self {
        Self { api }
    }

    /// Get reference to the underlying API
    pub fn api(&self) -> &A {
        &self.api
    }

    /// Get mutable reference to the underlying API
    pub fn api_mut(&mut self) -> &mut A {
        &mut self.api
    }
}

impl MacosWindowManager<RealMacosWindowApi> {
    /// Create a default window manager with real AppleScript-based API
    pub fn new_real() -> Self {
        Self {
            api: RealMacosWindowApi::new(),
        }
    }
}

impl<A: MacosWindowApi + Default> Default for MacosWindowManager<A> {
    fn default() -> Self {
        Self::new(A::default())
    }
}

impl<A: MacosWindowApi + Send + Sync> WindowManagerTrait for MacosWindowManager<A> {
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

// Implement CommonWindowApi for MacosWindowManager to use common window manager logic
impl<A: MacosWindowApi> CommonWindowApi for MacosWindowManager<A> {
    type WindowId = WindowId;
    type WindowInfo = WindowInfo;

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

/// Additional macOS-specific window management features
impl<A: MacosWindowApi> MacosWindowManager<A> {
    /// Get foreground window info
    pub fn get_foreground_window_info(&self) -> Option<Result<WindowInfo>> {
        let id = self.api.get_foreground_window()?;
        Some(self.api.get_window_info(id))
    }

    /// Set window frame directly
    pub fn set_window_frame(
        &self,
        window: WindowId,
        frame: &MacosWindowFrame,
    ) -> Result<()> {
        self.api
            .set_window_pos(window, frame.x, frame.y, frame.width, frame.height)
    }

    /// Move window to edge of screen (macOS convenience using EdgeDirection)
    pub fn move_to_edge_direction(
        &self,
        window: WindowId,
        direction: EdgeDirection,
    ) -> Result<()> {
        let edge = match direction {
            EdgeDirection::Left => Edge::Left,
            EdgeDirection::Right => Edge::Right,
            EdgeDirection::Up => Edge::Top,
            EdgeDirection::Down => Edge::Bottom,
        };
        <Self as CommonWindowApi>::move_to_edge(self, window, edge)
    }

    /// Loop through common widths (defaults to left alignment)
    pub fn loop_width_default(&self, window: WindowId) -> Result<()> {
        <Self as CommonWindowApi>::loop_width(self, window, Alignment::Left)
    }

    /// Loop through common heights (defaults to top alignment)
    pub fn loop_height_default(&self, window: WindowId) -> Result<()> {
        <Self as CommonWindowApi>::loop_height(self, window, Alignment::Top)
    }

    /// Set window to a fixed aspect ratio (macOS convenience using integer ratio)
    pub fn set_fixed_ratio_u32(
        &self,
        window: WindowId,
        ratio_w: u32,
        ratio_h: u32,
    ) -> Result<()> {
        let ratio = ratio_w as f32 / ratio_h as f32;
        <Self as CommonWindowApi>::set_fixed_ratio(self, window, ratio)
    }

    /// Switch to the next window of the same process (Cmd+~ equivalent on macOS)
    #[cfg(not(test))]
    pub fn switch_to_next_window_of_same_process(
        &self,
        _window: WindowId,
    ) -> Result<()> {
        // Use CGEvent to send Cmd+~ keyboard shortcut directly
        // Keycode 50 is the grave/tilde key (`~)
        use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation};
        use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

        let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
            .map_err(|e| anyhow::anyhow!("Failed to create event source: {:?}", e))?;

        // Create key down event for `~ (keycode 50)
        let key_down = CGEvent::new_keyboard_event(source.clone(), 50, true)
            .map_err(|e| anyhow::anyhow!("Failed to create key down event: {:?}", e))?;
        key_down.set_flags(CGEventFlags::CGEventFlagCommand);
        key_down.post(CGEventTapLocation::HID);

        // Create key up event
        let key_up = CGEvent::new_keyboard_event(source, 50, false)
            .map_err(|e| anyhow::anyhow!("Failed to create key up event: {:?}", e))?;
        key_up.set_flags(CGEventFlags::CGEventFlagCommand);
        key_up.post(CGEventTapLocation::HID);

        debug!("Switched to next window of same process (using CGEvent)");
        Ok(())
    }

    /// Switch to the next window of the same process (test mode - no-op)
    #[cfg(test)]
    pub fn switch_to_next_window_of_same_process(
        &self,
        _window: WindowId,
    ) -> Result<()> {
        debug!("[TEST MODE] switch_to_next_window_of_same_process called");
        Ok(())
    }

    /// Resize window maintaining aspect ratio from a corner
    pub fn resize_from_corner(
        &self,
        window: WindowId,
        delta_w: i32,
        delta_h: i32,
        anchor: Edge,
    ) -> Result<()> {
        let info = self.api.get_window_info(window)?;

        let (new_x, new_y, new_w, new_h) = match anchor {
            Edge::Right | Edge::Bottom => (
                info.x,
                info.y,
                (info.width + delta_w).max(100),
                (info.height + delta_h).max(100),
            ),
            Edge::Left => (
                (info.x - delta_w).max(0),
                info.y,
                (info.width + delta_w).max(100),
                (info.height + delta_h).max(100),
            ),
            Edge::Top => (
                info.x,
                (info.y - delta_h).max(0),
                (info.width + delta_w).max(100),
                (info.height + delta_h).max(100),
            ),
        };

        self.api
            .set_window_pos(window, new_x, new_y, new_w, new_h)?;
        debug!(
            "Resized from {:?}: {}x{} at ({}, {})",
            anchor, new_w, new_h, new_x, new_y
        );
        Ok(())
    }

    /// Snap window to grid (like Windows Snap Assist)
    pub fn snap_to_grid(
        &self,
        window: WindowId,
        cols: u32,
        rows: u32,
        col_idx: u32,
        row_idx: u32,
    ) -> Result<()> {
        let monitors = self.api.get_monitors();
        let monitor = monitors
            .first()
            .ok_or_else(|| anyhow::anyhow!("No monitors found"))?;

        let cell_w = monitor.width / cols as i32;
        let cell_h = monitor.height / rows as i32;

        let new_x = monitor.x + col_idx as i32 * cell_w;
        let new_y = monitor.y + row_idx as i32 * cell_h;

        self.api
            .set_window_pos(window, new_x, new_y, cell_w, cell_h)?;
        debug!(
            "Snapped to grid [{},{}] of {}x{}: {}x{} at ({}, {})",
            col_idx, row_idx, cols, rows, cell_w, cell_h, new_x, new_y
        );
        Ok(())
    }
}

// Type alias for convenience
pub type RealMacosWindowManager = MacosWindowManager<RealMacosWindowApi>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::macos::window_api::MockMacosWindowApi;

    #[test]
    fn test_window_manager_creation() {
        let mock = MockMacosWindowApi::new();
        let mgr = MacosWindowManager::<MockMacosWindowApi>::new(mock);
        drop(mgr);
    }

    #[test]
    fn test_get_foreground_window() {
        let mock = MockMacosWindowApi::new();
        let mgr = MacosWindowManager::<MockMacosWindowApi>::new(mock);
        assert_eq!(mgr.get_foreground_window(), Some(1));
    }

    #[test]
    fn test_move_to_center() {
        let mock = MockMacosWindowApi::new();
        let mgr = MacosWindowManager::<MockMacosWindowApi>::new(mock);
        CommonWindowApi::move_to_center(&mgr, 1).unwrap();

        let info = mgr.api.get_window_info(1).unwrap();
        assert_eq!(info.x, 560);
        assert_eq!(info.y, 240);
    }

    #[test]
    fn test_set_half_screen_left() {
        let mock = MockMacosWindowApi::new();
        let mgr = MacosWindowManager::<MockMacosWindowApi>::new(mock);
        CommonWindowApi::set_half_screen(&mgr, 1, Edge::Left).unwrap();

        let info = mgr.api.get_window_info(1).unwrap();
        assert_eq!(info.x, 0);
        assert_eq!(info.y, 0);
        assert_eq!(info.width, 960);
        assert_eq!(info.height, 1080);
    }

    #[test]
    fn test_set_half_screen_right() {
        let mock = MockMacosWindowApi::new();
        let mgr = MacosWindowManager::<MockMacosWindowApi>::new(mock);
        CommonWindowApi::set_half_screen(&mgr, 1, Edge::Right).unwrap();

        let info = mgr.api.get_window_info(1).unwrap();
        assert_eq!(info.width, 960);
        assert_eq!(info.height, 1080);
        assert_eq!(info.x, 960);
    }

    #[test]
    fn test_set_half_screen_top() {
        let mock = MockMacosWindowApi::new();
        let mgr = MacosWindowManager::<MockMacosWindowApi>::new(mock);
        CommonWindowApi::set_half_screen(&mgr, 1, Edge::Top).unwrap();

        let info = mgr.api.get_window_info(1).unwrap();
        assert_eq!(info.width, 1920);
        assert_eq!(info.height, 540);
        assert_eq!(info.x, 0);
        assert_eq!(info.y, 0);
    }

    #[test]
    fn test_set_half_screen_bottom() {
        let mock = MockMacosWindowApi::new();
        let mgr = MacosWindowManager::<MockMacosWindowApi>::new(mock);
        CommonWindowApi::set_half_screen(&mgr, 1, Edge::Bottom).unwrap();

        let info = mgr.api.get_window_info(1).unwrap();
        assert_eq!(info.width, 1920);
        assert_eq!(info.height, 540);
        assert_eq!(info.y, 540);
    }

    #[test]
    fn test_loop_width() {
        let mock = MockMacosWindowApi::new();
        let mgr = MacosWindowManager::<MockMacosWindowApi>::new(mock);

        mgr.loop_width_default(1).unwrap();
        let info = mgr.api.get_window_info(1).unwrap();
        assert!(info.width > 800);
    }

    #[test]
    fn test_loop_height() {
        let mock = MockMacosWindowApi::new();
        let mgr = MacosWindowManager::<MockMacosWindowApi>::new(mock);

        mgr.loop_height_default(1).unwrap();
        let info = mgr.api.get_window_info(1).unwrap();
        assert!(info.height > 600);
    }

    #[test]
    fn test_set_fixed_ratio() {
        let mock = MockMacosWindowApi::new();
        let mgr = MacosWindowManager::<MockMacosWindowApi>::new(mock);

        mgr.set_fixed_ratio_u32(1, 16, 9).unwrap();
        let info = mgr.api.get_window_info(1).unwrap();

        let ratio = info.width as f64 / info.height as f64;
        assert!((ratio - 16.0 / 9.0).abs() < 0.01);
    }

    #[test]
    fn test_toggle_topmost() {
        let mock = MockMacosWindowApi::new();
        let mgr = MacosWindowManager::<MockMacosWindowApi>::new(mock);

        assert!(!mgr.api.is_maximized(1));

        CommonWindowApi::toggle_topmost(&mgr, 1).unwrap();
        assert!(mgr.api.is_window_valid(1));
    }

    #[test]
    fn test_snap_to_grid_2x2() {
        let mock = MockMacosWindowApi::new();
        let mgr = MacosWindowManager::<MockMacosWindowApi>::new(mock);

        // Top-left quadrant of 2x2 grid
        mgr.snap_to_grid(1, 2, 2, 0, 0).unwrap();
        let info = mgr.api.get_window_info(1).unwrap();
        assert_eq!(info.x, 0);
        assert_eq!(info.y, 0);
        assert_eq!(info.width, 960); // 1920/2
        assert_eq!(info.height, 540); // 1080/2

        // Bottom-right quadrant
        mgr.snap_to_grid(1, 2, 2, 1, 1).unwrap();
        let info = mgr.api.get_window_info(1).unwrap();
        assert_eq!(info.x, 960);
        assert_eq!(info.y, 540);
        assert_eq!(info.width, 960);
        assert_eq!(info.height, 540);
    }

    #[test]
    fn test_move_to_edge() {
        let mock = MockMacosWindowApi::new();
        let mgr = MacosWindowManager::<MockMacosWindowApi>::new(mock);

        mgr.move_to_edge_direction(1, EdgeDirection::Left).unwrap();
        let info = mgr.api.get_window_info(1).unwrap();
        assert_eq!(info.x, 0);
        assert_eq!(info.y, 100);

        mgr.move_to_edge_direction(1, EdgeDirection::Right).unwrap();
        let info = mgr.api.get_window_info(1).unwrap();
        assert_eq!(info.x, 1120);

        mgr.move_to_edge_direction(1, EdgeDirection::Up).unwrap();
        let info = mgr.api.get_window_info(1).unwrap();
        assert_eq!(info.y, 0);

        mgr.move_to_edge_direction(1, EdgeDirection::Down).unwrap();
        let info = mgr.api.get_window_info(1).unwrap();
        assert_eq!(info.y, 480);
    }

    #[test]
    fn test_resize_from_corner() {
        let mock = MockMacosWindowApi::new();
        let mgr = MacosWindowManager::<MockMacosWindowApi>::new(mock);

        // Resize from bottom-right corner by +200, +150
        mgr.resize_from_corner(1, 200, 150, Edge::Right).unwrap();
        let info = mgr.api.get_window_info(1).unwrap();
        assert_eq!(info.width, 1000); // 800+200
        assert_eq!(info.height, 750); // 600+150
    }

    #[test]
    fn test_switch_same_process() {
        let mock = MockMacosWindowApi::new();
        let mgr = MacosWindowManager::<MockMacosWindowApi>::new(mock);
        // Should not panic
        mgr.switch_to_next_window_of_same_process(1).unwrap();
    }

    #[test]
    fn test_window_frame_aspect_ratio() {
        let frame = MacosWindowFrame::new(0, 0, 1920, 1080);
        assert!((frame.aspect_ratio() - 16.0 / 9.0).abs() < 0.001);
        assert!(frame.is_valid());
    }

    #[test]
    fn test_window_frame_invalid() {
        let frame = MacosWindowFrame::new(0, 0, -100, 500);
        assert!(!frame.is_valid());
    }
}
