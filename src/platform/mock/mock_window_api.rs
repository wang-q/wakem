//! Mock window API for testing
//!
//! Provides mock implementations of window-related traits that can be used
//! in unit tests without platform dependencies.

use crate::platform::traits::{
    ForegroundWindowOperations, MonitorOperations, WindowApiBase, WindowOperations,
    WindowStateQueries, WindowSwitching,
};
use crate::platform::types::{MonitorInfo, MonitorWorkArea, WindowFrame, WindowId};
use anyhow::{anyhow, Result};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

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

impl WindowOperations for MockWindowManager {
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
}

impl WindowStateQueries for MockWindowManager {
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
}

impl MonitorOperations for MockWindowManager {
    fn get_monitors(&self) -> Vec<MonitorInfo> {
        self.monitors.lock().unwrap().clone()
    }

    fn move_to_monitor(&self, _window: WindowId, _monitor_index: usize) -> Result<()> {
        Ok(())
    }
}

impl ForegroundWindowOperations for MockWindowManager {
    fn get_foreground_window(&self) -> Option<WindowId> {
        *self.foreground_window.lock().unwrap()
    }

    fn set_topmost(&self, window: WindowId, topmost: bool) -> Result<()> {
        let mut windows = self.windows.lock().unwrap();
        let w = windows
            .get_mut(&window)
            .ok_or_else(|| anyhow::anyhow!("Window not found"))?;
        w.topmost = topmost;
        Ok(())
    }
}

impl WindowSwitching for MockWindowManager {}

/// Mock implementation of [WindowApiBase] for testing
///
/// Unlike [MockWindowManager] which implements the high-level trait composition,
/// this implements the low-level [WindowApiBase] trait used by
/// [common::window_manager::WindowManager].
#[derive(Clone)]
pub struct MockWindowApiBase {
    windows: Arc<Mutex<HashMap<WindowId, crate::platform::types::WindowInfo>>>,
    foreground: Arc<Mutex<Option<WindowId>>>,
    monitors: Arc<Mutex<Vec<MonitorInfo>>>,
    minimized: Arc<Mutex<HashSet<WindowId>>>,
}

impl MockWindowApiBase {
    pub fn new() -> Self {
        let mut windows = HashMap::new();
        windows.insert(
            1,
            crate::platform::types::WindowInfo {
                id: 1,
                title: "Test Window".to_string(),
                process_name: "TestApp".to_string(),
                executable_path: None,
                x: 100,
                y: 100,
                width: 800,
                height: 600,
            },
        );

        Self {
            windows: Arc::new(Mutex::new(windows)),
            foreground: Arc::new(Mutex::new(Some(1))),
            monitors: Arc::new(Mutex::new(vec![MonitorInfo {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            }])),
            minimized: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub fn add_window(&self, id: WindowId, info: crate::platform::types::WindowInfo) {
        self.windows.lock().unwrap().insert(id, info);
    }

    pub fn set_monitors(&self, monitors: Vec<MonitorInfo>) {
        *self.monitors.lock().unwrap() = monitors;
    }

    pub fn set_minimized(&self, id: WindowId, minimized: bool) {
        if minimized {
            self.minimized.lock().unwrap().insert(id);
        } else {
            self.minimized.lock().unwrap().remove(&id);
        }
    }
}

impl Default for MockWindowApiBase {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowApiBase for MockWindowApiBase {
    type WindowId = WindowId;

    fn get_foreground_window(&self) -> Option<WindowId> {
        *self.foreground.lock().unwrap()
    }

    fn set_window_pos(
        &self,
        window: WindowId,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
    ) -> Result<()> {
        if let Some(info) = self.windows.lock().unwrap().get_mut(&window) {
            info.x = x;
            info.y = y;
            info.width = w;
            info.height = h;
            Ok(())
        } else {
            Err(anyhow!("Window not found: {}", window))
        }
    }

    fn minimize_window(&self, window: WindowId) -> Result<()> {
        self.set_minimized(window, true);
        Ok(())
    }

    fn maximize_window(&self, window: WindowId) -> Result<()> {
        let monitors = self.monitors.lock().unwrap();
        if let Some(monitor) = monitors.first() {
            if let Some(info) = self.windows.lock().unwrap().get_mut(&window) {
                info.x = monitor.x;
                info.y = monitor.y;
                info.width = monitor.width;
                info.height = monitor.height;
            }
        }
        Ok(())
    }

    fn restore_window(&self, window: WindowId) -> Result<()> {
        self.set_minimized(window, false);
        Ok(())
    }

    fn close_window(&self, window: WindowId) -> Result<()> {
        self.windows.lock().unwrap().remove(&window);
        Ok(())
    }

    fn set_topmost(&self, _window: WindowId, _topmost: bool) -> Result<()> {
        Ok(())
    }

    fn is_topmost(&self, _window: WindowId) -> bool {
        false
    }

    fn is_window_valid(&self, window: WindowId) -> bool {
        self.windows.lock().unwrap().contains_key(&window)
    }

    fn is_minimized(&self, window: WindowId) -> bool {
        self.minimized.lock().unwrap().contains(&window)
    }

    fn is_maximized(&self, window: WindowId) -> bool {
        if let Some(info) = self.windows.lock().unwrap().get(&window) {
            let monitors = self.monitors.lock().unwrap();
            if let Some(m) = monitors.first() {
                return info.width == m.width && info.height == m.height;
            }
        }
        false
    }

    fn get_window_title(&self, window: WindowId) -> Option<String> {
        self.windows
            .lock()
            .unwrap()
            .get(&window)
            .map(|i| i.title.clone())
    }

    fn get_window_rect(&self, window: WindowId) -> Result<WindowFrame> {
        self.windows
            .lock()
            .unwrap()
            .get(&window)
            .map(|i| WindowFrame::new(i.x, i.y, i.width, i.height))
            .ok_or_else(|| anyhow!("Window not found: {}", window))
    }

    fn get_monitors(&self) -> Vec<MonitorInfo> {
        self.monitors.lock().unwrap().clone()
    }

    fn get_monitor_work_area(&self, monitor_index: usize) -> Option<MonitorWorkArea> {
        let monitors = self.monitors.lock().unwrap();
        let m = monitors.get(monitor_index)?;
        Some(MonitorWorkArea {
            x: m.x,
            y: m.y,
            width: m.width,
            height: m.height,
        })
    }

    fn get_process_name(&self, window: WindowId) -> Option<String> {
        self.windows
            .lock()
            .unwrap()
            .get(&window)
            .map(|i| i.process_name.clone())
    }
}
