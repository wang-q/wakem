//! Common window manager logic shared across platforms.
//!
//! Provides a generic [`WindowManager<A>`] struct with inherent methods
//! for window operations (move, resize, monitor switching, aspect ratio).
//! Platform-specific modules wrap this via type aliases and implement
//! the component traits for [`WindowManagerTrait`] dispatch.

use crate::platform::traits::{
    find_monitor_for_point, find_next_ratio, ForegroundWindowOperations,
    MonitorOperations, WindowApiBase, WindowManagerExt, WindowOperations,
    WindowStateQueries,
};
use crate::platform::types::{WindowFrame, WindowId, WindowInfo};
use crate::types::{Alignment, Edge};
use anyhow::Result;
use tracing::debug;

/// Generic window manager implementation shared across platforms
///
/// Provides a unified interface for window management operations that delegates
/// to platform-specific `WindowApiBase` implementations. Both Windows and macOS
/// use this struct, with platform-specific extensions in their respective modules.
pub struct WindowManager<A: WindowApiBase> {
    api: A,
}

impl<A: WindowApiBase> WindowManager<A> {
    /// Create a window manager with a custom API implementation (for testing)
    pub fn with_api(api: A) -> Self {
        Self { api }
    }

    /// Get reference to the underlying API
    pub fn api(&self) -> &A {
        &self.api
    }

    /// Get foreground window
    pub fn get_foreground_window(&self) -> Option<A::WindowId> {
        self.api.get_foreground_window()
    }

    /// Set window position and size
    pub fn set_window_frame(
        &self,
        window: A::WindowId,
        frame: &WindowFrame,
    ) -> Result<()> {
        self.api.ensure_window_restored(window)?;
        self.api
            .set_window_pos(window, frame.x, frame.y, frame.width, frame.height)?;

        debug!(
            "Window moved to: x={}, y={}, width={}, height={}",
            frame.x, frame.y, frame.width, frame.height
        );

        Ok(())
    }

    /// Minimize window
    pub fn minimize_window(&self, window: A::WindowId) -> Result<()> {
        self.api.minimize_window(window)
    }

    /// Maximize window
    pub fn maximize_window(&self, window: A::WindowId) -> Result<()> {
        self.api.maximize_window(window)
    }

    /// Restore window
    pub fn restore_window(&self, window: A::WindowId) -> Result<()> {
        self.api.restore_window(window)
    }

    /// Close window
    pub fn close_window(&self, window: A::WindowId) -> Result<()> {
        self.api.close_window(window)
    }

    /// Move window to center of its current monitor
    pub fn move_to_center(&self, window: A::WindowId) -> Result<()> {
        let rect = self.api.get_window_rect(window)?;
        let monitors = self.api.get_monitors();

        let monitor = find_monitor_for_point(&monitors, rect.x, rect.y)
            .ok_or_else(|| anyhow::anyhow!("No monitors found"))?;

        let new_x = monitor.x + (monitor.width - rect.width) / 2;
        let new_y = monitor.y + (monitor.height - rect.height) / 2;

        self.set_window_frame(
            window,
            &WindowFrame::new(new_x, new_y, rect.width, rect.height),
        )?;
        debug!("Moved window to center: ({}, {})", new_x, new_y);
        Ok(())
    }

    /// Move window to edge of screen
    pub fn move_to_edge(&self, window: A::WindowId, edge: Edge) -> Result<()> {
        let rect = self.api.get_window_rect(window)?;
        let monitors = self.api.get_monitors();
        let monitor = find_monitor_for_point(&monitors, rect.x, rect.y)
            .ok_or_else(|| anyhow::anyhow!("No monitors found"))?;

        let (new_x, new_y) = match edge {
            Edge::Left => (monitor.x, rect.y),
            Edge::Right => (monitor.x + monitor.width - rect.width, rect.y),
            Edge::Top => (rect.x, monitor.y),
            Edge::Bottom => (rect.x, monitor.y + monitor.height - rect.height),
        };

        self.set_window_frame(
            window,
            &WindowFrame::new(new_x, new_y, rect.width, rect.height),
        )?;
        debug!("Moved window to {:?} edge: ({}, {})", edge, new_x, new_y);
        Ok(())
    }

    /// Set window to half screen (left/right/top/bottom)
    pub fn set_half_screen(&self, window: A::WindowId, edge: Edge) -> Result<()> {
        let rect = self.api.get_window_rect(window)?;
        let monitors = self.api.get_monitors();
        let monitor = find_monitor_for_point(&monitors, rect.x, rect.y)
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

        self.set_window_frame(
            window,
            &WindowFrame::new(new_x, new_y, new_width, new_height),
        )?;
        debug!(
            "Set half screen ({:?}): {}x{} at ({}, {})",
            edge, new_width, new_height, new_x, new_y
        );
        Ok(())
    }

