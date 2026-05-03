//! Mock window API for testing
//!
//! Provides a mock implementation of WindowApi that can be used
//! in unit tests without platform dependencies.

use crate::platform::traits::WindowManager;
use crate::platform::types::{MonitorInfo, WindowId};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Mutex;

/// Mock window manager for testing
pub struct MockWindowManager {
    windows: Mutex<HashMap<WindowId, MockWindow>>,
    foreground_window: Mutex<Option<WindowId>>,
    monitors: Mutex<Vec<MonitorInfo>>,
}

#[derive(Debug, Clone)]
struct MockWindow {
    title: String,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    minimized: bool,
    maximized: bool,
    topmost: bool,
}

impl MockWindowManager {
    pub fn new() -> Self {
        Self {
            windows: Mutex::new(HashMap::new()),
            foreground_window: Mutex::new(None),
            monitors: Mutex::new(vec![MonitorInfo {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            }]),
        }
    }

    pub fn add_window(
        &self,
        id: WindowId,
        title: &str,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) {
        self.windows.lock().unwrap().insert(
            id,
            MockWindow {
                title: title.to_string(),
                x,
                y,
                width,
                height,
                minimized: false,
                maximized: false,
                topmost: false,
            },
        );
    }

    pub fn set_foreground_window(&self, id: WindowId) {
        *self.foreground_window.lock().unwrap() = Some(id);
    }

    pub fn set_monitors(&self, monitors: Vec<MonitorInfo>) {
        *self.monitors.lock().unwrap() = monitors;
    }
}

impl Default for MockWindowManager {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowManager for MockWindowManager {
    fn get_foreground_window(&self) -> Option<WindowId> {
        *self.foreground_window.lock().unwrap()
    }

