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

    fn window_id_to_usize(id: Self::WindowId) -> usize {
        id
    }

    fn usize_to_window_id(id: usize) -> Self::WindowId {
        id
    }

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

    #[test]
    fn test_window_event_hook_creation() {
        let (tx, _rx) = std::sync::mpsc::channel::<PlatformWindowEvent>();
        let hook = WindowEventHook::new(tx);
        drop(hook);
    }

    #[test]
    fn test_event_hook_start_stop() {
        let (sender, receiver) = std::sync::mpsc::channel::<PlatformWindowEvent>();
        let mut hook = WindowEventHook::new(sender);

        hook.start().unwrap();

        std::thread::sleep(std::time::Duration::from_millis(100));

        hook.stop();

        drop(hook);
        drop(receiver);
    }

    #[test]
    fn test_shutdown_flag() {
        let (sender, _receiver) = std::sync::mpsc::channel::<PlatformWindowEvent>();
        let hook = WindowEventHook::new(sender);
        assert!(!hook.shutdown_flag().load(std::sync::atomic::Ordering::SeqCst));
    }

    #[test]
    fn test_get_frontmost_app_info() {
        match get_frontmost_app_info() {
            Ok((process, title, count)) => {
                println!("Frontmost: {} - {} ({} windows)", process, title, count);
                assert!(!process.is_empty());
            }
            Err(e) => {
                println!("Note: Could not get frontmost app info: {}", e);
            }
        }
    }
}

// ============================================================================
// Window Event Hook
// ============================================================================

use crate::platform::traits::PlatformWindowEvent;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use tracing::{info, trace};

/// Window event hook manager
pub struct WindowEventHook {
    event_sender: Sender<PlatformWindowEvent>,
    running: Arc<AtomicBool>,
    shutdown_flag: Arc<AtomicBool>,
    poll_interval_ms: u64,
    thread_handle: Option<std::thread::JoinHandle<()>>,
}

impl WindowEventHook {
    /// Create new window event hook
    pub fn new(event_sender: Sender<PlatformWindowEvent>) -> Self {
        Self {
            event_sender,
            running: Arc::new(AtomicBool::new(false)),
            shutdown_flag: Arc::new(AtomicBool::new(false)),
            poll_interval_ms: 200,
            thread_handle: None,
        }
    }

    #[allow(dead_code)]
    pub fn start(&mut self) -> Result<()> {
        self.start_with_shutdown(self.shutdown_flag.clone())
    }

    /// Start window event monitoring with shutdown flag for graceful exit
    pub fn start_with_shutdown(&mut self, shutdown_flag: Arc<AtomicBool>) -> Result<()> {
        if self.running.load(Ordering::SeqCst) {
            return Ok(());
        }

        self.shutdown_flag = shutdown_flag;
        self.shutdown_flag.store(false, Ordering::SeqCst);
        self.running.store(true, Ordering::SeqCst);

        let sender = self.event_sender.clone();
        let shutdown = self.shutdown_flag.clone();
        let is_running = self.running.clone();
        let poll_interval = self.poll_interval_ms;

        let handle = std::thread::spawn(move || {
            use std::time::Duration;

            let mut last_process = String::new();
            let mut last_title = String::new();
            let mut last_window_count: usize = 0;
            let mut initialized = false;

            while !shutdown.load(Ordering::SeqCst) {
                match get_frontmost_app_info() {
                    Ok((current_process, current_title, current_window_count)) => {
                        if !current_process.is_empty() {
                            if !initialized {
                                last_process = current_process;
                                last_title = current_title;
                                last_window_count = current_window_count;
                                initialized = true;
                            } else if current_process != last_process
                                || current_title != last_title
                            {
                                let _ =
                                    sender.send(PlatformWindowEvent::WindowActivated {
                                        process_name: current_process.clone(),
                                        window_title: current_title.clone(),
                                        window_id: 0,
                                    });
                                debug!(
                                    "Foreground window changed: {} - {}",
                                    current_process, current_title
                                );

                                last_process = current_process;
                                last_title = current_title;
                            }

                            if current_window_count != last_window_count && initialized {
                                if current_window_count > last_window_count {
                                    let _ = sender.send(
                                        PlatformWindowEvent::WindowCreated {
                                            process_name: last_process.clone(),
                                            window_title: last_title.clone(),
                                        },
                                    );
                                    debug!("Window created in {}", last_process);
                                } else if current_window_count < last_window_count {
                                    let _ =
                                        sender.send(PlatformWindowEvent::WindowClosed {
                                            process_name: last_process.clone(),
                                        });
                                    debug!("Window closed in {}", last_process);
                                }
                                last_window_count = current_window_count;
                            }
                        }
                    }
                    Err(e) => {
                        trace!("Failed to query foreground window: {}", e);
                    }
                }

                std::thread::sleep(Duration::from_millis(poll_interval));
            }

            is_running.store(false, Ordering::SeqCst);
            debug!("WindowEventHook thread stopped");
        });

        self.thread_handle = Some(handle);
        info!("WindowEventHook started (using native APIs)");

        Ok(())
    }

    /// Stop window event monitoring
    pub fn stop(&mut self) {
        self.shutdown_flag.store(true, Ordering::SeqCst);

        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
            debug!("WindowEventHook thread joined");
        }

        self.running.store(false, Ordering::SeqCst);
        debug!("WindowEventHook stopped");
    }

    /// Get shutdown flag reference
    pub fn shutdown_flag(&self) -> Arc<AtomicBool> {
        self.shutdown_flag.clone()
    }
}

impl Drop for WindowEventHook {
    fn drop(&mut self) {
        if self.running.load(Ordering::SeqCst) {
            self.stop();
        }
    }
}

fn get_frontmost_app_info() -> Result<(String, String, usize)> {
    use crate::platform::macos::native_api::cg_window::get_on_screen_windows;

    let windows = get_on_screen_windows()
        .map_err(|e| anyhow::anyhow!("Failed to get window list: {}", e))?;

    let frontmost = windows
        .iter()
        .rfind(|w| w.layer == 0 && !w.owner_name.is_empty());

    if let Some(window) = frontmost {
        let process_name = window.owner_name.clone();
        let window_title = window.name.clone();

        let window_count = windows
            .iter()
            .filter(|w| w.owner_name == process_name && w.layer == 0)
            .count();

        trace!(
            "Frontmost: {} - {} ({} windows)",
            process_name,
            window_title,
            window_count
        );

        Ok((process_name, window_title, window_count))
    } else {
        Err(anyhow::anyhow!("No frontmost window found"))
    }
}
