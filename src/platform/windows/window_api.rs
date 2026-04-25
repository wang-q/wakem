//! Windows window API implementation
#![cfg(target_os = "windows")]

use anyhow::Result;
#[allow(unused_imports)]
use std::cell::RefCell;
#[allow(unused_imports)]
use std::collections::HashMap;
use std::time::{Duration, Instant};
use windows::Win32::Foundation::{HWND, LPARAM, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{MonitorFromWindow, MONITOR_DEFAULTTONEAREST};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumChildWindows, EnumWindows, GetClassNameW, GetForegroundWindow, GetWindowRect,
    GetWindowTextW, IsIconic, IsWindow, IsWindowVisible, IsZoomed, SetWindowPos,
    ShowWindow, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOOWNERZORDER, SW_RESTORE,
};
use windows_core::BOOL;

use crate::platform::traits::{MonitorInfo, MonitorWorkArea, WindowFrame};

/// Window operation log (Windows-specific)
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum WindowOperation {
    GetForegroundWindow,
    GetWindowRect {
        hwnd: HWND,
    },
    SetWindowPos {
        hwnd: HWND,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    },
    GetMonitorInfo {
        hwnd: HWND,
    },
    IsWindow {
        hwnd: HWND,
    },
    GetWindowTitle {
        hwnd: HWND,
    },
    MinimizeWindow {
        hwnd: HWND,
    },
    MaximizeWindow {
        hwnd: HWND,
    },
    RestoreWindow {
        hwnd: HWND,
    },
    CloseWindow {
        hwnd: HWND,
    },
    SetTopmost {
        hwnd: HWND,
        topmost: bool,
    },
    EnsureRestored {
        hwnd: HWND,
    },
}

/// Window state (Windows-specific implementation details)
#[derive(Debug, Clone, Copy, Default)]
#[allow(dead_code)]
pub struct WindowStateDetail {
    pub minimized: bool,
    pub maximized: bool,
    pub topmost: bool,
}

/// Windows API abstract interface
#[allow(dead_code)]
pub trait WindowApi {
    /// Get foreground window handle
    fn get_foreground_window(&self) -> Option<HWND>;
    /// Get window rectangle
    fn get_window_rect(&self, hwnd: HWND) -> Option<WindowFrame>;
    /// Set window position
    fn set_window_pos(
        &self,
        hwnd: HWND,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<()>;
    /// Get monitor info
    fn get_monitor_info(&self, hwnd: HWND) -> Option<MonitorInfo>;
    /// Get monitor work area
    fn get_monitor_work_area(&self, hwnd: HWND) -> Option<MonitorWorkArea>;
    /// Check if window is valid
    fn is_window(&self, hwnd: HWND) -> bool;
    /// Get window title
    fn get_window_title(&self, hwnd: HWND) -> Option<String>;
    /// Check if window is minimized
    fn is_iconic(&self, hwnd: HWND) -> bool;
    /// Check if window is maximized
    fn is_zoomed(&self, hwnd: HWND) -> bool;
    /// Minimize window
    fn minimize_window(&self, hwnd: HWND) -> Result<()>;
    /// Maximize window
    fn maximize_window(&self, hwnd: HWND) -> Result<()>;
    /// Restore window
    fn restore_window(&self, hwnd: HWND) -> Result<()>;
    /// Close window
    fn close_window(&self, hwnd: HWND) -> Result<()>;
    /// Set topmost status
    fn set_topmost(&self, hwnd: HWND, topmost: bool) -> Result<()>;
    /// Check if window is topmost
    fn is_topmost(&self, hwnd: HWND) -> bool;
    /// Ensure window is restored
    fn ensure_window_restored(&self, hwnd: HWND) -> Result<()>;

    // ==================== New methods inspired by Win32::GuiTest ====================

    /// Wait for a window matching the given criteria to appear
    /// Returns the window handle if found within timeout, None otherwise
    fn wait_for_window(
        &self,
        title_pattern: Option<&str>,
        class_pattern: Option<&str>,
        timeout: Duration,
        poll_interval: Duration,
    ) -> Option<HWND>;

    /// Find windows matching the given criteria
    /// Supports regex patterns for title and class name
    fn find_windows(
        &self,
        title_pattern: Option<&str>,
        class_pattern: Option<&str>,
        visible_only: bool,
    ) -> Vec<HWND>;

    /// Get all child windows of a given parent window
    fn get_child_windows(&self, parent: HWND) -> Vec<HWND>;

    /// Get window class name
    fn get_window_class_name(&self, hwnd: HWND) -> Option<String>;

    /// Check if window is visible
    fn is_window_visible(&self, hwnd: HWND) -> bool;

    /// Get window text (content) - works for edit controls and similar
    fn get_window_text(&self, hwnd: HWND) -> Option<String>;

    /// Wait for window to become foreground/active
    fn wait_for_foreground_window(&self, hwnd: HWND, timeout: Duration) -> bool;

    // ==================== Mouse operations ====================

    /// Get current cursor position
    fn get_cursor_pos(&self) -> Option<(i32, i32)>;

    /// Set cursor position (absolute coordinates)
    fn set_cursor_pos(&self, x: i32, y: i32) -> Result<()>;

    /// Send left mouse button down
    fn send_lbutton_down(&self) -> Result<()>;

    /// Send left mouse button up
    fn send_lbutton_up(&self) -> Result<()>;

    /// Send right mouse button down
    fn send_rbutton_down(&self) -> Result<()>;

    /// Send right mouse button up
    fn send_rbutton_up(&self) -> Result<()>;

    /// Click at absolute coordinates (move + click)
    fn click_at(&self, x: i32, y: i32) -> Result<()>;
}

/// Real Windows API implementation
#[allow(dead_code)]
pub struct RealWindowApi;

#[allow(dead_code)]
impl RealWindowApi {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RealWindowApi {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowApi for RealWindowApi {
    fn get_foreground_window(&self) -> Option<HWND> {
        let hwnd = unsafe { GetForegroundWindow() };
        if hwnd.0.is_null() {
            None
        } else {
            Some(hwnd)
        }
    }

