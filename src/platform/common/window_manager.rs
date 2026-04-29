//! Common window manager logic shared across platforms.
//!
//! Provides a generic [`WindowManager<A>`] struct with inherent methods
//! for window operations (move, resize, monitor switching, aspect ratio).
//! Platform-specific modules wrap this via the [`impl_window_manager_types!`]
//! macro and implement the component traits for [`WindowManagerTrait`] dispatch.

use crate::platform::traits::{
    find_monitor_for_point, ForegroundWindowOperations, MonitorDirection, MonitorInfo,
    MonitorOperations, WindowApiBase, WindowFrame, WindowId, WindowInfo,
    WindowManagerTrait, WindowOperations, WindowStateQueries,
};
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

    pub fn get_window_info(&self, window: A::WindowId) -> Result<WindowInfo> {
        self.api.get_window_info(window)
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
        let info = self.api.get_window_info(window)?;
        let monitors = self.api.get_monitors();

        let monitor = find_monitor_for_point(&monitors, info.x, info.y)
            .ok_or_else(|| anyhow::anyhow!("No monitors found"))?;

        let frame = WindowFrame::new(info.x, info.y, info.width, info.height);
        let new_x = monitor.x + (monitor.width - frame.width) / 2;
        let new_y = monitor.y + (monitor.height - frame.height) / 2;

        self.set_window_frame(
            window,
            &WindowFrame::new(new_x, new_y, info.width, info.height),
        )?;
        debug!("Moved window to center: ({}, {})", new_x, new_y);
        Ok(())
    }

    /// Move window to edge of screen
    pub fn move_to_edge(&self, window: A::WindowId, edge: Edge) -> Result<()> {
        let info = self.api.get_window_info(window)?;
        let monitors = self.api.get_monitors();
        let monitor = find_monitor_for_point(&monitors, info.x, info.y)
            .ok_or_else(|| anyhow::anyhow!("No monitors found"))?;

        let (new_x, new_y) = match edge {
            Edge::Left => (monitor.x, info.y),
            Edge::Right => (monitor.x + monitor.width - info.width, info.y),
            Edge::Top => (info.x, monitor.y),
            Edge::Bottom => (info.x, monitor.y + monitor.height - info.height),
        };

        self.set_window_frame(
            window,
            &WindowFrame::new(new_x, new_y, info.width, info.height),
        )?;
        debug!("Moved window to {:?} edge: ({}, {})", edge, new_x, new_y);
        Ok(())
    }

    /// Set window to half screen (left/right/top/bottom)
    pub fn set_half_screen(&self, window: A::WindowId, edge: Edge) -> Result<()> {
        let info = self.api.get_window_info(window)?;
        let monitors = self.api.get_monitors();
        let monitor = find_monitor_for_point(&monitors, info.x, info.y)
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

        let info = self.api.get_window_info(window)?;
        let monitors = self.api.get_monitors();
        let monitor = find_monitor_for_point(&monitors, info.x, info.y)
            .ok_or_else(|| anyhow::anyhow!("No monitors found"))?;

        let current_ratio = info.width as f32 / monitor.width as f32;

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
            _ => info.x,
        };

        self.set_window_frame(
            window,
            &WindowFrame::new(new_x, info.y, new_width, info.height),
        )?;
        debug!("Looped width to {} (ratio: {})", new_width, next_ratio);
        Ok(())
    }

    /// Loop through common heights for the current window position
    pub fn loop_height(&self, window: A::WindowId, align: Alignment) -> Result<()> {
        const HEIGHT_RATIOS: [f32; 3] = [0.75, 0.5, 0.25];

        let info = self.api.get_window_info(window)?;
        let monitors = self.api.get_monitors();
        let monitor = find_monitor_for_point(&monitors, info.x, info.y)
            .ok_or_else(|| anyhow::anyhow!("No monitors found"))?;

        let current_ratio = info.height as f32 / monitor.height as f32;

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
            _ => info.y,
        };

        self.set_window_frame(
            window,
            &WindowFrame::new(info.x, new_y, info.width, new_height),
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

        let info = self.api.get_window_info(window)?;
        let monitors = self.api.get_monitors();
        let monitor = find_monitor_for_point(&monitors, info.x, info.y)
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
                let current_width_ratio = info.width as f32 / base_width as f32;
                let current_height_ratio = info.height as f32 / base_height as f32;
                let current_scale = (current_width_ratio + current_height_ratio) / 2.0;

                let mut next = SCALES[0];
                for (i, scale) in SCALES.iter().enumerate() {
                    if (current_scale - scale).abs() < 0.05 {
                        next = SCALES[(i + 1) % SCALES.len()];
                        break;
                    }
                }
                next
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
            "Set fixed ratio {} -> {}x{} at ({}, {})",
            ratio, new_width, new_height, new_x, new_y
        );
        Ok(())
    }

    /// Set window to its "native" content ratio and cycle sizes
    pub fn set_native_ratio(
        &self,
        window: A::WindowId,
        scale_index: Option<usize>,
    ) -> Result<()> {
        let info = self.api.get_window_info(window)?;
        let monitors = self.api.get_monitors();
        let monitor = find_monitor_for_point(&monitors, info.x, info.y)
            .ok_or_else(|| anyhow::anyhow!("No monitors found"))?;

        let screen_ratio = monitor.width as f32 / monitor.height as f32;
        self.set_fixed_ratio(window, screen_ratio, scale_index)
    }

    /// Toggle window topmost state, returns the new state
    pub fn toggle_topmost(&self, window: A::WindowId) -> Result<bool> {
        if !self.api.is_window_valid(window) {
            return Err(anyhow::anyhow!("Invalid window handle"));
        }

        let current = self.api.is_topmost(window);
        let new_state = !current;
        self.api.set_topmost(window, new_state)?;
        debug!("Toggled topmost: {} -> {}", current, new_state);
        Ok(new_state)
    }

    /// Move window to another monitor
    pub fn move_to_monitor(
        &self,
        window: A::WindowId,
        direction: MonitorDirection,
    ) -> Result<()> {
        let monitors = self.api.get_monitors();
        if monitors.len() < 2 {
            debug!("Only one monitor, nothing to do");
            return Ok(());
        }

        let info = self.api.get_window_info(window)?;

        let current_monitor_index = monitors
            .iter()
            .position(|m| {
                info.x >= m.x
                    && info.x < m.x + m.width
                    && info.y >= m.y
                    && info.y < m.y + m.height
            })
            .unwrap_or(0);

        let target_index = match direction {
            MonitorDirection::Next => (current_monitor_index + 1) % monitors.len(),
            MonitorDirection::Prev => {
                if current_monitor_index == 0 {
                    monitors.len() - 1
                } else {
                    current_monitor_index - 1
                }
            }
            MonitorDirection::Index(idx) => {
                let idx = idx as usize;
                if idx >= monitors.len() {
                    return Err(anyhow::anyhow!("Invalid monitor index: {}", idx));
                }
                idx
            }
        };

        let target_monitor = &monitors[target_index];
        let current_monitor = &monitors[current_monitor_index];

        let rel_x = (info.x - current_monitor.x) as f32 / current_monitor.width as f32;
        let rel_y = (info.y - current_monitor.y) as f32 / current_monitor.height as f32;
        let rel_width = info.width as f32 / current_monitor.width as f32;
        let rel_height = info.height as f32 / current_monitor.height as f32;

        let new_x = target_monitor.x + (rel_x * target_monitor.width as f32) as i32;
        let new_y = target_monitor.y + (rel_y * target_monitor.height as f32) as i32;
        let new_width = (rel_width * target_monitor.width as f32) as i32;
        let new_height = (rel_height * target_monitor.height as f32) as i32;

        self.set_window_frame(
            window,
            &WindowFrame::new(new_x, new_y, new_width, new_height),
        )?;

        debug!(
            "Moved window from monitor {} to monitor {}: ({}, {}) {}x{}",
            current_monitor_index, target_index, new_x, new_y, new_width, new_height
        );

        Ok(())
    }
}

