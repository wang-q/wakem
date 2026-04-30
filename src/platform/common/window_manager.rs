//! Common window manager logic shared across platforms
//!
//! This module provides platform-agnostic window management operations
//! that can be used by any platform-specific window manager.

use crate::platform::traits::{MonitorInfo, WindowFrame, WindowInfoProvider};
use crate::types::{Alignment, Edge};
use anyhow::Result;
use tracing::debug;

/// Common window manager operations
///
/// This struct provides high-level window management operations that are
/// platform-agnostic and can be used by any window manager implementation.
pub struct CommonWindowManager;

/// Find the monitor that contains the given point, falling back to the first monitor.
pub fn find_monitor_for_point(
    monitors: &[MonitorInfo],
    x: i32,
    y: i32,
) -> Option<&MonitorInfo> {
    monitors
        .iter()
        .find(|m| x >= m.x && x < m.x + m.width && y >= m.y && y < m.y + m.height)
        .or_else(|| monitors.first())
}

/// Find the next ratio in the cycle after the current one.
///
/// Uses "find closest match" instead of exact float comparison to avoid
/// floating-point precision issues caused by integer truncation in
/// `set_window_pos`. For example, on a 1366px monitor, `1366 * 0.6 = 819.6`
/// truncates to `819`, and `819 / 1366 = 0.59956...` which won't match `0.6`
/// with a tight threshold. Finding the closest ratio ensures we always advance
/// by exactly one step in the cycle.
pub fn find_next_ratio(ratios: &[f32], current: f32) -> f32 {
    let closest_idx = ratios
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            (current - **a)
                .abs()
                .partial_cmp(&(current - **b).abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(i, _)| i)
        .unwrap_or(0);

    ratios[(closest_idx + 1) % ratios.len()]
}

/// Trait for window API operations needed by common window manager
pub trait CommonWindowApi {
    type WindowId: Copy;
    type WindowInfo: WindowInfoProvider;