    fn get_window_rect(&self, hwnd: HWND) -> Option<WindowFrame> {
        unsafe {
            let mut rect = RECT::default();
            GetWindowRect(hwnd, &mut rect).ok()?;
            Some(WindowFrame::new(
                rect.left,
                rect.top,
                rect.right - rect.left,
                rect.bottom - rect.top,
            ))
        }
    }

    fn set_window_pos(
        &self,
        hwnd: HWND,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<()> {
        unsafe {
            SetWindowPos(
                hwnd,
                None,
                x,
                y,
                width,
                height,
                SWP_NOACTIVATE | SWP_NOOWNERZORDER | SWP_FRAMECHANGED,
            )?;
            Ok(())
        }
    }

    fn get_monitor_info(&self, hwnd: HWND) -> Option<MonitorInfo> {
        unsafe {
            let hmonitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
            if hmonitor.is_invalid() {
                return None;
            }

            use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MONITORINFO};
            let mut monitor_info = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };

            // GetMonitorInfoW returns BOOL, use as_bool() to check success
            if !GetMonitorInfoW(hmonitor, &mut monitor_info).as_bool() {
                return None;
            }

            let work_area = &monitor_info.rcWork;
            Some(MonitorInfo {
                x: work_area.left,
                y: work_area.top,
                width: work_area.right - work_area.left,
                height: work_area.bottom - work_area.top,
            })
        }
    }

    fn get_monitor_work_area(&self, hwnd: HWND) -> Option<MonitorWorkArea> {
        self.get_monitor_info(hwnd).map(|info| MonitorWorkArea {
            x: info.x,
            y: info.y,
            width: info.width,
            height: info.height,
        })
    }

    fn is_window(&self, hwnd: HWND) -> bool {
        unsafe { IsWindow(Some(hwnd)).as_bool() }
    }

    fn get_window_title(&self, hwnd: HWND) -> Option<String> {
        unsafe {
            let mut title_buffer = [0u16; 256];
            let len = GetWindowTextW(hwnd, &mut title_buffer);
            if len == 0 {
                None
            } else {
                Some(String::from_utf16_lossy(&title_buffer[..len as usize]))
            }
        }
    }