    /// Loop through common widths for the current window position
    pub fn loop_width(&self, window: A::WindowId, align: Alignment) -> Result<()> {
        const WIDTH_RATIOS: [f32; 5] = [0.75, 0.6, 0.5, 0.4, 0.25];

        let rect = self.api.get_window_rect(window)?;
        let monitors = self.api.get_monitors();
        let monitor = find_monitor_for_point(&monitors, rect.x, rect.y)
            .ok_or_else(|| anyhow::anyhow!("No monitors found"))?;

        let current_ratio = rect.width as f32 / monitor.width as f32;
        let next_ratio = find_next_ratio(&WIDTH_RATIOS, current_ratio);

        let new_width = (monitor.width as f32 * next_ratio) as i32;
        let new_x = match align {
            Alignment::Left => monitor.x,
            Alignment::Right => monitor.x + monitor.width - new_width,
            _ => rect.x,
        };

        self.set_window_frame(
            window,
            &WindowFrame::new(new_x, rect.y, new_width, rect.height),
        )?;
        debug!("Looped width to {} (ratio: {})", new_width, next_ratio);
        Ok(())
    }

    /// Loop through common heights for the current window position
    pub fn loop_height(&self, window: A::WindowId, align: Alignment) -> Result<()> {
        const HEIGHT_RATIOS: [f32; 3] = [0.75, 0.5, 0.25];

        let rect = self.api.get_window_rect(window)?;
        let monitors = self.api.get_monitors();
        let monitor = find_monitor_for_point(&monitors, rect.x, rect.y)
            .ok_or_else(|| anyhow::anyhow!("No monitors found"))?;

        let current_ratio = rect.height as f32 / monitor.height as f32;
        let next_ratio = find_next_ratio(&HEIGHT_RATIOS, current_ratio);

        let new_height = (monitor.height as f32 * next_ratio) as i32;
        let new_y = match align {
            Alignment::Top => monitor.y,
            Alignment::Bottom => monitor.y + monitor.height - new_height,
            _ => rect.y,
        };

        self.set_window_frame(
            window,
            &WindowFrame::new(rect.x, new_y, rect.width, new_height),
        )?;
        debug!("Looped height to {} (ratio: {})", new_height, next_ratio);
        Ok(())
    }

    /// Set window to a fixed aspect ratio and scale it up/down cyclically
    pub fn set_fixed_ratio(
        &self,
        window: A::WindowId,
        ratio: f32,
        scale_index: Option<usize>,
    ) -> Result<()> {
        const SCALES: [f32; 4] = [1.0, 0.9, 0.7, 0.5];

        let rect = self.api.get_window_rect(window)?;
        let monitors = self.api.get_monitors();
        let monitor = find_monitor_for_point(&monitors, rect.x, rect.y)
            .ok_or_else(|| anyhow::anyhow!("No monitors found"))?;

        let base_size = std::cmp::min(monitor.width, monitor.height);
        let base_width = (base_size as f32 * ratio) as i32;
        let base_height = base_size;

        let next_scale = match scale_index {
            Some(idx) if idx < SCALES.len() => SCALES[idx],
            Some(idx) => {
                anyhow::bail!(
                    "Scale index {} out of range (0-{})",
                    idx,
                    SCALES.len() - 1
                );
            }
            None => {
                // Auto-detect next scale based on current window size
                let current_scale = (rect.width as f32 / base_width as f32
                    + rect.height as f32 / base_height as f32)
                    / 2.0;
                find_next_ratio(&SCALES, current_scale)
            }
        };

        let new_width = (base_width as f32 * next_scale) as i32;
        let new_height = (base_height as f32 * next_scale) as i32;
        let new_x = monitor.x + (monitor.width - new_width) / 2;
        let new_y = monitor.y + (monitor.height - new_height) / 2;

        self.set_window_frame(
            window,
            &WindowFrame::new(new_x, new_y, new_width, new_height),
        )?;
        debug!(
            "Set fixed ratio {} at scale {}: {}x{} at ({}, {})",
            ratio, next_scale, new_width, new_height, new_x, new_y
        );
        Ok(())
    }

    /// Set window to native monitor aspect ratio
    pub fn set_native_ratio(
        &self,
        window: A::WindowId,
        scale_index: Option<usize>,
    ) -> Result<()> {
        let rect = self.api.get_window_rect(window)?;
        let monitors = self.api.get_monitors();
        let monitor = find_monitor_for_point(&monitors, rect.x, rect.y)
            .ok_or_else(|| anyhow::anyhow!("No monitors found"))?;
        let ratio = monitor.width as f32 / monitor.height as f32;
        self.set_fixed_ratio(window, ratio, scale_index)
    }

