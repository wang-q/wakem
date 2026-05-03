//! Mock macOS window API for testing
#![cfg(target_os = "macos")]
#![cfg(test)]

use crate::platform::traits::{
    MonitorInfo, MonitorWorkArea, WindowApiBase, WindowFrame, WindowId,
};
use anyhow::{anyhow, Result};

#[derive(Clone)]
pub struct MockMacosWindowApi {
    windows: std::sync::Arc<
        std::sync::Mutex<
            std::collections::HashMap<WindowId, crate::platform::types::WindowInfo>,
        >,
    >,
    foreground: std::sync::Arc<std::sync::Mutex<Option<WindowId>>>,
    monitors: std::sync::Arc<std::sync::Mutex<Vec<MonitorInfo>>>,
    minimized: std::sync::Arc<std::sync::Mutex<std::collections::HashSet<WindowId>>>,
}

impl MockMacosWindowApi {
    pub fn new() -> Self {
        let mut windows = std::collections::HashMap::new();
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
            windows: std::sync::Arc::new(std::sync::Mutex::new(windows)),
            foreground: std::sync::Arc::new(std::sync::Mutex::new(Some(1))),
            monitors: std::sync::Arc::new(std::sync::Mutex::new(vec![MonitorInfo {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            }])),
            minimized: std::sync::Arc::new(std::sync::Mutex::new(
                std::collections::HashSet::new(),
            )),
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

impl Default for MockMacosWindowApi {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowApiBase for MockMacosWindowApi {
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
        if let Some(mut info) = self.windows.lock().unwrap().get_mut(&window) {
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
            if let Some(mut info) = self.windows.lock().unwrap().get_mut(&window) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_creation() {
        let mock = MockMacosWindowApi::new();
        assert!(mock.is_window_valid(1));
        assert!(!mock.is_window_valid(999));
    }

    #[test]
    fn test_mock_set_window_pos() {
        let mock = MockMacosWindowApi::new();
        mock.set_window_pos(1, 200, 300, 1024, 768).unwrap();
        let rect = mock.get_window_rect(1).unwrap();
        assert_eq!(rect.x, 200);
        assert_eq!(rect.y, 300);
        assert_eq!(rect.width, 1024);
        assert_eq!(rect.height, 768);
    }

    #[test]
    fn test_mock_minimize_restore() {
        let mock = MockMacosWindowApi::new();
        assert!(!mock.is_minimized(1));

        mock.minimize_window(1).unwrap();
        assert!(mock.is_minimized(1));

        mock.restore_window(1).unwrap();
        assert!(!mock.is_minimized(1));
    }

    #[test]
    fn test_mock_maximize() {
        let mock = MockMacosWindowApi::new();
        mock.maximize_window(1).unwrap();
        assert!(mock.is_maximized(1));

        let rect = mock.get_window_rect(1).unwrap();
        assert_eq!(rect.width, 1920);
        assert_eq!(rect.height, 1080);
    }

    #[test]
    fn test_mock_close_window() {
        let mock = MockMacosWindowApi::new();
        assert!(mock.is_window_valid(1));
        mock.close_window(1).unwrap();
        assert!(!mock.is_window_valid(1));
    }

    #[test]
    fn test_mock_add_window() {
        let mock = MockMacosWindowApi::new();
        mock.add_window(
            2,
            crate::platform::types::WindowInfo {
                id: 2,
                title: "Second Window".to_string(),
                process_name: "OtherApp".to_string(),
                executable_path: None,
                x: 500,
                y: 500,
                width: 640,
                height: 480,
            },
        );
        assert!(mock.is_window_valid(2));
        assert_eq!(mock.get_window_title(2), Some("Second Window".to_string()));
    }

    #[test]
    fn test_mock_foreground_window() {
        let mock = MockMacosWindowApi::new();
        assert_eq!(mock.get_foreground_window(), Some(1));
    }
}
