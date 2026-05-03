//! Common window manager logic shared across platforms.
//!
//! Provides a generic [`WindowManager<A>`] struct that implements the
//! component traits (`WindowOperations`, `WindowStateQueries`, etc.).
//! High-level operations (center, half-screen, etc.) come from the
//! [`WindowManagerExt`] blanket impl in `traits.rs`.

use crate::platform::traits::{
    find_monitor_for_point, ForegroundWindowOperations, MonitorOperations,
    WindowApiBase, WindowOperations, WindowStateQueries, WindowSwitching,
};
use crate::platform::types::{WindowId, WindowInfo};
use anyhow::Result;

/// Generic window manager implementation shared across platforms
///
/// Delegates to platform-specific `WindowApiBase` implementations.
/// Both Windows and macOS use this struct, with platform-specific
/// extensions in their respective modules.
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
}

impl<A: WindowApiBase + Default> WindowManager<A> {
    pub fn new() -> Self {
        Self::with_api(A::default())
    }
}

impl<A: WindowApiBase + Default> Default for WindowManager<A> {
    fn default() -> Self {
        Self::new()
    }
}

impl<A: WindowApiBase<WindowId = WindowId>> WindowOperations for WindowManager<A> {
    fn get_window_info(&self, window: WindowId) -> Result<WindowInfo> {
        let title = self.api.get_window_title(window).unwrap_or_default();
        let rect = self.api.get_window_rect(window)?;
        let process_name = self.api.get_process_name(window).unwrap_or_default();
        let executable_path = self.api.get_executable_path(window);

        Ok(WindowInfo {
            id: window,
            title,
            process_name,
            executable_path,
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

    fn move_to_monitor(&self, window: WindowId, monitor_index: usize) -> Result<()> {
        let monitors = self.get_monitors();
        if monitors.len() < 2 {
            return Ok(());
        }
        if monitor_index >= monitors.len() {
            anyhow::bail!("Invalid monitor index: {}", monitor_index);
        }

        let info = self.get_window_info(window)?;
        let current_idx = find_monitor_for_point(&monitors, info.x, info.y)
            .ok_or_else(|| anyhow::anyhow!("No monitor found for window"))?;
        let current_idx = monitors
            .iter()
            .position(|m| m.x == current_idx.x && m.y == current_idx.y)
            .ok_or_else(|| anyhow::anyhow!("Monitor index not found"))?;
        let current_monitor = &monitors[current_idx];
        let target_monitor = &monitors[monitor_index];

        let current_work = self
            .api
            .get_monitor_work_area(current_idx)
            .unwrap_or_else(|| (*current_monitor).into());
        let target_work = self
            .api
            .get_monitor_work_area(monitor_index)
            .unwrap_or_else(|| (*target_monitor).into());

        let rel_x = (info.x - current_work.x) as f32 / current_work.width as f32;
        let rel_y = (info.y - current_work.y) as f32 / current_work.height as f32;
        let rel_width = info.width as f32 / current_work.width as f32;
        let rel_height = info.height as f32 / current_work.height as f32;

        let new_x = target_work.x + (rel_x * target_work.width as f32) as i32;
        let new_y = target_work.y + (rel_y * target_work.height as f32) as i32;
        let new_width = (rel_width * target_work.width as f32) as i32;
        let new_height = (rel_height * target_work.height as f32) as i32;

        self.set_window_pos(window, new_x, new_y, new_width, new_height)
    }
}

impl<A: WindowApiBase<WindowId = WindowId> + Send + Sync> WindowSwitching
    for WindowManager<A>
{
    fn switch_to_next_window_of_same_process(&self) -> Result<()> {
        self.api().switch_to_next_window_of_same_process()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_monitor_for_point() {
        let monitors = vec![crate::platform::types::MonitorInfo {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        }];
        let m = find_monitor_for_point(&monitors, 500, 500).unwrap();
        assert_eq!(m.width, 1920);
    }

    #[test]
    fn test_find_next_ratio() {
        let ratios: [f32; 5] = [0.75, 0.6, 0.5, 0.4, 0.25];
        assert!(
            (crate::platform::traits::find_next_ratio(&ratios, 0.75) - 0.6).abs()
                < 0.001
        );
        assert!(
            (crate::platform::traits::find_next_ratio(&ratios, 0.25) - 0.75).abs()
                < 0.001
        );
    }
}
