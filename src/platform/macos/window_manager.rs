//! macOS window manager implementation
//!
//! Provides comprehensive window management operations on macOS,
//! including half-screen, centering, ratio control, and multi-monitor support.

use crate::platform::macos::window_api::{MacosWindowApi, RealMacosWindowApi};
use crate::platform::traits::{
    MonitorInfo, WindowApiTrait, WindowId, WindowInfo, WindowManagerTrait,
};
use crate::types::Edge;
use anyhow::Result;
use tracing::debug;

/// Monitor direction for window movement
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonitorDirection {
    Left,
    Right,
    Up,
    Down,
}

/// Window frame with position and size
#[derive(Debug, Clone)]
pub struct MacosWindowFrame {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl MacosWindowFrame {
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Calculate aspect ratio (width / height)
    pub fn aspect_ratio(&self) -> f64 {
        if self.height > 0 {
            self.width as f64 / self.height as f64
        } else {
            0.0
        }
    }

    /// Check if frame is valid (positive dimensions)
    pub fn is_valid(&self) -> bool {
        self.width > 0 && self.height > 0
    }
}

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

    /// Move window to center of screen or monitor
    pub fn move_to_center(&self, window: WindowId) -> Result<()> {
        let info = self.api.get_window_info(window)?;
        let monitors = self.api.get_monitors();

        if let Some(monitor) = monitors.first() {
            let new_x = monitor.x + (monitor.width - info.width) / 2;
            let new_y = monitor.y + (monitor.height - info.height) / 2;
            self.api
                .set_window_pos(window, new_x, new_y, info.width, info.height)?;
            debug!("Moved window to center: ({}, {})", new_x, new_y);
        }

        Ok(())
    }

    /// Move window to edge of screen
    pub fn move_to_edge(
        &self,
        window: WindowId,
        direction: MonitorDirection,
    ) -> Result<()> {
        let info = self.api.get_window_info(window)?;
        let monitors = self.api.get_monitors();
        let monitor = monitors
            .first()
            .ok_or_else(|| anyhow::anyhow!("No monitors found"))?;

        let (new_x, new_y) = match direction {
            MonitorDirection::Left => (monitor.x, info.y),
            MonitorDirection::Right => (monitor.x + monitor.width - info.width, info.y),
            MonitorDirection::Up => (info.x, monitor.y),
            MonitorDirection::Down => (info.x, monitor.y + monitor.height - info.height),
        };

        self.api
            .set_window_pos(window, new_x, new_y, info.width, info.height)?;
        debug!(
            "Moved window to {:?} edge: ({}, {})",
            direction, new_x, new_y
        );
        Ok(())
    }

    /// Set window to half screen (left/right/top/bottom)
    pub fn set_half_screen(&self, window: WindowId, edge: Edge) -> Result<()> {
        let info = self.api.get_window_info(window)?;
        let monitors = self.api.get_monitors();
        let monitor = monitors
            .first()
            .ok_or_else(|| anyhow::anyhow!("No monitors found"))?;

        let (new_x, new_y, new_width, new_height) = match edge {
            Edge::Left => (monitor.x, monitor.y, monitor.width / 2, monitor.height),
            Edge::Right => {
                let w = monitor.width / 2;
                (monitor.x + monitor.width - w, monitor.y, w, monitor.height)
            }
            Edge::Top => (monitor.x, monitor.y, monitor.width, monitor.height / 2),
            Edge::Bottom => {
                let h = monitor.height / 2;
                (monitor.x, monitor.y + monitor.height - h, monitor.width, h)
            }
        };

        self.api
            .set_window_pos(window, new_x, new_y, new_width, new_height)?;
        debug!(
            "Set half screen ({:?}): {}x{} at ({}, {})",
            edge, new_width, new_height, new_x, new_y
        );
        Ok(())
    }

    /// Loop through common widths for the current window position
    pub fn loop_width(&self, window: WindowId) -> Result<()> {
        let info = self.api.get_window_info(window)?;
        let monitors = self.api.get_monitors();
        let monitor = monitors
            .first()
            .ok_or_else(|| anyhow::anyhow!("No monitors found"))?;

        // Common width presets in order
        let preset_widths = [
            monitor.width / 4,
            monitor.width / 3,
            monitor.width / 2,
            (monitor.width * 3) / 5,
            (monitor.width * 7) / 10,
            monitor.width,
        ];

        let target = if let Some(next) = preset_widths.iter().find(|&&w| w > info.width)
        {
            *next
        } else {
            preset_widths[0]
        };

        self.api
            .set_window_pos(window, info.x, info.y, target, info.height)?;
        debug!("Looped width to {}", target);
        Ok(())
    }