    /// Get the foreground (active) window
    fn get_foreground_window(&self) -> Option<Self::WindowId>;
    /// Get window information
    fn get_window_info(&self, window: Self::WindowId) -> Result<Self::WindowInfo>;
    /// Set window position and size
    fn set_window_pos(
        &self,
        window: Self::WindowId,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<()>;
    /// Minimize window
    fn minimize_window(&self, window: Self::WindowId) -> Result<()>;
    /// Maximize window
    fn maximize_window(&self, window: Self::WindowId) -> Result<()>;
    /// Restore window
    fn restore_window(&self, window: Self::WindowId) -> Result<()>;
    /// Close window
    fn close_window(&self, window: Self::WindowId) -> Result<()>;
    /// Get all monitors
    fn get_monitors(&self) -> Vec<MonitorInfo>;
    /// Check if window is valid
    fn is_window_valid(&self, window: Self::WindowId) -> bool;
    /// Check if window is maximized
    fn is_maximized(&self, window: Self::WindowId) -> bool;
    /// Check if window is topmost (always on top)
    fn is_topmost(&self, window: Self::WindowId) -> bool;
    /// Set window topmost state
    fn set_topmost(&self, window: Self::WindowId, topmost: bool) -> Result<()>;

    /// Get the underlying API reference (for extension methods)
    fn api(&self) -> &dyn std::any::Any;

    /// Move window to center of its current monitor
    fn move_to_center(&self, window: Self::WindowId) -> Result<()>
    where
        Self: Sized,
    {
        CommonWindowManager::move_to_center(self, window)
    }

    /// Move window to edge of screen
    fn move_to_edge(&self, window: Self::WindowId, edge: Edge) -> Result<()>
    where
        Self: Sized,
    {
        CommonWindowManager::move_to_edge(self, window, edge)
    }

    /// Set window to half screen (left/right/top/bottom)
    fn set_half_screen(&self, window: Self::WindowId, edge: Edge) -> Result<()>
    where
        Self: Sized,
    {
        CommonWindowManager::set_half_screen(self, window, edge)
    }

    /// Loop through common widths for the current window position
    fn loop_width(&self, window: Self::WindowId, align: Alignment) -> Result<()>
    where
        Self: Sized,
    {
        CommonWindowManager::loop_width(self, window, align)
    }

    /// Loop through common heights for the current window position
    fn loop_height(&self, window: Self::WindowId, align: Alignment) -> Result<()>
    where
        Self: Sized,
    {
        CommonWindowManager::loop_height(self, window, align)
    }

    /// Set window to a fixed aspect ratio and scale it up/down cyclically
    fn set_fixed_ratio(&self, window: Self::WindowId, ratio: f32) -> Result<()>
    where
        Self: Sized,
    {
        CommonWindowManager::set_fixed_ratio(self, window, ratio)
    }

    /// Set window to its "native" content ratio and cycle sizes
    fn set_native_ratio(&self, window: Self::WindowId) -> Result<()>
    where
        Self: Sized,
    {
        CommonWindowManager::set_native_ratio(self, window)
    }

    /// Toggle window topmost state, returns the new state
    fn toggle_topmost(&self, window: Self::WindowId) -> Result<bool>
    where
        Self: Sized,
    {
        CommonWindowManager::toggle_topmost(self, window)
    }

    /// Get foreground window information
    fn get_foreground_window_info(
        &self,
    ) -> Option<Result<crate::platform::traits::WindowInfo>>
    where
        Self: Sized,
    {
        None
    }

    /// Set window frame (convenience method)
    fn set_window_frame(&self, window: Self::WindowId, frame: &WindowFrame) -> Result<()>
    where
        Self: Sized,
    {
        self.set_window_pos(window, frame.x, frame.y, frame.width, frame.height)
    }

    /// Resize window from corner with anchor
    fn resize_from_corner(
        &self,
        window: Self::WindowId,
        delta_w: i32,
        delta_h: i32,
        anchor: Edge,
    ) -> Result<()>
    where
        Self: Sized,
    {
        let info = self.get_window_info(window)?;

        let (new_x, new_y, new_w, new_h) = match anchor {
            Edge::Right | Edge::Bottom => (
                info.x(),
                info.y(),
                (info.width() + delta_w).max(100),
                (info.height() + delta_h).max(100),
            ),
            Edge::Left => (
                (info.x() - delta_w).max(0),
                info.y(),
                (info.width() + delta_w).max(100),
                (info.height() + delta_h).max(100),
            ),
            Edge::Top => (
                info.x(),
                (info.y() - delta_h).max(0),
                (info.width() + delta_w).max(100),
                (info.height() + delta_h).max(100),
            ),
        };

        self.set_window_pos(window, new_x, new_y, new_w, new_h)?;
        debug!(
            "Resized from {:?}: {}x{} at ({}, {})",
            anchor, new_w, new_h, new_x, new_y
        );
        Ok(())
    }

    /// Snap window to grid position
    fn snap_to_grid(
        &self,
        window: Self::WindowId,
        cols: u32,
        rows: u32,
        col_idx: u32,
        row_idx: u32,
    ) -> Result<()>
    where
        Self: Sized,
    {
        let monitors = self.get_monitors();
        let monitor = monitors
            .first()
            .ok_or_else(|| anyhow::anyhow!("No monitors found"))?;

        let cell_w = monitor.width / cols as i32;
        let cell_h = monitor.height / rows as i32;

        let new_x = monitor.x + col_idx as i32 * cell_w;
        let new_y = monitor.y + row_idx as i32 * cell_h;

        self.set_window_pos(window, new_x, new_y, cell_w, cell_h)?;
        debug!(
            "Snapped to grid [{},{}] of {}x{}: {}x{} at ({}, {})",
            col_idx, row_idx, cols, rows, cell_w, cell_h, new_x, new_y
        );
        Ok(())
    }
}

impl CommonWindowManager {
    /// Move window to center of its current monitor
    pub fn move_to_center<A, W, I>(api: &A, window: W) -> Result<()>
    where
        A: CommonWindowApi<WindowId = W, WindowInfo = I>,
        I: WindowInfoProvider,
        W: Copy,
    {
        let info = api.get_window_info(window)?;
        let monitors = api.get_monitors();

        let monitor = find_monitor_for_point(&monitors, info.x(), info.y())
            .ok_or_else(|| anyhow::anyhow!("No monitors found"))?;

        let frame = WindowFrame::new(info.x(), info.y(), info.width(), info.height());
        let (new_x, new_y) = frame.center_in(monitor);

        api.set_window_pos(window, new_x, new_y, info.width(), info.height())?;
        debug!("Moved window to center: ({}, {})", new_x, new_y);
        Ok(())
    }

    /// Set window to half screen (left/right/top/bottom)
    pub fn set_half_screen<A, W, I>(api: &A, window: W, edge: Edge) -> Result<()>
    where
        A: CommonWindowApi<WindowId = W, WindowInfo = I>,
        I: WindowInfoProvider,
        W: Copy,
    {
        let info = api.get_window_info(window)?;
        let monitors = api.get_monitors();
        let monitor = find_monitor_for_point(&monitors, info.x(), info.y())
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

        api.set_window_pos(window, new_x, new_y, new_width, new_height)?;
        debug!(
            "Set half screen ({:?}): {}x{} at ({}, {})",
            edge, new_width, new_height, new_x, new_y
        );
        Ok(())
    }

