//! Mock window API for testing
//!
//! Provides a mock implementation of WindowApi that can be used
//! in unit tests without platform dependencies.

use crate::platform::traits::WindowManager;
use crate::platform::types::{MonitorInfo, WindowFrame, WindowId};
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
}