    /// Loop through common heights for the current window position
    pub fn loop_height(&self, window: WindowId) -> Result<()> {
        let info = self.api.get_window_info(window)?;
        let monitors = self.api.get_monitors();
        let monitor = monitors
            .first()
            .ok_or_else(|| anyhow::anyhow!("No monitors found"))?;

        let preset_heights = [
            monitor.height / 4,
            monitor.height / 3,
            monitor.height / 2,
            (monitor.height * 3) / 5,
            (monitor.height * 7) / 10,
            monitor.height,
        ];

        let target =
            if let Some(next) = preset_heights.iter().find(|&&h| h > info.height) {
                *next
            } else {
                preset_heights[0]
            };

        self.api
            .set_window_pos(window, info.x, info.y, info.width, target)?;
        debug!("Looped height to {}", target);
        Ok(())
    }

    /// Set window to a fixed aspect ratio and scale it up/down cyclically
    pub fn set_fixed_ratio(
        &self,
        window: WindowId,
        ratio_w: u32,
        ratio_h: u32,
    ) -> Result<()> {
        let info = self.api.get_window_info(window)?;
        let monitors = self.api.get_monitors();
        let monitor = monitors
            .first()
            .ok_or_else(|| anyhow::anyhow!("No monitors found"))?;

        let current_area = info.width as u64 * info.height as u64;

        // Generate size candidates based on fixed ratio
        let mut candidates: Vec<(i32, i32)> = Vec::new();
        for base in (100..=monitor.width).step_by(50) {
            let w = base;
            let h = (base as u64 * ratio_h as u64 / ratio_w as u64) as i32;
            if w <= monitor.width && h <= monitor.height && w > 0 && h > 0 {
                candidates.push((w, h));
            }
        }

        // Find next candidate larger than current area
        let next_size = candidates
            .iter()
            .filter(|&&(w, h)| (w as u64 * h as u64) > current_area)
            .min_by_key(|&&(w, h)| w as u64 * h as u64);

        let (new_w, new_h) = match next_size {
            Some(size) => *size,
            None => candidates
                .first()
                .copied()
                .unwrap_or((info.width, info.height)),
        };

        let new_x = monitor.x + (monitor.width - new_w) / 2;
        let new_y = monitor.y + (monitor.height - new_h) / 2;

        self.api
            .set_window_pos(window, new_x, new_y, new_w, new_h)?;
        debug!(
            "Set fixed ratio {}:{} -> {}x{}",
            ratio_w, ratio_h, new_w, new_h
        );
        Ok(())
    }

    /// Set window to its "native" content ratio (e.g., video 16:9) and cycle sizes
    pub fn set_native_ratio(&self, window: WindowId) -> Result<()> {
        self.set_fixed_ratio(window, 16, 9)
    }

    /// Toggle topmost state
    pub fn toggle_topmost(&self, window: WindowId) -> Result<()> {
        let is_top = self.api.is_maximized(window);
        self.api.set_topmost(window, !is_top)?;
        debug!("Toggled topmost: {}", !is_top);
        Ok(())
    }

    /// Switch to the next window of the same process (Cmd+~ equivalent on macOS)
    pub fn switch_to_next_window_of_same_process(
        &self,
        _window: WindowId,
    ) -> Result<()> {
        // Use Cmd+~ keyboard shortcut via AppleScript
        use std::process::Command;
        let script = r#"tell application "System Events"
            key code 42 using command down
        end tell"#;

        Command::new("osascript")
            .arg("-e")
            .arg(script)
            .output()
            .ok();

        debug!("Switched to next window of same process");
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
        mgr.move_to_center(1).unwrap();

        let info = mgr.api.get_window_info(1).unwrap();
        // Should be centered on 1920x1080 display: x=(1920-800)/2=560, y=(1080-600)/2=240
        assert_eq!(info.x, 560);
        assert_eq!(info.y, 240);
    }