    fn is_iconic(&self, hwnd: HWND) -> bool {
        unsafe { IsIconic(hwnd).as_bool() }
    }

    fn is_zoomed(&self, hwnd: HWND) -> bool {
        unsafe { IsZoomed(hwnd).as_bool() }
    }

    fn minimize_window(&self, hwnd: HWND) -> Result<()> {
        unsafe {
            ShowWindow(hwnd, windows::Win32::UI::WindowsAndMessaging::SW_MINIMIZE)
                .ok()
                .map_err(|e| anyhow::anyhow!("Failed to minimize window: {}", e))?;
            Ok(())
        }
    }

    fn maximize_window(&self, hwnd: HWND) -> Result<()> {
        unsafe {
            ShowWindow(hwnd, windows::Win32::UI::WindowsAndMessaging::SW_MAXIMIZE)
                .ok()
                .map_err(|e| anyhow::anyhow!("Failed to maximize window: {}", e))?;
            Ok(())
        }
    }

    fn restore_window(&self, hwnd: HWND) -> Result<()> {
        unsafe {
            ShowWindow(hwnd, SW_RESTORE)
                .ok()
                .map_err(|e| anyhow::anyhow!("Failed to restore window: {}", e))?;
            Ok(())
        }
    }

    fn close_window(&self, hwnd: HWND) -> Result<()> {
        unsafe {
            use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_CLOSE};
            let _ = PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
            Ok(())
        }
    }

    fn set_topmost(&self, hwnd: HWND, topmost: bool) -> Result<()> {
        unsafe {
            use windows::Win32::UI::WindowsAndMessaging::{
                SetWindowPos, HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOMOVE, SWP_NOSIZE,
            };
            let pos = if topmost {
                Some(HWND_TOPMOST)
            } else {
                Some(HWND_NOTOPMOST)
            };
            let _ = SetWindowPos(hwnd, pos, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE);
            Ok(())
        }
    }

    fn is_topmost(&self, hwnd: HWND) -> bool {
        unsafe {
            use windows::Win32::UI::WindowsAndMessaging::WS_EX_TOPMOST;
            use windows::Win32::UI::WindowsAndMessaging::{
                GetWindowLongW, IsWindow, GWL_EXSTYLE,
            };

            // Check if window is valid first
            if !IsWindow(Some(hwnd)).as_bool() {
                return false;
            }

            let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
            (ex_style as u32) & WS_EX_TOPMOST.0 != 0
        }
    }

    fn ensure_window_restored(&self, hwnd: HWND) -> Result<()> {
        if self.is_iconic(hwnd) || self.is_zoomed(hwnd) {
            self.restore_window(hwnd)?;
        }
        Ok(())
    }

    // ==================== New methods implementation ====================

    fn wait_for_window(
        &self,
        title_pattern: Option<&str>,
        class_pattern: Option<&str>,
        timeout: Duration,
        poll_interval: Duration,
    ) -> Option<HWND> {
        let start = Instant::now();

        while start.elapsed() < timeout {
            let windows = self.find_windows(title_pattern, class_pattern, true);
            if let Some(&hwnd) = windows.first() {
                return Some(hwnd);
            }
            std::thread::sleep(poll_interval);
        }

        None
    }

    fn find_windows(
        &self,
        title_pattern: Option<&str>,
        class_pattern: Option<&str>,
        visible_only: bool,
    ) -> Vec<HWND> {
        use regex::Regex;

        let title_regex = title_pattern.and_then(|p| Regex::new(p).ok());
        let class_regex = class_pattern.and_then(|p| Regex::new(p).ok());

        let mut results = Vec::new();
        let context = EnumContext {
            title_regex,
            class_regex,
            visible_only,
            results: &mut results,
        };

        unsafe {
            let _ = EnumWindows(
                Some(enum_windows_callback),
                LPARAM(&context as *const _ as isize),
            );
        }

        results
    }

    fn get_child_windows(&self, parent: HWND) -> Vec<HWND> {
        let mut children = Vec::new();

        unsafe {
            let _ = EnumChildWindows(
                Some(parent),
                Some(enum_child_windows_callback),
                LPARAM(&mut children as *mut _ as isize),
            );
        }

        children
    }