    /// Loop through common widths for the current window position
    pub fn loop_width<A, W, I>(api: &A, window: W, align: Alignment) -> Result<()>
    where
        A: CommonWindowApi<WindowId = W, WindowInfo = I>,
        I: WindowInfoProvider,
        W: Copy,
    {
        const WIDTH_RATIOS: [f32; 5] = [0.75, 0.6, 0.5, 0.4, 0.25];

        let info = api.get_window_info(window)?;
        let monitors = api.get_monitors();
        let monitor = find_monitor_for_point(&monitors, info.x(), info.y())
            .ok_or_else(|| anyhow::anyhow!("No monitors found"))?;

        let current_ratio = info.width() as f32 / monitor.width as f32;

        let next_ratio = find_next_ratio(&WIDTH_RATIOS, current_ratio);

        let new_width = (monitor.width as f32 * next_ratio) as i32;
        let new_x = match align {
            Alignment::Left => monitor.x,
            Alignment::Right => monitor.x + monitor.width - new_width,
            _ => info.x(),
        };

        api.set_window_pos(window, new_x, info.y(), new_width, info.height())?;
        debug!("Looped width to {} (ratio: {})", new_width, next_ratio);
        Ok(())
    }

    /// Loop through common heights for the current window position
    pub fn loop_height<A, W, I>(api: &A, window: W, align: Alignment) -> Result<()>
    where
        A: CommonWindowApi<WindowId = W, WindowInfo = I>,
        I: WindowInfoProvider,
        W: Copy,
    {
        const HEIGHT_RATIOS: [f32; 3] = [0.75, 0.5, 0.25];

        let info = api.get_window_info(window)?;
        let monitors = api.get_monitors();
        let monitor = find_monitor_for_point(&monitors, info.x(), info.y())
            .ok_or_else(|| anyhow::anyhow!("No monitors found"))?;

        let current_ratio = info.height() as f32 / monitor.height as f32;

        let next_ratio = find_next_ratio(&HEIGHT_RATIOS, current_ratio);

        let new_height = (monitor.height as f32 * next_ratio) as i32;
        let new_y = match align {
            Alignment::Top => monitor.y,
            Alignment::Bottom => monitor.y + monitor.height - new_height,
            _ => info.y(),
        };

        api.set_window_pos(window, info.x(), new_y, info.width(), new_height)?;
        debug!("Looped height to {} (ratio: {})", new_height, next_ratio);
        Ok(())
    }

    /// Set window to a fixed aspect ratio and scale it up/down cyclically
    /// Automatically cycles through scales: 100% -> 90% -> 70% -> 50% -> 100%
    pub fn set_fixed_ratio<A, W, I>(api: &A, window: W, ratio: f32) -> Result<()>
    where
        A: CommonWindowApi<WindowId = W, WindowInfo = I>,
        I: WindowInfoProvider,
        W: Copy,
    {
        const SCALES: [f32; 4] = [1.0, 0.9, 0.7, 0.5];

        let info = api.get_window_info(window)?;
        let monitors = api.get_monitors();
        let monitor = find_monitor_for_point(&monitors, info.x(), info.y())
            .ok_or_else(|| anyhow::anyhow!("No monitors found"))?;

        let base_size = std::cmp::min(monitor.width, monitor.height);
        let base_width = (base_size as f32 * ratio) as i32;
        let base_height = base_size;

        let current_width_ratio = info.width() as f32 / base_width as f32;
        let current_height_ratio = info.height() as f32 / base_height as f32;
        let current_scale = (current_width_ratio + current_height_ratio) / 2.0;

        let mut next_scale = SCALES[0];
        for (i, scale) in SCALES.iter().enumerate() {
            if (current_scale - scale).abs() < 0.05 {
                next_scale = SCALES[(i + 1) % SCALES.len()];
                break;
            }
        }

        let new_width = (base_width as f32 * next_scale) as i32;
        let new_height = (base_height as f32 * next_scale) as i32;

        let new_x = monitor.x + (monitor.width - new_width) / 2;
        let new_y = monitor.y + (monitor.height - new_height) / 2;

        api.set_window_pos(window, new_x, new_y, new_width, new_height)?;
        debug!(
            "Set fixed ratio {} -> {}x{} at ({}, {})",
            ratio, new_width, new_height, new_x, new_y
        );
        Ok(())
    }

