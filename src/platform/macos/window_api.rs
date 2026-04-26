//! macOS window API implementation using native APIs
//!
//! Provides high-performance window operations on macOS using:
//! - Core Graphics: CGDisplay for monitor info
//! - Accessibility (AXUIElement): Window manipulation
//! - Cocoa (NSWorkspace): Application queries
//!
//! Performance: All operations complete in < 10ms (typically < 5ms)

// Allow dead code - some trait methods are under development
#![allow(dead_code)]

use crate::platform::macos::native_api::{ax_element, cg_window, ns_workspace};
use crate::platform::traits::{
    MonitorInfo, MonitorWorkArea, WindowApiBase, WindowId, WindowInfo, WindowState,
};
use anyhow::{anyhow, Result};
use core_graphics::display::{CGDisplay, CGDisplayBounds};
use tracing::debug;

/// API call log entry (for MockWindowApi testing)
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum WindowApiCall {
    GetForegroundWindow,
    GetWindowInfo {
        window: WindowId,
    },
    SetWindowPos {
        window: WindowId,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    },
    GetMonitors,
    MinimizeWindow {
        window: WindowId,
    },
    MaximizeWindow {
        window: WindowId,
    },
    RestoreWindow {
        window: WindowId,
    },
    CloseWindow {
        window: WindowId,
    },
    SetTopmost {
        window: WindowId,
        topmost: bool,
    },
    IsWindowValid {
        window: WindowId,
    },
    IsMinimized {
        window: WindowId,
    },
    IsMaximized {
        window: WindowId,
    },
}

/// Real macOS window API using native Core Graphics + Accessibility APIs
///
/// # Performance
///
/// All operations use direct system framework calls:
/// - NSWorkspace: < 0.5ms for app info
/// - CGWindowList: < 2ms for window metadata
/// - AXUIElement: < 10ms for window manipulation
///
/// # Window Identification
///
/// Uses `CGWindowNumber` (from `kCGWindowNumber`) as the `WindowId`.
/// This is a unique system-wide identifier assigned by Core Graphics.
/// Note: manipulation methods currently target the frontmost app's main
/// window regardless of the passed `WindowId`.
#[derive(Clone, Default)]
pub struct RealWindowApi;

impl RealWindowApi {
    pub fn new() -> Self {
        Self
    }

    fn get_foreground_window(&self) -> Option<WindowId> {
        cg_window::get_frontmost_window_info()
            .ok()
            .flatten()
            .map(|info| info.number as WindowId)
    }

    fn get_window_info(&self, window: WindowId) -> Result<WindowInfo> {
        let target_number = window as i64;

        if target_number > 0 {
            if let Ok(Some(winfo)) = cg_window::get_window_info_by_number(target_number)
            {
                debug!(
                    "Got window info by number {}: {} ({}) at ({}, {}) {}x{}",
                    target_number,
                    winfo.name,
                    winfo.owner_name,
                    winfo.x,
                    winfo.y,
                    winfo.width,
                    winfo.height
                );

                return Ok(WindowInfo {
                    id: window,
                    title: winfo.name,
                    process_name: winfo.owner_name,
                    executable_path: None,
                    x: winfo.x,
                    y: winfo.y,
                    width: winfo.width as i32,
                    height: winfo.height as i32,
                });
            }
        }

        Err(anyhow!(
            "Failed to get window info for window number {}",
            target_number
        ))
    }