    fn get_window_class_name(&self, hwnd: HWND) -> Option<String> {
        unsafe {
            let mut buffer = [0u16; 256];
            let len = GetClassNameW(hwnd, &mut buffer);
            if len == 0 {
                None
            } else {
                Some(String::from_utf16_lossy(&buffer[..len as usize]))
            }
        }
    }

    fn is_window_visible(&self, hwnd: HWND) -> bool {
        unsafe { IsWindowVisible(hwnd).as_bool() }
    }

    fn get_window_text(&self, hwnd: HWND) -> Option<String> {
        // This uses the same implementation as get_window_title for now
        // In the future, this could be enhanced to handle edit controls differently
        self.get_window_title(hwnd)
    }

    fn wait_for_foreground_window(&self, hwnd: HWND, timeout: Duration) -> bool {
        let start = Instant::now();

        while start.elapsed() < timeout {
            unsafe {
                let fg = GetForegroundWindow();
                if fg.0 == hwnd.0 {
                    return true;
                }
            }
            std::thread::sleep(Duration::from_millis(50));
        }

        false
    }

    fn get_cursor_pos(&self) -> Option<(i32, i32)> {
        unsafe {
            use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
            let mut point = windows::Win32::Foundation::POINT::default();
            if GetCursorPos(&mut point).is_ok() {
                Some((point.x, point.y))
            } else {
                None
            }
        }
    }

    fn set_cursor_pos(&self, x: i32, y: i32) -> Result<()> {
        unsafe {
            use windows::Win32::UI::WindowsAndMessaging::SetCursorPos;
            SetCursorPos(x, y)?;
            Ok(())
        }
    }

    fn send_lbutton_down(&self) -> Result<()> {
        unsafe {
            use windows::Win32::UI::Input::KeyboardAndMouse::{
                mouse_event, MOUSEEVENTF_LEFTDOWN,
            };
            mouse_event(MOUSEEVENTF_LEFTDOWN, 0, 0, 0, 0);
            Ok(())
        }
    }

    fn send_lbutton_up(&self) -> Result<()> {
        unsafe {
            use windows::Win32::UI::Input::KeyboardAndMouse::{
                mouse_event, MOUSEEVENTF_LEFTUP,
            };
            mouse_event(MOUSEEVENTF_LEFTUP, 0, 0, 0, 0);
            Ok(())
        }
    }

    fn send_rbutton_down(&self) -> Result<()> {
        unsafe {
            use windows::Win32::UI::Input::KeyboardAndMouse::{
                mouse_event, MOUSEEVENTF_RIGHTDOWN,
            };
            mouse_event(MOUSEEVENTF_RIGHTDOWN, 0, 0, 0, 0);
            Ok(())
        }
    }

    fn send_rbutton_up(&self) -> Result<()> {
        unsafe {
            use windows::Win32::UI::Input::KeyboardAndMouse::{
                mouse_event, MOUSEEVENTF_RIGHTUP,
            };
            mouse_event(MOUSEEVENTF_RIGHTUP, 0, 0, 0, 0);
            Ok(())
        }
    }

    fn click_at(&self, x: i32, y: i32) -> Result<()> {
        // Save current position
        let original_pos = self.get_cursor_pos();

        // Move to target position
        self.set_cursor_pos(x, y)?;

        // Small delay to ensure cursor moved
        std::thread::sleep(Duration::from_millis(50));

        // Click
        self.send_lbutton_down()?;
        std::thread::sleep(Duration::from_millis(50));
        self.send_lbutton_up()?;

        // Restore original position if known
        if let Some((orig_x, orig_y)) = original_pos {
            std::thread::sleep(Duration::from_millis(50));
            self.set_cursor_pos(orig_x, orig_y)?;
        }

        Ok(())
    }
}

// ==================== Helper structures and callbacks for new methods ====================

#[allow(dead_code)]
struct EnumContext<'a> {
    title_regex: Option<regex::Regex>,
    class_regex: Option<regex::Regex>,
    visible_only: bool,
    results: &'a mut Vec<HWND>,
}

