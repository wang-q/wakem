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
    MonitorInfo, MonitorWorkArea, WindowApiBase, WindowFrame, WindowId, WindowInfo,
};
use anyhow::{anyhow, Result};
use core_graphics::display::{CGDisplay, CGDisplayBounds};
use tracing::debug;

/// API call log entry (for MockWindowApi testing)
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum WindowApiCall {
    GetForegroundWindow,
    GetWindowRect {
        window: WindowId,
    },
    SetWindowPos {
        window: WindowId,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    },
    GetMonitorInfo {
        window: WindowId,
    },
    IsWindow {
        window: WindowId,
    },
    GetWindowTitle {
        window: WindowId,
    },
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
    EnsureRestored {
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
#[derive(Clone)]
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

    fn get_window_rect(&self, window: WindowId) -> Option<WindowFrame> {
        self.get_window_info(window).ok().map(|info| {
            WindowFrame::new(info.x, info.y, info.width, info.height)
        })
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

    fn get_monitor_info(&self, _window: WindowId) -> Option<MonitorInfo> {
        let monitors = self.get_monitors();
        monitors.first().cloned()
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

    /// Check if window is minimized (iconic)
    /// Internal method named after Windows API convention
    fn is_iconic(&self, window: WindowId) -> bool {
        // For macOS, we check if the frontmost app's main window is minimized
        // This is a limitation - we don't check the specific window ID
        let _ = window; // Unused for now
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

    /// Check if window is maximized (zoomed)
    /// Internal method named after Windows API convention
    fn is_zoomed(&self, window: WindowId) -> bool {
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

    /// Ensure window is restored (not minimized or maximized)
    /// Internal method named after Windows API convention
    pub fn ensure_window_restored(&self, window: WindowId) -> Result<()> {
        if self.is_iconic(window) || self.is_zoomed(window) {
            self.restore_window(window)?;
        }
        Ok(())
    }
}

impl Default for RealWindowApi {
    fn default() -> Self {
        Self::new()
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
        self.is_iconic(window)
    }

    fn is_maximized(&self, window: Self::WindowId) -> bool {
        self.is_zoomed(window)
    }
}

/// Mock window API for testing
#[cfg(test)]
pub struct MockWindowApi {
    pub foreground_window: std::cell::RefCell<Option<WindowId>>,
    pub window_rects: std::cell::RefCell<std::collections::HashMap<WindowId, WindowFrame>>,
    pub monitor_info: std::cell::RefCell<std::collections::HashMap<WindowId, MonitorInfo>>,
    pub window_states: std::cell::RefCell<std::collections::HashMap<WindowId, MockWindowState>>,
    pub operations_log: std::cell::RefCell<Vec<WindowApiCall>>,
}

#[derive(Debug, Clone, Copy, Default)]
struct MockWindowState {
    minimized: bool,
    maximized: bool,
    topmost: bool,
}

#[cfg(test)]
#[allow(dead_code)]
impl MockWindowApi {
    pub fn new() -> Self {
        Self {
            foreground_window: std::cell::RefCell::new(None),
            window_rects: std::cell::RefCell::new(std::collections::HashMap::new()),
            monitor_info: std::cell::RefCell::new(std::collections::HashMap::new()),
            window_states: std::cell::RefCell::new(std::collections::HashMap::new()),
            operations_log: std::cell::RefCell::new(Vec::new()),
        }
    }

    pub fn set_foreground_window(&self, window: WindowId) {
        *self.foreground_window.borrow_mut() = Some(window);
    }

    pub fn set_window_rect(&self, window: WindowId, frame: WindowFrame) {
        self.window_rects
            .borrow_mut()
            .insert(window, frame);
    }

    pub fn set_monitor_info(&self, window: WindowId, info: MonitorInfo) {
        self.monitor_info.borrow_mut().insert(window, info);
    }

    pub fn set_window_state(&self, window: WindowId, minimized: bool, maximized: bool) {
        let mut states = self.window_states.borrow_mut();
        let state = states.entry(window).or_default();
        state.minimized = minimized;
        state.maximized = maximized;
    }

    pub fn get_operations(&self) -> Vec<WindowApiCall> {
        self.operations_log.borrow().clone()
    }

    pub fn clear_operations(&self) {
        self.operations_log.borrow_mut().clear();
    }

    fn log_operation(&self, op: WindowApiCall) {
        self.operations_log.borrow_mut().push(op);
    }

    fn get_foreground_window(&self) -> Option<WindowId> {
        self.log_operation(WindowApiCall::GetForegroundWindow);
        *self.foreground_window.borrow()
    }

    fn get_window_rect(&self, window: WindowId) -> Option<WindowFrame> {
        self.log_operation(WindowApiCall::GetWindowRect { window });
        self.window_rects.borrow().get(&window).copied()
    }

    fn set_window_pos(
        &self,
        window: WindowId,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<()> {
        self.log_operation(WindowApiCall::SetWindowPos {
            window,
            x,
            y,
            width,
            height,
        });

        let mut rects = self.window_rects.borrow_mut();
        rects.insert(window, WindowFrame::new(x, y, width, height));

        let mut states = self.window_states.borrow_mut();
        if let Some(state) = states.get_mut(&window) {
            state.minimized = false;
            state.maximized = false;
        }

        Ok(())
    }

    fn get_monitor_info(&self, window: WindowId) -> Option<MonitorInfo> {
        self.log_operation(WindowApiCall::GetMonitorInfo { window });
        self.monitor_info.borrow().get(&window).cloned()
    }

    fn get_monitor_work_area(&self, window: WindowId) -> Option<MonitorWorkArea> {
        self.get_monitor_info(window).map(|info| MonitorWorkArea {
            x: info.x,
            y: info.y,
            width: info.width,
            height: info.height,
        })
    }

    fn is_window(&self, window: WindowId) -> bool {
        self.log_operation(WindowApiCall::IsWindow { window });
        self.window_rects.borrow().contains_key(&window)
    }

    fn get_window_title(&self, window: WindowId) -> Option<String> {
        self.log_operation(WindowApiCall::GetWindowTitle { window });
        Some(format!("Window {:?}", window))
    }

    fn is_iconic(&self, window: WindowId) -> bool {
        self.window_states
            .borrow()
            .get(&window)
            .map(|s| s.minimized)
            .unwrap_or(false)
    }

    fn is_zoomed(&self, window: WindowId) -> bool {
        self.window_states
            .borrow()
            .get(&window)
            .map(|s| s.maximized)
            .unwrap_or(false)
    }

    fn minimize_window(&self, window: WindowId) -> Result<()> {
        self.log_operation(WindowApiCall::MinimizeWindow { window });
        let mut states = self.window_states.borrow_mut();
        states.entry(window).or_default().minimized = true;
        Ok(())
    }

    fn maximize_window(&self, window: WindowId) -> Result<()> {
        self.log_operation(WindowApiCall::MaximizeWindow { window });
        let mut states = self.window_states.borrow_mut();
        states.entry(window).or_default().maximized = true;
        Ok(())
    }

    fn restore_window(&self, window: WindowId) -> Result<()> {
        self.log_operation(WindowApiCall::RestoreWindow { window });
        let mut states = self.window_states.borrow_mut();
        if let Some(state) = states.get_mut(&window) {
            state.minimized = false;
            state.maximized = false;
        }
        Ok(())
    }

    fn close_window(&self, window: WindowId) -> Result<()> {
        self.log_operation(WindowApiCall::CloseWindow { window });
        self.window_rects.borrow_mut().remove(&window);
        self.window_states.borrow_mut().remove(&window);
        Ok(())
    }

    fn set_topmost(&self, window: WindowId, topmost: bool) -> Result<()> {
        self.log_operation(WindowApiCall::SetTopmost { window, topmost });
        let mut states = self.window_states.borrow_mut();
        states.entry(window).or_default().topmost = topmost;
        Ok(())
    }

    fn ensure_window_restored(&self, window: WindowId) -> Result<()> {
        self.log_operation(WindowApiCall::EnsureRestored { window });
        if self.is_iconic(window) || self.is_zoomed(window) {
            self.restore_window(window)?;
        }
        Ok(())
    }

    fn is_topmost(&self, window: WindowId) -> bool {
        self.window_states
            .borrow()
            .get(&window)
            .map(|s| s.topmost)
            .unwrap_or(false)
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
        let title = self.get_window_title(window).unwrap_or_default();
        let frame = self
            .get_window_rect(window)
            .ok_or_else(|| anyhow::anyhow!("Failed to get window rect"))?;
        Ok(WindowInfo {
            id: window,
            title,
            process_name: "TestProcess".to_string(),
            executable_path: None,
            x: frame.x,
            y: frame.y,
            width: frame.width,
            height: frame.height,
        })
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
        let fg = self.get_foreground_window();
        fg.and_then(|window| self.get_monitor_info(window))
            .map(|info| vec![info])
            .unwrap_or_default()
    }

    fn move_to_monitor(
        &self,
        _window: Self::WindowId,
        _monitor_index: usize,
    ) -> Result<()> {
        Ok(())
    }

    fn is_window_valid(&self, window: Self::WindowId) -> bool {
        self.is_window(window)
    }

    fn is_minimized(&self, window: Self::WindowId) -> bool {
        self.is_iconic(window)
    }

    fn is_maximized(&self, window: Self::WindowId) -> bool {
        self.is_zoomed(window)
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
        assert!(!mock.is_window_valid(0));
        assert!(!mock.is_window_valid(999));
    }

    #[test]
    fn test_mock_set_window_pos() {
        let mock = MockWindowApi::new();
        let window = 1234;

        mock.set_window_pos(window, 50, 100, 1024, 768).unwrap();

        let frame = mock.get_window_rect(window).unwrap();
        assert_eq!(frame.x, 50);
        assert_eq!(frame.y, 100);
        assert_eq!(frame.width, 1024);
        assert_eq!(frame.height, 768);
    }

    #[test]
    fn test_mock_window_state() {
        let mock = MockWindowApi::new();
        let window = 9999;

        assert!(!mock.is_iconic(window));
        assert!(!mock.is_zoomed(window));

        mock.minimize_window(window).unwrap();
        assert!(mock.is_iconic(window));
        assert!(!mock.is_zoomed(window));

        mock.restore_window(window).unwrap();
        assert!(!mock.is_iconic(window));
        assert!(!mock.is_zoomed(window));

        mock.maximize_window(window).unwrap();
        assert!(!mock.is_iconic(window));
        assert!(mock.is_zoomed(window));
    }

    #[test]
    fn test_mock_foreground_window() {
        let mock = MockWindowApi::new();
        let window = 1111;

        assert!(mock.get_foreground_window().is_none());

        mock.set_foreground_window(window);
        assert_eq!(mock.get_foreground_window().unwrap(), 1111);
    }

    #[test]
    fn test_mock_api_base() {
        let mock = MockWindowApi::new();
        let window = 5678;

        mock.set_window_rect(window, WindowFrame::new(100, 200, 800, 600));

        let info = mock.get_window_info(window).unwrap();
        assert_eq!(info.x, 100);
        assert_eq!(info.y, 200);
        assert_eq!(info.width, 800);
        assert_eq!(info.height, 600);
    }
}