    /// Set window to its "native" content ratio (e.g., video 16:9) and cycle sizes
    /// Automatically cycles through scales: 100% -> 90% -> 70% -> 50% -> 100%
    pub fn set_native_ratio<A, W, I>(api: &A, window: W) -> Result<()>
    where
        A: CommonWindowApi<WindowId = W, WindowInfo = I>,
        I: WindowInfoProvider,
        W: Copy,
    {
        let info = api.get_window_info(window)?;
        let monitors = api.get_monitors();
        let monitor = find_monitor_for_point(&monitors, info.x(), info.y())
            .ok_or_else(|| anyhow::anyhow!("No monitors found"))?;

        let screen_ratio = monitor.width as f32 / monitor.height as f32;
        let base_size = std::cmp::min(monitor.width, monitor.height);
        let base_width = (base_size as f32 * screen_ratio) as i32;
        let base_height = base_size;

        let current_width_ratio = info.width() as f32 / base_width as f32;
        let current_height_ratio = info.height() as f32 / base_height as f32;
        let current_scale = (current_width_ratio + current_height_ratio) / 2.0;

        const SCALES: [f32; 4] = [1.0, 0.9, 0.7, 0.5];

        let mut next_scale = SCALES[0];
        for (i, scale) in SCALES.iter().enumerate() {
            if (current_scale - scale).abs() < 0.05 {
                next_scale = SCALES[(i + 1) % SCALES.len()];
                break;
            }
        }

        let new_width = (base_width as f32 * next_scale) as i32;
        let new_height = (base_height as f32 * next_scale) as i32;

        let new_x = monitor.x + (monitor.width - new_width) / 2;
        let new_y = monitor.y + (monitor.height - new_height) / 2;

        api.set_window_pos(window, new_x, new_y, new_width, new_height)?;
        debug!(
            "Set native ratio -> {}x{} at ({}, {})",
            new_width, new_height, new_x, new_y
        );
        Ok(())
    }

    /// Toggle window topmost state
    pub fn toggle_topmost<A, W, I>(api: &A, window: W) -> Result<bool>
    where
        A: CommonWindowApi<WindowId = W, WindowInfo = I>,
        I: WindowInfoProvider,
        W: Copy,
    {
        if !api.is_window_valid(window) {
            return Err(anyhow::anyhow!("Invalid window handle"));
        }

        let current = api.is_topmost(window);
        let new_state = !current;
        api.set_topmost(window, new_state)?;
        debug!("Toggled topmost: {} -> {}", current, new_state);
        Ok(new_state)
    }