#[allow(dead_code)]
unsafe extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let context = &mut *(lparam.0 as *mut EnumContext);

    // Check visibility if required
    if context.visible_only && !IsWindowVisible(hwnd).as_bool() {
        return BOOL(1); // Continue enumeration
    }

    // Check title pattern
    if let Some(ref regex) = context.title_regex {
        let mut buffer = [0u16; 256];
        let len = GetWindowTextW(hwnd, &mut buffer);
        if len > 0 {
            let title = String::from_utf16_lossy(&buffer[..len as usize]);
            if !regex.is_match(&title) {
                return BOOL(1); // Continue enumeration
            }
        } else {
            return BOOL(1); // No title, skip
        }
    }

    // Check class pattern
    if let Some(ref regex) = context.class_regex {
        let mut buffer = [0u16; 256];
        let len = GetClassNameW(hwnd, &mut buffer);
        if len > 0 {
            let class_name = String::from_utf16_lossy(&buffer[..len as usize]);
            if !regex.is_match(&class_name) {
                return BOOL(1); // Continue enumeration
            }
        } else {
            return BOOL(1); // No class name, skip
        }
    }

    // Window matches all criteria
    context.results.push(hwnd);
    BOOL(1) // Continue enumeration
}

#[allow(dead_code)]
unsafe extern "system" fn enum_child_windows_callback(
    hwnd: HWND,
    lparam: LPARAM,
) -> BOOL {
    let children = &mut *(lparam.0 as *mut Vec<HWND>);
    children.push(hwnd);
    BOOL(1) // Continue enumeration
}

/// Mock implementation for testing
#[cfg(test)]
pub struct MockWindowApi {
    pub foreground_window: RefCell<Option<HWND>>,
    pub window_rects: RefCell<HashMap<isize, WindowFrame>>,
    pub monitor_info: RefCell<HashMap<isize, MonitorInfo>>,
    pub window_states: RefCell<HashMap<isize, WindowStateDetail>>,
    pub operations_log: RefCell<Vec<WindowOperation>>,
}

#[cfg(test)]
#[allow(dead_code)]
impl MockWindowApi {
    pub fn new() -> Self {
        Self {
            foreground_window: RefCell::new(None),
            window_rects: RefCell::new(HashMap::new()),
            monitor_info: RefCell::new(HashMap::new()),
            window_states: RefCell::new(HashMap::new()),
            operations_log: RefCell::new(Vec::new()),
        }
    }

    pub fn set_foreground_window(&self, hwnd: HWND) {
        *self.foreground_window.borrow_mut() = Some(hwnd);
    }

    pub fn set_window_rect(&self, hwnd: HWND, frame: WindowFrame) {
        self.window_rects
            .borrow_mut()
            .insert(hwnd.0 as isize, frame);
    }

    pub fn set_monitor_info(&self, hwnd: HWND, info: MonitorInfo) {
        self.monitor_info.borrow_mut().insert(hwnd.0 as isize, info);
    }

    pub fn set_window_state(&self, hwnd: HWND, state: WindowStateDetail) {
        self.window_states
            .borrow_mut()
            .insert(hwnd.0 as isize, state);
    }

    pub fn get_operations(&self) -> Vec<WindowOperation> {
        self.operations_log.borrow().clone()
    }

    pub fn clear_operations(&self) {
        self.operations_log.borrow_mut().clear();
    }

    fn log_operation(&self, op: WindowOperation) {
        self.operations_log.borrow_mut().push(op);
    }
}

#[cfg(test)]
impl WindowApi for MockWindowApi {
    fn get_foreground_window(&self) -> Option<HWND> {
        self.log_operation(WindowOperation::GetForegroundWindow);
        *self.foreground_window.borrow()
    }

    fn get_window_rect(&self, hwnd: HWND) -> Option<WindowFrame> {
        self.log_operation(WindowOperation::GetWindowRect { hwnd });
        self.window_rects.borrow().get(&(hwnd.0 as isize)).copied()
    }