    /// Toggle window topmost state
    pub fn toggle_topmost(&self, window: A::WindowId) -> Result<bool> {
        let current = self.api.is_topmost(window);
        let new_state = !current;
        self.api.set_topmost(window, new_state)?;
        Ok(new_state)
    }
}

// ============================================================================
// Trait Implementations for Generic WindowManager
// ============================================================================

/// Implementation of WindowOperations for the generic WindowManager
/// This is only available when the platform's WindowId is the same as the common WindowId
impl<A: WindowApiBase<WindowId = WindowId>> WindowOperations for WindowManager<A> {
    fn get_window_info(&self, window: WindowId) -> Result<WindowInfo> {
        let title = self.api.get_window_title(window).unwrap_or_default();
        let rect = self.api.get_window_rect(window)?;

        Ok(WindowInfo {
            id: window,
            title,
            process_name: String::new(),
            executable_path: None,
            x: rect.x,
            y: rect.y,
            width: rect.width,
            height: rect.height,
        })
    }

    fn set_window_pos(
        &self,
        window: WindowId,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<()> {
        self.api.ensure_window_restored(window)?;
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
}

impl<A: WindowApiBase<WindowId = WindowId>> WindowStateQueries for WindowManager<A> {
    fn is_window_valid(&self, window: WindowId) -> bool {
        self.api.is_window_valid(window)
    }

    fn is_minimized(&self, window: WindowId) -> bool {
        self.api.is_minimized(window)
    }

    fn is_maximized(&self, window: WindowId) -> bool {
        self.api.is_maximized(window)
    }

    fn is_topmost(&self, window: WindowId) -> bool {
        self.api.is_topmost(window)
    }
}

impl<A: WindowApiBase<WindowId = WindowId>> ForegroundWindowOperations
    for WindowManager<A>
{
    fn get_foreground_window(&self) -> Option<WindowId> {
        self.api.get_foreground_window()
    }

    fn set_topmost(&self, window: WindowId, topmost: bool) -> Result<()> {
        self.api.set_topmost(window, topmost)
    }
}

impl<A: WindowApiBase<WindowId = WindowId>> MonitorOperations for WindowManager<A> {
    fn get_monitors(&self) -> Vec<crate::platform::types::MonitorInfo> {
        self.api.get_monitors()
    }

    fn move_to_monitor(&self, _window: WindowId, _monitor_index: usize) -> Result<()> {
        // Default implementation - platforms should override this
        anyhow::bail!("move_to_monitor not implemented on this platform")
    }
}

impl<A: WindowApiBase<WindowId = WindowId>> WindowManagerExt for WindowManager<A> {
    fn move_to_center(&self, window: WindowId) -> Result<()> {
        self.move_to_center(window)
    }

    fn move_to_edge(&self, window: WindowId, edge: Edge) -> Result<()> {
        self.move_to_edge(window, edge)
    }

    fn set_half_screen(&self, window: WindowId, edge: Edge) -> Result<()> {
        self.set_half_screen(window, edge)
    }

    fn loop_width(&self, window: WindowId, align: Alignment) -> Result<()> {
        self.loop_width(window, align)
    }

    fn loop_height(&self, window: WindowId, align: Alignment) -> Result<()> {
        self.loop_height(window, align)
    }

    fn set_fixed_ratio(
        &self,
        window: WindowId,
        ratio: f32,
        scale_index: Option<usize>,
    ) -> Result<()> {
        self.set_fixed_ratio(window, ratio, scale_index)
    }

    fn set_native_ratio(
        &self,
        window: WindowId,
        scale_index: Option<usize>,
    ) -> Result<()> {
        self.set_native_ratio(window, scale_index)
    }

    fn toggle_topmost(&self, window: WindowId) -> Result<bool> {
        self.toggle_topmost(window)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_monitor() -> crate::platform::types::MonitorInfo {
        crate::platform::types::MonitorInfo {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        }
    }

    fn test_window_frame() -> WindowFrame {
        WindowFrame::new(100, 100, 800, 600)
    }

    #[test]
    fn test_find_monitor_for_point() {
        let monitors = vec![test_monitor()];
        let m = find_monitor_for_point(&monitors, 500, 500).unwrap();
        assert_eq!(m.width, 1920);
    }

    #[test]
    fn test_find_next_ratio() {
        let ratios: [f32; 5] = [0.75, 0.6, 0.5, 0.4, 0.25];
        assert!((find_next_ratio(&ratios, 0.75) - 0.6).abs() < 0.001);
        assert!((find_next_ratio(&ratios, 0.25) - 0.75).abs() < 0.001);
    }
}