    fn get_window_info(
        &self,
        window: WindowId,
    ) -> Result<crate::platform::types::WindowInfo> {
        let windows = self.windows.lock().unwrap();
        let w = windows
            .get(&window)
            .ok_or_else(|| anyhow::anyhow!("Window not found"))?;

        Ok(crate::platform::types::WindowInfo {
            id: window,
            title: w.title.clone(),
            process_name: String::new(),
            executable_path: None,
            x: w.x,
            y: w.y,
            width: w.width,
            height: w.height,
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
        let mut windows = self.windows.lock().unwrap();
        let w = windows
            .get_mut(&window)
            .ok_or_else(|| anyhow::anyhow!("Window not found"))?;
        w.x = x;
        w.y = y;
        w.width = width;
        w.height = height;
        w.minimized = false;
        w.maximized = false;
        Ok(())
    }

    fn minimize_window(&self, window: WindowId) -> Result<()> {
        let mut windows = self.windows.lock().unwrap();
        let w = windows
            .get_mut(&window)
            .ok_or_else(|| anyhow::anyhow!("Window not found"))?;
        w.minimized = true;
        w.maximized = false;
        Ok(())
    }

    fn maximize_window(&self, window: WindowId) -> Result<()> {
        let mut windows = self.windows.lock().unwrap();
        let w = windows
            .get_mut(&window)
            .ok_or_else(|| anyhow::anyhow!("Window not found"))?;
        w.minimized = false;
        w.maximized = true;
        Ok(())
    }

    fn restore_window(&self, window: WindowId) -> Result<()> {
        let mut windows = self.windows.lock().unwrap();
        let w = windows
            .get_mut(&window)
            .ok_or_else(|| anyhow::anyhow!("Window not found"))?;
        w.minimized = false;
        w.maximized = false;
        Ok(())
    }

    fn close_window(&self, window: WindowId) -> Result<()> {
        self.windows.lock().unwrap().remove(&window);
        Ok(())
    }

    fn set_topmost(&self, window: WindowId, topmost: bool) -> Result<()> {
        let mut windows = self.windows.lock().unwrap();
        let w = windows
            .get_mut(&window)
            .ok_or_else(|| anyhow::anyhow!("Window not found"))?;
        w.topmost = topmost;
        Ok(())
    }

    fn is_topmost(&self, window: WindowId) -> bool {
        self.windows
            .lock()
            .unwrap()
            .get(&window)
            .map(|w| w.topmost)
            .unwrap_or(false)
    }

    fn is_window_valid(&self, window: WindowId) -> bool {
        self.windows.lock().unwrap().contains_key(&window)
    }

    fn is_minimized(&self, window: WindowId) -> bool {
        self.windows
            .lock()
            .unwrap()
            .get(&window)
            .map(|w| w.minimized)
            .unwrap_or(false)
    }

    fn is_maximized(&self, window: WindowId) -> bool {
        self.windows
            .lock()
            .unwrap()
            .get(&window)
            .map(|w| w.maximized)
            .unwrap_or(false)
    }

    fn get_monitors(&self) -> Vec<MonitorInfo> {
        self.monitors.lock().unwrap().clone()
    }

    fn move_to_monitor(&self, _window: WindowId, _monitor_index: usize) -> Result<()> {
        Ok(())
    }

    fn switch_to_next_window_of_same_process(&self) -> Result<()> {
        Ok(())
    }

    fn move_to_center(&self, window: WindowId) -> Result<()> {
        let monitors = self.get_monitors();
        let monitor = monitors
            .first()
            .ok_or_else(|| anyhow::anyhow!("No monitors"))?;

        let info = self.get_window_info(window)?;
        let new_x = monitor.x + (monitor.width - info.width) / 2;
        let new_y = monitor.y + (monitor.height - info.height) / 2;
        self.set_window_pos(window, new_x, new_y, info.width, info.height)
    }

    fn move_to_edge(&self, window: WindowId, edge: crate::types::Edge) -> Result<()> {
        let monitors = self.get_monitors();
        let monitor = monitors
            .first()
            .ok_or_else(|| anyhow::anyhow!("No monitors"))?;

        let info = self.get_window_info(window)?;
        let (new_x, new_y) = match edge {
            crate::types::Edge::Left => (monitor.x, info.y),
            crate::types::Edge::Right => {
                (monitor.x + monitor.width - info.width, info.y)
            }
            crate::types::Edge::Top => (info.x, monitor.y),
            crate::types::Edge::Bottom => {
                (info.x, monitor.y + monitor.height - info.height)
            }
        };
        self.set_window_pos(window, new_x, new_y, info.width, info.height)
    }

    fn set_half_screen(&self, window: WindowId, edge: crate::types::Edge) -> Result<()> {
        let monitors = self.get_monitors();
        let monitor = monitors
            .first()
            .ok_or_else(|| anyhow::anyhow!("No monitors"))?;

        let (new_x, new_y, new_width, new_height) = match edge {
            crate::types::Edge::Left => {
                (monitor.x, monitor.y, monitor.width / 2, monitor.height)
            }
            crate::types::Edge::Right => {
                let w = monitor.width / 2;
                (monitor.x + monitor.width - w, monitor.y, w, monitor.height)
            }
            crate::types::Edge::Top => {
                (monitor.x, monitor.y, monitor.width, monitor.height / 2)
            }
            crate::types::Edge::Bottom => {
                let h = monitor.height / 2;
                (monitor.x, monitor.y + monitor.height - h, monitor.width, h)
            }
        };
        self.set_window_pos(window, new_x, new_y, new_width, new_height)
    }

    fn loop_width(
        &self,
        window: WindowId,
        align: crate::types::Alignment,
    ) -> Result<()> {
        const WIDTH_RATIOS: [f32; 5] = [0.75, 0.6, 0.5, 0.4, 0.25];

        let monitors = self.get_monitors();
        let monitor = monitors
            .first()
            .ok_or_else(|| anyhow::anyhow!("No monitors"))?;

        let info = self.get_window_info(window)?;
        let current_ratio = info.width as f32 / monitor.width as f32;
        let next_ratio =
            crate::platform::traits::find_next_ratio(&WIDTH_RATIOS, current_ratio);

        let new_width = (monitor.width as f32 * next_ratio) as i32;
        let new_x = match align {
            crate::types::Alignment::Left => monitor.x,
            crate::types::Alignment::Right => monitor.x + monitor.width - new_width,
            _ => info.x,
        };
        self.set_window_pos(window, new_x, info.y, new_width, info.height)
    }

    fn loop_height(
        &self,
        window: WindowId,
        align: crate::types::Alignment,
    ) -> Result<()> {
        const HEIGHT_RATIOS: [f32; 3] = [0.75, 0.5, 0.25];

        let monitors = self.get_monitors();
        let monitor = monitors
            .first()
            .ok_or_else(|| anyhow::anyhow!("No monitors"))?;

        let info = self.get_window_info(window)?;
        let current_ratio = info.height as f32 / monitor.height as f32;
        let next_ratio =
            crate::platform::traits::find_next_ratio(&HEIGHT_RATIOS, current_ratio);

        let new_height = (monitor.height as f32 * next_ratio) as i32;
        let new_y = match align {
            crate::types::Alignment::Top => monitor.y,
            crate::types::Alignment::Bottom => monitor.y + monitor.height - new_height,
            _ => info.y,
        };
        self.set_window_pos(window, info.x, new_y, info.width, new_height)
    }

    fn set_fixed_ratio(
        &self,
        window: WindowId,
        ratio: f32,
        scale_index: Option<usize>,
    ) -> Result<()> {
        const SCALES: [f32; 4] = [1.0, 0.9, 0.7, 0.5];

        let monitors = self.get_monitors();
        let monitor = monitors
            .first()
            .ok_or_else(|| anyhow::anyhow!("No monitors"))?;

        let base_size = std::cmp::min(monitor.width, monitor.height);
        let base_width = (base_size as f32 * ratio) as i32;
        let base_height = base_size;

        let scale = scale_index
            .and_then(|i| SCALES.get(i))
            .copied()
            .unwrap_or(1.0);

        let new_width = (base_width as f32 * scale) as i32;
        let new_height = (base_height as f32 * scale) as i32;
        let new_x = monitor.x + (monitor.width - new_width) / 2;
        let new_y = monitor.y + (monitor.height - new_height) / 2;

        self.set_window_pos(window, new_x, new_y, new_width, new_height)
    }

    fn set_native_ratio(
        &self,
        window: WindowId,
        scale_index: Option<usize>,
    ) -> Result<()> {
        let monitors = self.get_monitors();
        let monitor = monitors
            .first()
            .ok_or_else(|| anyhow::anyhow!("No monitors"))?;
        let ratio = monitor.width as f32 / monitor.height as f32;
        self.set_fixed_ratio(window, ratio, scale_index)
    }

    fn toggle_topmost(&self, window: WindowId) -> Result<bool> {
        let current = self.is_topmost(window);
        let new_state = !current;
        self.set_topmost(window, new_state)?;
        Ok(new_state)
    }
}