    fn set_window_pos(
        &self,
        hwnd: HWND,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<()> {
        self.log_operation(WindowOperation::SetWindowPos {
            hwnd,
            x,
            y,
            width,
            height,
        });

        let mut rects = self.window_rects.borrow_mut();
        rects.insert(hwnd.0 as isize, WindowFrame::new(x, y, width, height));

        // Update window state
        let mut states = self.window_states.borrow_mut();
        if let Some(state) = states.get_mut(&(hwnd.0 as isize)) {
            state.minimized = false;
            state.maximized = false;
        }

        Ok(())
    }

    fn get_monitor_info(&self, hwnd: HWND) -> Option<MonitorInfo> {
        self.log_operation(WindowOperation::GetMonitorInfo { hwnd });
        self.monitor_info.borrow().get(&(hwnd.0 as isize)).cloned()
    }

    fn get_monitor_work_area(&self, hwnd: HWND) -> Option<MonitorWorkArea> {
        self.get_monitor_info(hwnd).map(|info| MonitorWorkArea {
            x: info.x,
            y: info.y,
            width: info.width,
            height: info.height,
        })
    }

    fn is_window(&self, hwnd: HWND) -> bool {
        self.log_operation(WindowOperation::IsWindow { hwnd });
        self.window_rects.borrow().contains_key(&(hwnd.0 as isize))
    }

    fn get_window_title(&self, hwnd: HWND) -> Option<String> {
        self.log_operation(WindowOperation::GetWindowTitle { hwnd });
        Some(format!("Window {:?}", hwnd.0 as isize))
    }

    fn is_iconic(&self, hwnd: HWND) -> bool {
        self.window_states
            .borrow()
            .get(&(hwnd.0 as isize))
            .map(|s| s.minimized)
            .unwrap_or(false)
    }

    fn is_zoomed(&self, hwnd: HWND) -> bool {
        self.window_states
            .borrow()
            .get(&(hwnd.0 as isize))
            .map(|s| s.maximized)
            .unwrap_or(false)
    }

    fn minimize_window(&self, hwnd: HWND) -> Result<()> {
        self.log_operation(WindowOperation::MinimizeWindow { hwnd });
        let mut states = self.window_states.borrow_mut();
        states.entry(hwnd.0 as isize).or_default().minimized = true;
        Ok(())
    }

    fn maximize_window(&self, hwnd: HWND) -> Result<()> {
        self.log_operation(WindowOperation::MaximizeWindow { hwnd });
        let mut states = self.window_states.borrow_mut();
        states.entry(hwnd.0 as isize).or_default().maximized = true;
        Ok(())
    }

    fn restore_window(&self, hwnd: HWND) -> Result<()> {
        self.log_operation(WindowOperation::RestoreWindow { hwnd });
        let mut states = self.window_states.borrow_mut();
        if let Some(state) = states.get_mut(&(hwnd.0 as isize)) {
            state.minimized = false;
            state.maximized = false;
        }
        Ok(())
    }

    fn close_window(&self, hwnd: HWND) -> Result<()> {
        self.log_operation(WindowOperation::CloseWindow { hwnd });
        self.window_rects.borrow_mut().remove(&(hwnd.0 as isize));
        self.window_states.borrow_mut().remove(&(hwnd.0 as isize));
        Ok(())
    }

    fn set_topmost(&self, hwnd: HWND, topmost: bool) -> Result<()> {
        self.log_operation(WindowOperation::SetTopmost { hwnd, topmost });
        let mut states = self.window_states.borrow_mut();
        states.entry(hwnd.0 as isize).or_default().topmost = topmost;
        Ok(())
    }

    fn ensure_window_restored(&self, hwnd: HWND) -> Result<()> {
        self.log_operation(WindowOperation::EnsureRestored { hwnd });
        if self.is_iconic(hwnd) || self.is_zoomed(hwnd) {
            self.restore_window(hwnd)?;
        }
        Ok(())
    }

    fn is_topmost(&self, hwnd: HWND) -> bool {
        self.window_states
            .borrow()
            .get(&(hwnd.0 as isize))
            .map(|s| s.topmost)
            .unwrap_or(false)
    }

    fn wait_for_window(
        &self,
        _title_pattern: Option<&str>,
        _class_pattern: Option<&str>,
        _timeout: Duration,
        _poll_interval: Duration,
    ) -> Option<HWND> {
        // Mock implementation: return the first available window if any
        self.foreground_window.borrow().or_else(|| {
            self.window_rects
                .borrow()
                .keys()
                .next()
                .map(|&key| HWND(key as *mut core::ffi::c_void))
        })
    }

    fn find_windows(
        &self,
        _title_pattern: Option<&str>,
        _class_pattern: Option<&str>,
        _visible_only: bool,
    ) -> Vec<HWND> {
        // Mock implementation: return all known windows
        self.window_rects
            .borrow()
            .keys()
            .map(|&key| HWND(key as *mut core::ffi::c_void))
            .collect()
    }

    fn get_child_windows(&self, _parent: HWND) -> Vec<HWND> {
        // Mock implementation: return empty vector
        Vec::new()
    }

    fn get_window_class_name(&self, _hwnd: HWND) -> Option<String> {
        // Mock implementation: return a default class name
        Some("MockWindowClass".to_string())
    }

    fn is_window_visible(&self, hwnd: HWND) -> bool {
        // Mock implementation: visible if it exists in our collection
        self.window_rects.borrow().contains_key(&(hwnd.0 as isize))
    }

    fn get_window_text(&self, hwnd: HWND) -> Option<String> {
        // Delegate to get_window_title
        self.get_window_title(hwnd)
    }

    fn wait_for_foreground_window(&self, hwnd: HWND, _timeout: Duration) -> bool {
        // Mock implementation: check if it's the foreground window
        self.foreground_window
            .borrow()
            .map(|fg| fg.0 == hwnd.0)
            .unwrap_or(false)
    }

    fn get_cursor_pos(&self) -> Option<(i32, i32)> {
        // Mock implementation: return a default position
        Some((100, 100))
    }

    fn set_cursor_pos(&self, _x: i32, _y: i32) -> Result<()> {
        // Mock implementation: always succeed
        Ok(())
    }

    fn send_lbutton_down(&self) -> Result<()> {
        // Mock implementation: always succeed
        Ok(())
    }

    fn send_lbutton_up(&self) -> Result<()> {
        // Mock implementation: always succeed
        Ok(())
    }

    fn send_rbutton_down(&self) -> Result<()> {
        // Mock implementation: always succeed
        Ok(())
    }

    fn send_rbutton_up(&self) -> Result<()> {
        // Mock implementation: always succeed
        Ok(())
    }

    fn click_at(&self, _x: i32, _y: i32) -> Result<()> {
        // Mock implementation: always succeed
        Ok(())
    }
}

#[cfg(test)]
impl Default for MockWindowApi {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_hwnd(value: usize) -> HWND {
        HWND(value as *mut core::ffi::c_void)
    }