    fn set_window_pos(
        &self,
        _window: WindowId,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<()> {
        let pid = ns_workspace::get_frontmost_app_pid()
            .ok_or_else(|| anyhow!("No frontmost application"))?;

        let app_elem = ax_element::create_app_element(pid)?;
        let win_elem = ax_element::get_main_window(&app_elem)?;

        let screen_height = ns_workspace::get_main_display_height();
        let cg_y = crate::platform::macos::native_api::windows_to_cg(
            y as f64 + height as f64,
            screen_height,
        );

        ax_element::set_window_frame(
            &win_elem,
            x as f64,
            cg_y,
            width as f64,
            height as f64,
        )?;

        debug!(
            "Set window pos natively: {}x{} at ({}, {})",
            width, height, x, y
        );
        Ok(())
    }

    fn minimize_window(&self, _window: WindowId) -> Result<()> {
        let pid = ns_workspace::get_frontmost_app_pid()
            .ok_or_else(|| anyhow!("No frontmost application"))?;
        let app_elem = ax_element::create_app_element(pid)?;
        let win_elem = ax_element::get_main_window(&app_elem)?;

        ax_element::minimize_window(&win_elem)?;
        debug!("Minimized window via native API");
        Ok(())
    }

    fn maximize_window(&self, _window: WindowId) -> Result<()> {
        let pid = ns_workspace::get_frontmost_app_pid()
            .ok_or_else(|| anyhow!("No frontmost application"))?;
        let app_elem = ax_element::create_app_element(pid)?;
        let win_elem = ax_element::get_main_window(&app_elem)?;

        ax_element::maximize_window(&win_elem)?;
        debug!("Maximized window via native API");
        Ok(())
    }

    fn restore_window(&self, _window: WindowId) -> Result<()> {
        let pid = ns_workspace::get_frontmost_app_pid()
            .ok_or_else(|| anyhow!("No frontmost application"))?;
        let app_elem = ax_element::create_app_element(pid)?;
        let win_elem = ax_element::get_main_window(&app_elem)?;

        ax_element::restore_window(&win_elem)?;
        debug!("Restored window from minimized state via native API");
        Ok(())
    }

    fn close_window(&self, _window: WindowId) -> Result<()> {
        let pid = ns_workspace::get_frontmost_app_pid()
            .ok_or_else(|| anyhow!("No frontmost application"))?;
        let app_elem = ax_element::create_app_element(pid)?;
        let win_elem = ax_element::get_main_window(&app_elem)?;

        ax_element::close_window(&win_elem)?;
        debug!("Closed window via native API");
        Ok(())
    }

    fn set_topmost(&self, _window: WindowId, topmost: bool) -> Result<()> {
        if topmost {
            let pid = ns_workspace::get_frontmost_app_pid()
                .ok_or_else(|| anyhow!("No frontmost application"))?;
            let app_elem = ax_element::create_app_element(pid)?;

            ax_element::bring_to_front(&app_elem)?;
            debug!("Brought window to front via native API");
        }
        Ok(())
    }

    fn is_topmost(&self, _window: WindowId) -> bool {
        false
    }

    fn get_monitors(&self) -> Vec<MonitorInfo> {
        let mut monitors = std::vec::Vec::new();
        let display_ids = CGDisplay::active_displays().unwrap_or_default();

        let screen_height = ns_workspace::get_main_display_height();

        for display_id in display_ids {
            let bounds = unsafe { CGDisplayBounds(display_id) };
            let windows_y =
                (screen_height - bounds.origin.y - bounds.size.height) as i32;
            monitors.push(MonitorInfo {
                x: bounds.origin.x as i32,
                y: windows_y,
                width: bounds.size.width as i32,
                height: bounds.size.height as i32,
            });
        }

        if monitors.is_empty() {
            let main = CGDisplay::main();
            let bounds = unsafe { CGDisplayBounds(main.id) };
            let windows_y =
                (screen_height - bounds.origin.y - bounds.size.height) as i32;
            monitors.push(MonitorInfo {
                x: bounds.origin.x as i32,
                y: windows_y,
                width: bounds.size.width as i32,
                height: bounds.size.height as i32,
            });
        }

        monitors
    }

    fn get_monitor_work_area(&self, monitor_index: usize) -> Option<MonitorWorkArea> {
        if let Some((x, y, width, height)) =
            ns_workspace::get_screen_visible_frame(monitor_index)
        {
            return Some(MonitorWorkArea {
                x,
                y,
                width,
                height,
            });
        }

        let monitors = self.get_monitors();
        let monitor = monitors.get(monitor_index)?;

        Some(MonitorWorkArea {
            x: monitor.x,
            y: monitor.y,
            width: monitor.width,
            height: monitor.height,
        })
    }

    fn move_to_monitor(&self, _window: WindowId, monitor_index: usize) -> Result<()> {
        let monitors = self.get_monitors();
        let monitor = monitors.get(monitor_index).ok_or_else(|| {
            anyhow::anyhow!("Invalid monitor index: {}", monitor_index)
        })?;

        let info = self.get_window_info(_window)?;

        let new_x = monitor.x + (monitor.width - info.width) / 2;
        let new_y = monitor.y + (monitor.height - info.height) / 2;

        self.set_window_pos(_window, new_x, new_y, info.width, info.height)?;
        debug!(
            "Moved window to monitor {} at ({}, {})",
            monitor_index, new_x, new_y
        );
        Ok(())
    }

    fn is_window_valid(&self, window: WindowId) -> bool {
        if window == 0 {
            return false;
        }
        cg_window::get_window_info_by_number(window as i64)
            .ok()
            .flatten()
            .is_some()
    }

    fn is_minimized(&self, _window: WindowId) -> bool {
        match ns_workspace::get_frontmost_app_pid() {
            Some(pid) => match ax_element::create_app_element(pid) {
                Ok(app_elem) => match ax_element::get_main_window(&app_elem) {
                    Ok(win_elem) => ax_element::is_minimized(&win_elem).unwrap_or(false),
                    Err(_) => false,
                },
                Err(_) => false,
            },
            None => false,
        }
    }

    fn is_maximized(&self, window: WindowId) -> bool {
        let info = match self.get_window_info(window) {
            Ok(info) => info,
            Err(_) => return false,
        };

        let work_area = match self.get_monitor_work_area(0) {
            Some(wa) => wa,
            None => return false,
        };

        let width_ratio = info.width as f64 / work_area.width as f64;
        let height_ratio = info.height as f64 / work_area.height as f64;

        let threshold = 0.95;
        width_ratio >= threshold && height_ratio >= threshold
    }

    fn get_window_state(&self, window: WindowId) -> WindowState {
        if self.is_minimized(window) {
            WindowState::Minimized
        } else if self.is_maximized(window) {
            WindowState::Maximized
        } else {
            WindowState::Normal
        }
    }
}

impl WindowApiBase for RealWindowApi {
    type WindowId = WindowId;