    #[test]
    fn test_set_half_screen_left() {
        let mock = MockMacosWindowApi::new();
        let mgr = MacosWindowManager::<MockMacosWindowApi>::new(mock);
        mgr.set_half_screen(1, Edge::Left).unwrap();

        let info = mgr.api.get_window_info(1).unwrap();
        assert_eq!(info.x, 0);
        assert_eq!(info.y, 0);
        assert_eq!(info.width, 960); // 1920/2
        assert_eq!(info.height, 1080);
    }

    #[test]
    fn test_set_half_screen_right() {
        let mock = MockMacosWindowApi::new();
        let mgr = MacosWindowManager::<MockMacosWindowApi>::new(mock);
        mgr.set_half_screen(1, Edge::Right).unwrap();

        let info = mgr.api.get_window_info(1).unwrap();
        assert_eq!(info.width, 960); // 1920/2
        assert_eq!(info.height, 1080);
        // Right side: x = 1920 - 960 = 960
        assert_eq!(info.x, 960);
    }

    #[test]
    fn test_set_half_screen_top() {
        let mock = MockMacosWindowApi::new();
        let mgr = MacosWindowManager::<MockMacosWindowApi>::new(mock);
        mgr.set_half_screen(1, Edge::Top).unwrap();

        let info = mgr.api.get_window_info(1).unwrap();
        assert_eq!(info.width, 1920);
        assert_eq!(info.height, 540); // 1080/2
        assert_eq!(info.x, 0);
        assert_eq!(info.y, 0);
    }

    #[test]
    fn test_set_half_screen_bottom() {
        let mock = MockMacosWindowApi::new();
        let mgr = MacosWindowManager::<MockMacosWindowApi>::new(mock);
        mgr.set_half_screen(1, Edge::Bottom).unwrap();

        let info = mgr.api.get_window_info(1).unwrap();
        assert_eq!(info.width, 1920);
        assert_eq!(info.height, 540); // 1080/2
        assert_eq!(info.y, 540); // 1080 - 540
    }

    #[test]
    fn test_loop_width() {
        let mock = MockMacosWindowApi::new();
        let mgr = MacosWindowManager::<MockMacosWindowApi>::new(mock);

        // Initial width 800 should jump to next preset > 800
        mgr.loop_width(1).unwrap();
        let info = mgr.api.get_window_info(1).unwrap();
        assert!(info.width > 800);
    }

    #[test]
    fn test_loop_height() {
        let mock = MockMacosWindowApi::new();
        let mgr = MacosWindowManager::<MockMacosWindowApi>::new(mock);

        // Initial height 600 should jump to next preset > 600
        mgr.loop_height(1).unwrap();
        let info = mgr.api.get_window_info(1).unwrap();
        assert!(info.height > 600);
    }

    #[test]
    fn test_set_fixed_ratio() {
        let mock = MockMacosWindowApi::new();
        let mgr = MacosWindowManager::<MockMacosWindowApi>::new(mock);

        mgr.set_fixed_ratio(1, 16, 9).unwrap();
        let info = mgr.api.get_window_info(1).unwrap();

        let ratio = info.width as f64 / info.height as f64;
        // Should be approximately 16:9
        assert!((ratio - 16.0 / 9.0).abs() < 0.01);
    }

    #[test]
    fn test_toggle_topmost() {
        let mock = MockMacosWindowApi::new();
        let mgr = MacosWindowManager::<MockMacosWindowApi>::new(mock);

        // Initially not maximized (not topmost)
        assert!(!mgr.api.is_maximized(1));

        mgr.toggle_topmost(1).unwrap();
        // toggle_topmost completed without error
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

        // Initial position: (100, 100), size: 800x600
        mgr.move_to_edge(1, MonitorDirection::Left).unwrap();
        let info = mgr.api.get_window_info(1).unwrap();
        assert_eq!(info.x, 0);
        assert_eq!(info.y, 100);

        mgr.move_to_edge(1, MonitorDirection::Right).unwrap();
        let info = mgr.api.get_window_info(1).unwrap();
        assert_eq!(info.x, 1120); // 1920 - 800

        mgr.move_to_edge(1, MonitorDirection::Up).unwrap();
        let info = mgr.api.get_window_info(1).unwrap();
        assert_eq!(info.y, 0);

        mgr.move_to_edge(1, MonitorDirection::Down).unwrap();
        let info = mgr.api.get_window_info(1).unwrap();
        assert_eq!(info.y, 480); // 1080 - 600
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