    /// Move window to edge of screen
    pub fn move_to_edge<A, W, I>(api: &A, window: W, edge: Edge) -> Result<()>
    where
        A: CommonWindowApi<WindowId = W, WindowInfo = I>,
        I: WindowInfoProvider,
        W: Copy,
    {
        let info = api.get_window_info(window)?;
        let monitors = api.get_monitors();
        let monitor = find_monitor_for_point(&monitors, info.x(), info.y())
            .ok_or_else(|| anyhow::anyhow!("No monitors found"))?;

        let (new_x, new_y) = match edge {
            Edge::Left => (monitor.x, info.y()),
            Edge::Right => (monitor.x + monitor.width - info.width(), info.y()),
            Edge::Top => (info.x(), monitor.y),
            Edge::Bottom => (info.x(), monitor.y + monitor.height - info.height()),
        };

        api.set_window_pos(window, new_x, new_y, info.width(), info.height())?;
        debug!("Moved window to {:?} edge: ({}, {})", edge, new_x, new_y);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_frame_creation() {
        let frame = WindowFrame::new(100, 200, 800, 600);
        assert_eq!(frame.x, 100);
        assert_eq!(frame.y, 200);
        assert_eq!(frame.width, 800);
        assert_eq!(frame.height, 600);
    }

    #[test]
    fn test_window_frame_aspect_ratio() {
        let frame = WindowFrame::new(0, 0, 1920, 1080);
        assert!((frame.aspect_ratio() - 16.0 / 9.0).abs() < 0.001);
    }

    #[test]
    fn test_window_frame_invalid() {
        let frame = WindowFrame::new(0, 0, -100, 500);
        assert!(!frame.is_valid());

        let frame = WindowFrame::new(0, 0, 100, -500);
        assert!(!frame.is_valid());
    }

    #[test]
    fn test_window_frame_center_in() {
        let frame = WindowFrame::new(0, 0, 800, 600);
        let monitor = MonitorInfo {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        };
        let (x, y) = frame.center_in(&monitor);
        assert_eq!(x, 560);
        assert_eq!(y, 240);
    }

    #[test]
    fn test_find_monitor_for_point() {
        let monitors = vec![
            MonitorInfo {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
            MonitorInfo {
                x: 1920,
                y: 0,
                width: 1920,
                height: 1080,
            },
        ];

        let m = find_monitor_for_point(&monitors, 500, 500).unwrap();
        assert_eq!(m.x, 0);

        let m = find_monitor_for_point(&monitors, 2500, 500).unwrap();
        assert_eq!(m.x, 1920);

        let m = find_monitor_for_point(&monitors, -100, -100).unwrap();
        assert_eq!(m.x, 0);
    }

    #[test]
    fn test_find_next_ratio_exact_match() {
        let ratios: [f32; 5] = [0.75, 0.6, 0.5, 0.4, 0.25];
        assert!((find_next_ratio(&ratios, 0.75) - 0.6).abs() < 0.001);
        assert!((find_next_ratio(&ratios, 0.6) - 0.5).abs() < 0.001);
        assert!((find_next_ratio(&ratios, 0.5) - 0.4).abs() < 0.001);
        assert!((find_next_ratio(&ratios, 0.4) - 0.25).abs() < 0.001);
        assert!((find_next_ratio(&ratios, 0.25) - 0.75).abs() < 0.001);
    }

    #[test]
    fn test_find_next_ratio_float_truncation() {
        let ratios: [f32; 5] = [0.75, 0.6, 0.5, 0.4, 0.25];

        // Simulate 1366px monitor: 1366 * 0.6 = 819.6 -> truncated to 819
        // 819 / 1366 = 0.59956... which should still match 0.6
        let truncated_ratio = 819.0_f32 / 1366.0_f32;
        assert!((find_next_ratio(&ratios, truncated_ratio) - 0.5).abs() < 0.001);

        // Simulate 1366px monitor: 1366 * 0.75 = 1024.5 -> truncated to 1024
        // 1024 / 1366 = 0.7498... which should still match 0.75
        let truncated_ratio = 1024.0_f32 / 1366.0_f32;
        assert!((find_next_ratio(&ratios, truncated_ratio) - 0.6).abs() < 0.001);
    }

    #[test]
    fn test_find_next_ratio_non_standard() {
        let ratios: [f32; 5] = [0.75, 0.6, 0.5, 0.4, 0.25];

        // Non-standard ratio (e.g., user manually resized) should find closest
        // 0.55 is closest to 0.5 (distance 0.05) vs 0.6 (distance 0.05)
        // With equal distance, min_by picks the first one found (0.75 -> index 0)
        // Actually 0.55: |0.55-0.75|=0.20, |0.55-0.6|=0.05, |0.55-0.5|=0.05
        // Closest is either 0.6 or 0.5 (tie), min_by is stable -> picks 0.6
        let next = find_next_ratio(&ratios, 0.55);
        // Should advance from closest (0.6) to next (0.5)
        assert!((next - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_find_next_ratio_height_cycle() {
        let ratios: [f32; 3] = [0.75, 0.5, 0.25];
        assert!((find_next_ratio(&ratios, 0.75) - 0.5).abs() < 0.001);
        assert!((find_next_ratio(&ratios, 0.5) - 0.25).abs() < 0.001);
        assert!((find_next_ratio(&ratios, 0.25) - 0.75).abs() < 0.001);
    }

    #[test]
    fn test_find_next_ratio_no_double_skip() {
        let ratios: [f32; 5] = [0.75, 0.6, 0.5, 0.4, 0.25];

        // Simulate the full cycle on a 1920px monitor
        // Each step should advance by exactly one position
        let monitor_width = 1920.0_f32;

        // Start at 75% (1440px)
        let w1 = (monitor_width * 0.75) as i32; // 1440
        let r1 = w1 as f32 / monitor_width; // 0.75
        let next1 = find_next_ratio(&ratios, r1);
        assert!((next1 - 0.6).abs() < 0.001, "Step 1: 0.75 -> 0.6");

        // After setting to 60% (1152px)
        let w2 = (monitor_width * next1) as i32; // 1152
        let r2 = w2 as f32 / monitor_width; // 0.6
        let next2 = find_next_ratio(&ratios, r2);
        assert!((next2 - 0.5).abs() < 0.001, "Step 2: 0.6 -> 0.5");

        // After setting to 50% (960px)
        let w3 = (monitor_width * next2) as i32; // 960
        let r3 = w3 as f32 / monitor_width; // 0.5
        let next3 = find_next_ratio(&ratios, r3);
        assert!((next3 - 0.4).abs() < 0.001, "Step 3: 0.5 -> 0.4");

        // After setting to 40% (768px)
        let w4 = (monitor_width * next3) as i32; // 768
        let r4 = w4 as f32 / monitor_width; // 0.4
        let next4 = find_next_ratio(&ratios, r4);
        assert!((next4 - 0.25).abs() < 0.001, "Step 4: 0.4 -> 0.25");

        // After setting to 25% (480px) - wraps back
        let w5 = (monitor_width * next4) as i32; // 480
        let r5 = w5 as f32 / monitor_width; // 0.25
        let next5 = find_next_ratio(&ratios, r5);
        assert!((next5 - 0.75).abs() < 0.001, "Step 5: 0.25 -> 0.75 (wrap)");
    }

    #[test]
    fn test_find_next_ratio_odd_resolution() {
        let ratios: [f32; 5] = [0.75, 0.6, 0.5, 0.4, 0.25];

        // Test with 1366x768 (common laptop resolution)
        let monitor_width = 1366.0_f32;

        // 75% of 1366 = 1024.5 -> truncated to 1024
        let w1 = (monitor_width * 0.75) as i32;
        let r1 = w1 as f32 / monitor_width;
        let next1 = find_next_ratio(&ratios, r1);
        assert!((next1 - 0.6).abs() < 0.001, "1366: 75% -> 60%");

        // 60% of 1366 = 819.6 -> truncated to 819
        let w2 = (monitor_width * next1) as i32;
        let r2 = w2 as f32 / monitor_width;
        let next2 = find_next_ratio(&ratios, r2);
        assert!((next2 - 0.5).abs() < 0.001, "1366: 60% -> 50%");

        // 50% of 1366 = 683.0
        let w3 = (monitor_width * next2) as i32;
        let r3 = w3 as f32 / monitor_width;
        let next3 = find_next_ratio(&ratios, r3);
        assert!((next3 - 0.4).abs() < 0.001, "1366: 50% -> 40%");

        // 40% of 1366 = 546.4 -> truncated to 546
        let w4 = (monitor_width * next3) as i32;
        let r4 = w4 as f32 / monitor_width;
        let next4 = find_next_ratio(&ratios, r4);
        assert!((next4 - 0.25).abs() < 0.001, "1366: 40% -> 25%");

        // 25% of 1366 = 341.5 -> truncated to 341
        let w5 = (monitor_width * next4) as i32;
        let r5 = w5 as f32 / monitor_width;
        let next5 = find_next_ratio(&ratios, r5);
        assert!((next5 - 0.75).abs() < 0.001, "1366: 25% -> 75% (wrap)");
    }

    // ==================== Mock CommonWindowApi for integration tests ====================

    use std::cell::RefCell;

    #[derive(Clone, Copy)]
    struct TestWindowInfo {
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    }

    impl WindowInfoProvider for TestWindowInfo {
        fn x(&self) -> i32 {
            self.x
        }
        fn y(&self) -> i32 {
            self.y
        }
        fn width(&self) -> i32 {
            self.width
        }
        fn height(&self) -> i32 {
            self.height
        }
    }

    struct TestApi {
        info: RefCell<TestWindowInfo>,
        monitors: Vec<MonitorInfo>,
        pos_log: RefCell<Vec<(i32, i32, i32, i32)>>,
    }

    impl TestApi {
        fn new(monitor: MonitorInfo, window_width: i32, window_height: i32) -> Self {
            Self {
                info: RefCell::new(TestWindowInfo {
                    x: monitor.x,
                    y: monitor.y,
                    width: window_width,
                    height: window_height,
                }),
                monitors: vec![monitor],
                pos_log: RefCell::new(Vec::new()),
            }
        }

        fn last_pos(&self) -> (i32, i32, i32, i32) {
            self.pos_log.borrow().last().copied().unwrap()
        }
    }

    impl CommonWindowApi for TestApi {
        type WindowId = ();
        type WindowInfo = TestWindowInfo;

        fn get_foreground_window(&self) -> Option<Self::WindowId> {
            Some(())
        }

        fn get_window_info(&self, _window: Self::WindowId) -> Result<Self::WindowInfo> {
            Ok(*self.info.borrow())
        }

        fn set_window_pos(
            &self,
            _window: Self::WindowId,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
        ) -> Result<()> {
            self.pos_log.borrow_mut().push((x, y, width, height));
            *self.info.borrow_mut() = TestWindowInfo {
                x,
                y,
                width,
                height,
            };
            Ok(())
        }

        fn minimize_window(&self, _window: Self::WindowId) -> Result<()> {
            Ok(())
        }

        fn maximize_window(&self, _window: Self::WindowId) -> Result<()> {
            Ok(())
        }

        fn restore_window(&self, _window: Self::WindowId) -> Result<()> {
            Ok(())
        }

        fn close_window(&self, _window: Self::WindowId) -> Result<()> {
            Ok(())
        }

        fn get_monitors(&self) -> Vec<MonitorInfo> {
            self.monitors.clone()
        }

        fn is_window_valid(&self, _window: Self::WindowId) -> bool {
            true
        }

        fn is_maximized(&self, _window: Self::WindowId) -> bool {
            false
        }

        fn is_topmost(&self, _window: Self::WindowId) -> bool {
            false
        }

        fn set_topmost(&self, _window: Self::WindowId, _topmost: bool) -> Result<()> {
            Ok(())
        }

        fn api(&self) -> &dyn std::any::Any {
            self
        }
    }

    // ==================== Loop cycle regression tests ====================

    #[test]
    fn test_loop_width_full_cycle_1920() {
        let monitor = MonitorInfo {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        };
        let api = TestApi::new(monitor, 1440, 1080);

        let expected_widths = [1152, 960, 768, 480, 1440];
        for (i, &expected_w) in expected_widths.iter().enumerate() {
            CommonWindowManager::loop_width(&api, (), Alignment::Left).unwrap();
            let (_, _, w, _) = api.last_pos();
            assert_eq!(
                w,
                expected_w,
                "Step {}: expected width {}, got {}",
                i + 1,
                expected_w,
                w
            );
        }
    }

    #[test]
    fn test_loop_width_full_cycle_1366() {
        let monitor = MonitorInfo {
            x: 0,
            y: 0,
            width: 1366,
            height: 768,
        };
        let api = TestApi::new(monitor, 1024, 768);

        let expected_widths = [819, 683, 546, 341, 1024];
        for (i, &expected_w) in expected_widths.iter().enumerate() {
            CommonWindowManager::loop_width(&api, (), Alignment::Left).unwrap();
            let (_, _, w, _) = api.last_pos();
            assert_eq!(
                w,
                expected_w,
                "Step {}: expected width {}, got {}",
                i + 1,
                expected_w,
                w
            );
        }
    }

    #[test]
    fn test_loop_width_full_cycle_2560() {
        let monitor = MonitorInfo {
            x: 0,
            y: 0,
            width: 2560,
            height: 1440,
        };
        let api = TestApi::new(monitor, 1920, 1440);

        let expected_widths = [1536, 1280, 1024, 640, 1920];
        for (i, &expected_w) in expected_widths.iter().enumerate() {
            CommonWindowManager::loop_width(&api, (), Alignment::Left).unwrap();
            let (_, _, w, _) = api.last_pos();
            assert_eq!(
                w,
                expected_w,
                "Step {}: expected width {}, got {}",
                i + 1,
                expected_w,
                w
            );
        }
    }

    #[test]
    fn test_loop_width_right_alignment() {
        let monitor = MonitorInfo {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        };
        let api = TestApi::new(monitor, 1440, 1080);

        CommonWindowManager::loop_width(&api, (), Alignment::Right).unwrap();
        let (x, _, w, _) = api.last_pos();
        assert_eq!(w, 1152);
        assert_eq!(
            x,
            1920 - 1152,
            "Right-aligned: x should be monitor.width - width"
        );
    }

    #[test]
    fn test_loop_height_full_cycle_1080() {
        let monitor = MonitorInfo {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        };
        let api = TestApi::new(monitor, 1920, 810);

        let expected_heights = [540, 270, 810];
        for (i, &expected_h) in expected_heights.iter().enumerate() {
            CommonWindowManager::loop_height(&api, (), Alignment::Top).unwrap();
            let (_, _, _, h) = api.last_pos();
            assert_eq!(
                h,
                expected_h,
                "Step {}: expected height {}, got {}",
                i + 1,
                expected_h,
                h
            );
        }
    }

    #[test]
    fn test_loop_height_full_cycle_768() {
        let monitor = MonitorInfo {
            x: 0,
            y: 0,
            width: 1366,
            height: 768,
        };
        let api = TestApi::new(monitor, 1366, 576);

        let expected_heights = [384, 192, 576];
        for (i, &expected_h) in expected_heights.iter().enumerate() {
            CommonWindowManager::loop_height(&api, (), Alignment::Top).unwrap();
            let (_, _, _, h) = api.last_pos();
            assert_eq!(
                h,
                expected_h,
                "Step {}: expected height {}, got {}",
                i + 1,
                expected_h,
                h
            );
        }
    }

    #[test]
    fn test_loop_height_bottom_alignment() {
        let monitor = MonitorInfo {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        };
        let api = TestApi::new(monitor, 1920, 810);

        CommonWindowManager::loop_height(&api, (), Alignment::Bottom).unwrap();
        let (_, y, _, h) = api.last_pos();
        assert_eq!(h, 540);
        assert_eq!(
            y,
            1080 - 540,
            "Bottom-aligned: y should be monitor.height - height"
        );
    }

    #[test]
    fn test_loop_width_no_double_skip_regression() {
        let monitor = MonitorInfo {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        };
        let api = TestApi::new(monitor, 1440, 1080);

        let mut seen_widths = Vec::new();
        for _ in 0..5 {
            CommonWindowManager::loop_width(&api, (), Alignment::Left).unwrap();
            let (_, _, w, _) = api.last_pos();
            seen_widths.push(w);
        }

        assert_eq!(
            seen_widths.len(),
            seen_widths
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len(),
            "Each loop step should produce a unique width (no skipping): {:?}",
            seen_widths
        );
    }

    #[test]
    fn test_loop_height_no_double_skip_regression() {
        let monitor = MonitorInfo {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        };
        let api = TestApi::new(monitor, 1920, 810);

        let mut seen_heights = Vec::new();
        for _ in 0..3 {
            CommonWindowManager::loop_height(&api, (), Alignment::Top).unwrap();
            let (_, _, _, h) = api.last_pos();
            seen_heights.push(h);
        }

        assert_eq!(
            seen_heights.len(),
            seen_heights
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len(),
            "Each loop step should produce a unique height (no skipping): {:?}",
            seen_heights
        );
    }

    // ==================== Taskbar coverage regression tests ====================

    #[test]
    fn test_half_screen_bottom_within_work_area() {
        let work_area = MonitorInfo {
            x: 0,
            y: 0,
            width: 1920,
            height: 1040,
        };
        let api = TestApi::new(work_area, 800, 600);

        CommonWindowManager::set_half_screen(&api, (), Edge::Bottom).unwrap();
        let (_x, y, _w, h) = api.last_pos();

        assert_eq!(h, 1040 / 2, "Height should be half of work area");
        assert_eq!(
            y + h,
            1040,
            "Window bottom edge should align with work area bottom"
        );
        assert!(
            y + h <= work_area.height,
            "Window should not extend below work area: y={} h={} work_area_height={}",
            y,
            h,
            work_area.height
        );
    }

    #[test]
    fn test_half_screen_top_within_work_area() {
        let work_area = MonitorInfo {
            x: 0,
            y: 0,
            width: 1920,
            height: 1040,
        };
        let api = TestApi::new(work_area, 800, 600);

        CommonWindowManager::set_half_screen(&api, (), Edge::Top).unwrap();
        let (_x, y, _w, h) = api.last_pos();

        assert_eq!(y, 0, "Top half should start at work area top");
        assert_eq!(h, 1040 / 2, "Height should be half of work area");
    }

    #[test]
    fn test_move_to_edge_bottom_within_work_area() {
        let work_area = MonitorInfo {
            x: 0,
            y: 0,
            width: 1920,
            height: 1040,
        };
        let api = TestApi::new(work_area, 800, 600);

        CommonWindowManager::move_to_edge(&api, (), Edge::Bottom).unwrap();
        let (_, y, _, _) = api.last_pos();

        assert_eq!(
            y,
            1040 - 600,
            "Window should be positioned at work_area.height - window.height"
        );
    }

    #[test]
    fn test_loop_height_bottom_within_work_area() {
        let work_area = MonitorInfo {
            x: 0,
            y: 0,
            width: 1920,
            height: 1040,
        };
        let api = TestApi::new(work_area, 1920, 780);

        CommonWindowManager::loop_height(&api, (), Alignment::Bottom).unwrap();
        let (_, y, _, h) = api.last_pos();

        assert!(
            y + h <= work_area.height,
            "Window bottom edge should not exceed work area: y={} h={} work_area_height={}",
            y,
            h,
            work_area.height
        );
    }

    #[test]
    fn test_work_area_with_taskbar_offset() {
        let work_area = MonitorInfo {
            x: 0,
            y: 0,
            width: 1920,
            height: 1040,
        };
        let api = TestApi::new(work_area, 800, 600);

        CommonWindowManager::set_half_screen(&api, (), Edge::Bottom).unwrap();
        let (_, y, _, h) = api.last_pos();

        assert!(
            y + h <= 1040,
            "Window should fit within 1040px work area (40px taskbar): y={} h={}",
            y,
            h
        );
    }

    #[test]
    fn test_half_screen_right_within_work_area() {
        let work_area = MonitorInfo {
            x: 0,
            y: 0,
            width: 1920,
            height: 1040,
        };
        let api = TestApi::new(work_area, 800, 600);

        CommonWindowManager::set_half_screen(&api, (), Edge::Right).unwrap();
        let (x, _y, w, h) = api.last_pos();

        assert_eq!(w, 1920 / 2, "Width should be half of work area");
        assert_eq!(
            x + w,
            1920,
            "Window right edge should align with work area right"
        );
        assert_eq!(h, 1040, "Height should be full work area height");
    }
}