    #[test]
    fn test_mock_window_api_basic() {
        let api = MockWindowApi::new();
        let hwnd = test_hwnd(1234);

        // Set window rect
        let frame = WindowFrame::new(100, 200, 800, 600);
        api.set_window_rect(hwnd, frame);

        // Verify can be retrieved
        let retrieved = api.get_window_rect(hwnd).unwrap();
        assert_eq!(retrieved.x, 100);
        assert_eq!(retrieved.y, 200);
        assert_eq!(retrieved.width, 800);
        assert_eq!(retrieved.height, 600);

        // Verify operation log
        let ops = api.get_operations();
        assert_eq!(ops.len(), 1);
        assert!(matches!(ops[0], WindowOperation::GetWindowRect { .. }));
    }

    #[test]
    fn test_mock_window_api_set_window_pos() {
        let api = MockWindowApi::new();
        let hwnd = test_hwnd(5678);

        api.set_window_pos(hwnd, 50, 100, 1024, 768).unwrap();

        let frame = api.get_window_rect(hwnd).unwrap();
        assert_eq!(frame.x, 50);
        assert_eq!(frame.y, 100);
        assert_eq!(frame.width, 1024);
        assert_eq!(frame.height, 768);
    }

    #[test]
    fn test_mock_window_api_window_state() {
        let api = MockWindowApi::new();
        let hwnd = test_hwnd(9999);

        // Initial state
        assert!(!api.is_iconic(hwnd));
        assert!(!api.is_zoomed(hwnd));

        // Minimize
        api.minimize_window(hwnd).unwrap();
        assert!(api.is_iconic(hwnd));
        assert!(!api.is_zoomed(hwnd));

        // Restore
        api.restore_window(hwnd).unwrap();
        assert!(!api.is_iconic(hwnd));
        assert!(!api.is_zoomed(hwnd));

        // Maximize
        api.maximize_window(hwnd).unwrap();
        assert!(!api.is_iconic(hwnd));
        assert!(api.is_zoomed(hwnd));
    }

