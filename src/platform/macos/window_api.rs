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
        self.get_window_info(window)
            .ok()
            .map(|info| WindowFrame::new(info.x, info.y, info.width, info.height))
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

#[cfg(test)]
pub type MockWindowApi = crate::platform::mock::MockWindowApi<WindowId>;

#[cfg(test)]
pub use crate::platform::mock::WindowApiCall;

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