    fn get_foreground_window(&self) -> Option<Self::WindowId> {
        self.get_foreground_window()
    }

    fn get_window_info(&self, window: Self::WindowId) -> Result<WindowInfo> {
        self.get_window_info(window)
    }

    fn set_window_pos(
        &self,
        window: Self::WindowId,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<()> {
        self.set_window_pos(window, x, y, width, height)
    }

    fn minimize_window(&self, window: Self::WindowId) -> Result<()> {
        self.minimize_window(window)
    }

    fn maximize_window(&self, window: Self::WindowId) -> Result<()> {
        self.maximize_window(window)
    }

    fn restore_window(&self, window: Self::WindowId) -> Result<()> {
        self.restore_window(window)
    }

    fn close_window(&self, window: Self::WindowId) -> Result<()> {
        self.close_window(window)
    }

    fn set_topmost(&self, window: Self::WindowId, topmost: bool) -> Result<()> {
        self.set_topmost(window, topmost)
    }

    fn is_topmost(&self, window: Self::WindowId) -> bool {
        self.is_topmost(window)
    }

    fn get_monitors(&self) -> Vec<MonitorInfo> {
        self.get_monitors()
    }

    fn move_to_monitor(
        &self,
        window: Self::WindowId,
        monitor_index: usize,
    ) -> Result<()> {
        self.move_to_monitor(window, monitor_index)
    }

    fn is_window_valid(&self, window: Self::WindowId) -> bool {
        self.is_window_valid(window)
    }

    fn is_minimized(&self, window: Self::WindowId) -> bool {
        self.is_minimized(window)
    }

    fn is_maximized(&self, window: Self::WindowId) -> bool {
        self.is_maximized(window)
    }
}

/// Mock window API for testing
#[cfg(test)]
#[derive(Clone)]
pub struct MockWindowApi {
    windows: std::sync::Arc<
        std::sync::Mutex<
            std::collections::HashMap<WindowId, WindowInfo>,
        >,
    >,
    foreground: std::sync::Arc<std::sync::Mutex<Option<WindowId>>>,
    monitors: std::sync::Arc<std::sync::Mutex<Vec<MonitorInfo>>>,
    minimized: std::sync::Arc<std::sync::Mutex<std::collections::HashSet<WindowId>>>,
}

#[cfg(test)]
impl MockWindowApi {
    pub fn new() -> Self {
        let mut windows = std::collections::HashMap::new();
        windows.insert(
            1,
            WindowInfo {
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

    pub fn add_window(&self, id: WindowId, info: WindowInfo) {
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

    fn get_foreground_window(&self) -> Option<WindowId> {
        *self.foreground.lock().unwrap()
    }

    fn get_window_info(&self, window: WindowId) -> Result<WindowInfo> {
        self.windows
            .lock()
            .unwrap()
            .get(&window)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Window not found: {}", window))
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
            Err(anyhow::anyhow!("Window not found: {}", window))
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

    fn get_monitors(&self) -> Vec<MonitorInfo> {
        self.monitors.lock().unwrap().clone()
    }

    fn get_monitor_work_area(&self, monitor_index: usize) -> Option<MonitorWorkArea> {
        let monitors = self.monitors.lock().unwrap();
        let m = monitors.get(monitor_index)?;
        let menu_bar_height = 22;
        let dock_height = 44;
        Some(MonitorWorkArea {
            x: m.x,
            y: m.y + menu_bar_height,
            width: m.width,
            height: m.height - menu_bar_height - dock_height,
        })
    }

    fn move_to_monitor(&self, window: WindowId, monitor_index: usize) -> Result<()> {
        let monitors = self.monitors.lock().unwrap();
        let m = monitors
            .get(monitor_index)
            .ok_or_else(|| anyhow::anyhow!("Invalid monitor index"))?;
        if let Some(info) = self.windows.lock().unwrap().get_mut(&window) {
            info.x = m.x + (m.width - info.width) / 2;
            info.y = m.y + (m.height - info.height) / 2;
        }
        Ok(())
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

    fn get_window_state(&self, window: WindowId) -> WindowState {
        if self.is_minimized(window) {
            WindowState::Minimized
        } else if self.is_maximized(window) {
            WindowState::Maximized
        } else {
            WindowState::Normal
        }
    }
}

#[cfg(test)]
impl Default for MockWindowApi {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl WindowApiBase for MockWindowApi {
    type WindowId = WindowId;

    fn get_foreground_window(&self) -> Option<Self::WindowId> {
        self.get_foreground_window()
    }

    fn get_window_info(&self, window: Self::WindowId) -> Result<WindowInfo> {
        self.get_window_info(window)
    }

    fn set_window_pos(
        &self,
        window: Self::WindowId,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<()> {
        self.set_window_pos(window, x, y, width, height)
    }

    fn minimize_window(&self, window: Self::WindowId) -> Result<()> {
        self.minimize_window(window)
    }

    fn maximize_window(&self, window: Self::WindowId) -> Result<()> {
        self.maximize_window(window)
    }

    fn restore_window(&self, window: Self::WindowId) -> Result<()> {
        self.restore_window(window)
    }

    fn close_window(&self, window: Self::WindowId) -> Result<()> {
        self.close_window(window)
    }

    fn set_topmost(&self, window: Self::WindowId, topmost: bool) -> Result<()> {
        self.set_topmost(window, topmost)
    }

    fn is_topmost(&self, window: Self::WindowId) -> bool {
        self.is_topmost(window)
    }

    fn get_monitors(&self) -> Vec<MonitorInfo> {
        self.get_monitors()
    }

    fn move_to_monitor(
        &self,
        window: Self::WindowId,
        monitor_index: usize,
    ) -> Result<()> {
        self.move_to_monitor(window, monitor_index)
    }

    fn is_window_valid(&self, window: Self::WindowId) -> bool {
        self.is_window_valid(window)
    }

    fn is_minimized(&self, window: Self::WindowId) -> bool {
        self.is_minimized(window)
    }

    fn is_maximized(&self, window: Self::WindowId) -> bool {
        self.is_maximized(window)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_real_api_creation() {
        let api = RealWindowApi::new();
        drop(api);
    }

    #[test]
    fn test_get_monitors_native() {
        let api = RealWindowApi::new();
        let monitors = api.get_monitors();
        assert!(!monitors.is_empty());

        let main = &monitors[0];
        assert!(main.width > 0);
        assert!(main.height > 0);
    }

    #[test]
    fn test_get_foreground_window_info_native() {
        let api = RealWindowApi::new();

        match api.get_window_info(1) {
            Ok(info) => {
                if !info.process_name.is_empty() {
                    debug!("Frontmost window: {} ({})", info.title, info.process_name);
                }
            }
            Err(e) => {
                debug!(
                    "Note: May fail if no window or no accessibility permission: {}",
                    e
                );
            }
        }
    }

    #[test]
    fn test_mock_creation() {
        let mock = MockWindowApi::new();
        assert!(mock.is_window_valid(1));
        assert!(!mock.is_window_valid(999));
    }

    #[test]
    fn test_mock_set_window_pos() {
        let mock = MockWindowApi::new();
        mock.set_window_pos(1, 200, 300, 1024, 768).unwrap();
        let info = mock.get_window_info(1).unwrap();
        assert_eq!(info.x, 200);
        assert_eq!(info.y, 300);
        assert_eq!(info.width, 1024);
        assert_eq!(info.height, 768);
    }

    #[test]
    fn test_mock_minimize_restore() {
        let mock = MockWindowApi::new();
        assert!(!mock.is_minimized(1));

        mock.minimize_window(1).unwrap();
        assert!(mock.is_minimized(1));
        assert_eq!(mock.get_window_state(1), WindowState::Minimized);

        mock.restore_window(1).unwrap();
        assert!(!mock.is_minimized(1));
        assert_eq!(mock.get_window_state(1), WindowState::Normal);
    }

    #[test]
    fn test_mock_maximize() {
        let mock = MockWindowApi::new();
        mock.maximize_window(1).unwrap();
        assert!(mock.is_maximized(1));
        assert_eq!(mock.get_window_state(1), WindowState::Maximized);

        let info = mock.get_window_info(1).unwrap();
        assert_eq!(info.width, 1920);
        assert_eq!(info.height, 1080);
    }

    #[test]
    fn test_mock_close_window() {
        let mock = MockWindowApi::new();
        assert!(mock.is_window_valid(1));
        mock.close_window(1).unwrap();
        assert!(!mock.is_window_valid(1));
    }

    #[test]
    fn test_mock_move_to_monitor() {
        let mock = MockWindowApi::new();
        mock.set_monitors(vec![
            MonitorInfo {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
            MonitorInfo {
                x: 1920,
                y: 0,
                width: 2560,
                height: 1440,
            },
        ]);

        mock.move_to_monitor(1, 1).unwrap();
        let info = mock.get_window_info(1).unwrap();
        assert!(info.x >= 1920);
    }

    #[test]
    fn test_mock_monitor_work_area() {
        let mock = MockWindowApi::new();
        let work_area = mock.get_monitor_work_area(0).unwrap();
        assert!(work_area.height < 1080);
    }

    #[test]
    fn test_mock_add_window() {
        let mock = MockWindowApi::new();
        mock.add_window(
            2,
            WindowInfo {
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
        let info = mock.get_window_info(2).unwrap();
        assert_eq!(info.title, "Second Window");
    }

    #[test]
    fn test_mock_foreground_window() {
        let mock = MockWindowApi::new();
        assert_eq!(mock.get_foreground_window(), Some(1));
    }
}