impl<A: WindowApiBase + Send + Sync + 'static> WindowOperations for WindowManager<A> {
    fn get_window_info(&self, window: WindowId) -> Result<WindowInfo> {
        let id = A::usize_to_window_id(window);
        let info = self.api.get_window_info(id)?;
        Ok(WindowInfo {
            id: window,
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
        window: WindowId,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<()> {
        let id = A::usize_to_window_id(window);
        let frame = WindowFrame::new(x, y, width, height);
        self.set_window_frame(id, &frame)
    }

    fn minimize_window(&self, window: WindowId) -> Result<()> {
        let id = A::usize_to_window_id(window);
        self.api.minimize_window(id)
    }

    fn maximize_window(&self, window: WindowId) -> Result<()> {
        let id = A::usize_to_window_id(window);
        self.api.maximize_window(id)
    }

    fn restore_window(&self, window: WindowId) -> Result<()> {
        let id = A::usize_to_window_id(window);
        self.api.restore_window(id)
    }

    fn close_window(&self, window: WindowId) -> Result<()> {
        let id = A::usize_to_window_id(window);
        self.api.close_window(id)
    }
}

// Implement WindowStateQueries trait
impl<A: WindowApiBase + Send + Sync + 'static> WindowStateQueries for WindowManager<A> {
    fn is_window_valid(&self, window: WindowId) -> bool {
        let id = A::usize_to_window_id(window);
        self.api.is_window_valid(id)
    }

    fn is_minimized(&self, window: WindowId) -> bool {
        let id = A::usize_to_window_id(window);
        self.api.is_minimized(id)
    }

    fn is_maximized(&self, window: WindowId) -> bool {
        let id = A::usize_to_window_id(window);
        self.api.is_maximized(id)
    }

    fn is_topmost(&self, window: WindowId) -> bool {
        let id = A::usize_to_window_id(window);
        self.api.is_topmost(id)
    }
}

// Implement MonitorOperations trait
impl<A: WindowApiBase + Send + Sync + 'static> MonitorOperations for WindowManager<A> {
    fn get_monitors(&self) -> Vec<MonitorInfo> {
        self.api.get_monitors()
    }

    fn move_to_monitor(&self, window: WindowId, monitor_index: usize) -> Result<()> {
        let id = A::usize_to_window_id(window);
        self.move_to_monitor(id, MonitorDirection::Index(monitor_index as i32))
    }
}

// Implement ForegroundWindowOperations trait
impl<A: WindowApiBase + Send + Sync + 'static> ForegroundWindowOperations
    for WindowManager<A>
{
    fn get_foreground_window(&self) -> Option<WindowId> {
        self.api
            .get_foreground_window()
            .map(|id| A::window_id_to_usize(id))
    }

    fn set_topmost(&self, window: WindowId, topmost: bool) -> Result<()> {
        let id = A::usize_to_window_id(window);
        self.api.set_topmost(id, topmost)
    }
}

// WindowManagerTrait is automatically implemented since all component traits are implemented
impl<A: WindowApiBase + Send + Sync + 'static> WindowManagerTrait for WindowManager<A> {}

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
}
