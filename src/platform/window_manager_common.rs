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

/// Trait for window API operations needed by common window manager
pub trait CommonWindowApi {
    type WindowId: Copy;
    type WindowInfo: WindowInfoProvider;

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
    /// Get all monitors
    fn get_monitors(&self) -> Vec<MonitorInfo>;
    /// Check if window is valid
    fn is_window_valid(&self, window: Self::WindowId) -> bool;
    /// Check if window is maximized (for topmost toggle)
    fn is_maximized(&self, window: Self::WindowId) -> bool;
    /// Set window topmost state
    fn set_topmost(&self, window: Self::WindowId, topmost: bool) -> Result<()>;

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
    fn set_fixed_ratio(
        &self,
        window: Self::WindowId,
        ratio: f32,
        scale_index: usize,
    ) -> Result<()>
    where
        Self: Sized,
    {
        CommonWindowManager::set_fixed_ratio(self, window, ratio, scale_index)
    }

    /// Set window to its "native" content ratio and cycle sizes
    fn set_native_ratio(&self, window: Self::WindowId, scale_index: usize) -> Result<()>
    where
        Self: Sized,
    {
        CommonWindowManager::set_native_ratio(self, window, scale_index)
    }

    /// Toggle window topmost state, returns the new state
    fn toggle_topmost(&self, window: Self::WindowId) -> Result<bool>
    where
        Self: Sized,
    {
        CommonWindowManager::toggle_topmost(self, window)
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

        let monitor = monitors
            .first()
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
        let monitors = api.get_monitors();
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
        let monitor = monitors
            .first()
            .ok_or_else(|| anyhow::anyhow!("No monitors found"))?;

        let current_ratio = info.width() as f32 / monitor.width as f32;

        // Find next ratio
        let mut next_ratio = WIDTH_RATIOS[0];
        for (i, ratio) in WIDTH_RATIOS.iter().enumerate() {
            if (current_ratio - ratio).abs() < 0.01 {
                next_ratio = WIDTH_RATIOS[(i + 1) % WIDTH_RATIOS.len()];
                break;
            }
        }

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
        let monitor = monitors
            .first()
            .ok_or_else(|| anyhow::anyhow!("No monitors found"))?;

        let current_ratio = info.height() as f32 / monitor.height as f32;

        // Find next ratio
        let mut next_ratio = HEIGHT_RATIOS[0];
        for (i, ratio) in HEIGHT_RATIOS.iter().enumerate() {
            if (current_ratio - ratio).abs() < 0.01 {
                next_ratio = HEIGHT_RATIOS[(i + 1) % HEIGHT_RATIOS.len()];
                break;
            }
        }

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
    pub fn set_fixed_ratio<A, W, I>(
        api: &A,
        window: W,
        ratio: f32,
        _scale_index: usize,
    ) -> Result<()>
    where
        A: CommonWindowApi<WindowId = W, WindowInfo = I>,
        I: WindowInfoProvider,
        W: Copy,
    {
        const SCALES: [f32; 4] = [1.0, 0.9, 0.7, 0.5];

        let info = api.get_window_info(window)?;
        let monitors = api.get_monitors();
        let monitor = monitors
            .first()
            .ok_or_else(|| anyhow::anyhow!("No monitors found"))?;

        // Calculate base size based on the smaller side of work area
        let base_size = std::cmp::min(monitor.width, monitor.height);
        let base_width = (base_size as f32 * ratio) as i32;
        let base_height = base_size;

        // Calculate current scale based on window size
        let current_width_ratio = info.width() as f32 / base_width as f32;
        let current_height_ratio = info.height() as f32 / base_height as f32;
        let current_scale = (current_width_ratio + current_height_ratio) / 2.0;

        // Find next scale (loop through SCALES array)
        let mut next_scale = SCALES[0];
        for (i, scale) in SCALES.iter().enumerate() {
            if (current_scale - scale).abs() < 0.05 {
                next_scale = SCALES[(i + 1) % SCALES.len()];
                break;
            }
        }

        let new_width = (base_width as f32 * next_scale) as i32;
        let new_height = (base_height as f32 * next_scale) as i32;

        // Center
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
    pub fn set_native_ratio<A, W, I>(
        api: &A,
        window: W,
        _scale_index: usize,
    ) -> Result<()>
    where
        A: CommonWindowApi<WindowId = W, WindowInfo = I>,
        I: WindowInfoProvider,
        W: Copy,
    {
        let monitors = api.get_monitors();
        let monitor = monitors
            .first()
            .ok_or_else(|| anyhow::anyhow!("No monitors found"))?;

        // Calculate base size based on screen aspect ratio
        let screen_ratio = monitor.width as f32 / monitor.height as f32;
        let base_size = std::cmp::min(monitor.width, monitor.height);
        let base_width = (base_size as f32 * screen_ratio) as i32;
        let base_height = base_size;

        let info = api.get_window_info(window)?;

        // Calculate current scale based on window size
        let current_width_ratio = info.width() as f32 / base_width as f32;
        let current_height_ratio = info.height() as f32 / base_height as f32;
        let current_scale = (current_width_ratio + current_height_ratio) / 2.0;

        const SCALES: [f32; 4] = [1.0, 0.9, 0.7, 0.5];

        // Find next scale (loop through SCALES array)
        let mut next_scale = SCALES[0];
        for (i, scale) in SCALES.iter().enumerate() {
            if (current_scale - scale).abs() < 0.05 {
                next_scale = SCALES[(i + 1) % SCALES.len()];
                break;
            }
        }

        let new_width = (base_width as f32 * next_scale) as i32;
        let new_height = (base_height as f32 * next_scale) as i32;

        // Center
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

        let current = api.is_maximized(window);
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
        let monitor = monitors
            .first()
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
        assert_eq!(x, 560); // (1920 - 800) / 2
        assert_eq!(y, 240); // (1080 - 600) / 2
    }
}