    #[test]
    fn test_mock_window_api_foreground_window() {
        let api = MockWindowApi::new();
        let hwnd = test_hwnd(1111);

        // Initially empty
        assert!(api.get_foreground_window().is_none());

        // Set foreground window
        api.set_foreground_window(hwnd);
        assert_eq!(api.get_foreground_window().unwrap().0 as usize, 1111);
    }

    // ==================== Tests for new methods ====================

    #[test]
    fn test_mock_find_windows() {
        let api = MockWindowApi::new();
        let hwnd1 = test_hwnd(1001);
        let hwnd2 = test_hwnd(1002);

        // Set up windows
        api.set_window_rect(hwnd1, WindowFrame::new(0, 0, 100, 100));
        api.set_window_rect(hwnd2, WindowFrame::new(0, 0, 200, 200));

        // Find all windows
        let windows = api.find_windows(None, None, false);
        assert_eq!(windows.len(), 2);
        assert!(windows.contains(&hwnd1));
        assert!(windows.contains(&hwnd2));
    }

    #[test]
    fn test_mock_get_child_windows() {
        let api = MockWindowApi::new();
        let parent = test_hwnd(2000);

        // Mock returns empty vector
        let children = api.get_child_windows(parent);
        assert!(children.is_empty());
    }

    #[test]
    fn test_mock_get_window_class_name() {
        let api = MockWindowApi::new();
        let hwnd = test_hwnd(3000);

        // Mock returns default class name
        let class_name = api.get_window_class_name(hwnd);
        assert_eq!(class_name, Some("MockWindowClass".to_string()));
    }

    #[test]
    fn test_mock_is_window_visible() {
        let api = MockWindowApi::new();
        let hwnd = test_hwnd(4000);

        // Not visible (not in collection)
        assert!(!api.is_window_visible(hwnd));

        // Add to collection
        api.set_window_rect(hwnd, WindowFrame::new(0, 0, 100, 100));

        // Now visible
        assert!(api.is_window_visible(hwnd));
    }

    #[test]
    fn test_mock_get_window_text() {
        let api = MockWindowApi::new();
        let hwnd = test_hwnd(5000);

        // Mock returns formatted title
        let text = api.get_window_text(hwnd);
        assert_eq!(text, Some("Window 5000".to_string()));
    }

    #[test]
    fn test_mock_wait_for_foreground_window() {
        let api = MockWindowApi::new();
        let hwnd = test_hwnd(6000);

        // Set as foreground
        api.set_foreground_window(hwnd);

        // Should return true immediately
        assert!(api.wait_for_foreground_window(hwnd, Duration::from_secs(1)));

        // Different window should return false
        let other = test_hwnd(6001);
        assert!(!api.wait_for_foreground_window(other, Duration::from_millis(100)));
    }

    #[test]
    fn test_mock_mouse_operations() {
        let api = MockWindowApi::new();

        // All mouse operations should succeed in mock
        assert!(api.get_cursor_pos().is_some());
        assert!(api.set_cursor_pos(100, 200).is_ok());
        assert!(api.send_lbutton_down().is_ok());
        assert!(api.send_lbutton_up().is_ok());
        assert!(api.send_rbutton_down().is_ok());
        assert!(api.send_rbutton_up().is_ok());
        assert!(api.click_at(50, 50).is_ok());
    }

    #[test]
    fn test_mock_wait_for_window() {
        let api = MockWindowApi::new();
        let hwnd = test_hwnd(7000);

        // Set up a window
        api.set_window_rect(hwnd, WindowFrame::new(0, 0, 100, 100));

        // wait_for_window should return the first available window
        let result = api.wait_for_window(
            None,
            None,
            Duration::from_secs(1),
            Duration::from_millis(10),
        );
        assert!(result.is_some());
    }
}
