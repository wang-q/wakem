//! Windows window API implementation
#![cfg(target_os = "windows")]

use anyhow::Result;
#[cfg(test)]
use std::cell::RefCell;
#[cfg(test)]
use std::collections::HashMap;
use windows::Win32::Foundation::{HWND, LPARAM, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{MonitorFromWindow, MONITOR_DEFAULTTONEAREST};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowRect, IsIconic, IsWindow, IsZoomed, SetWindowPos,
    ShowWindow, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOOWNERZORDER, SW_RESTORE,
};

use crate::platform::traits::{
    MonitorInfo, MonitorWorkArea, WindowApiBase, WindowFrame,
};

/// API call log entry (for MockWindowApi testing)
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum WindowApiCall {
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

impl WindowApiBase for RealWindowApi {
    type WindowId = HWND;

    fn get_foreground_window(&self) -> Option<Self::WindowId> {
        WindowApi::get_foreground_window(self)
    }

    fn set_window_pos(
        &self,
        window: Self::WindowId,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<()> {
        WindowApi::set_window_pos(self, window, x, y, width, height)
    }

    fn minimize_window(&self, window: Self::WindowId) -> Result<()> {
        WindowApi::minimize_window(self, window)
    }

    fn maximize_window(&self, window: Self::WindowId) -> Result<()> {
        WindowApi::maximize_window(self, window)
    }

    fn restore_window(&self, window: Self::WindowId) -> Result<()> {
        WindowApi::restore_window(self, window)
    }

    fn close_window(&self, window: Self::WindowId) -> Result<()> {
        WindowApi::close_window(self, window)
    }

    fn set_topmost(&self, window: Self::WindowId, topmost: bool) -> Result<()> {
        WindowApi::set_topmost(self, window, topmost)
    }

    fn get_monitors(&self) -> Vec<MonitorInfo> {
        let fg = WindowApi::get_foreground_window(self);
        fg.and_then(|hwnd| WindowApi::get_monitor_info(self, hwnd))
            .map(|info| vec![info])
            .unwrap_or_default()
    }

    fn is_window_valid(&self, window: Self::WindowId) -> bool {
        WindowApi::is_window(self, window)
    }

    fn is_minimized(&self, window: Self::WindowId) -> bool {
        WindowApi::is_iconic(self, window)
    }

    fn is_maximized(&self, window: Self::WindowId) -> bool {
        WindowApi::is_zoomed(self, window)
    }
}

impl WindowApi for RealWindowApi {
    fn get_foreground_window(&self) -> Option<HWND> {
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.0.is_null() {
                None
            } else {
                Some(hwnd)
            }
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

            if !GetMonitorInfoW(hmonitor, &mut monitor_info).as_bool() {
                return None;
            }

            let rect = &monitor_info.rcMonitor;
            Some(MonitorInfo {
                x: rect.left,
                y: rect.top,
                width: rect.right - rect.left,
                height: rect.bottom - rect.top,
            })
        }
    }

    fn get_monitor_work_area(&self, hwnd: HWND) -> Option<MonitorWorkArea> {
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

            if !GetMonitorInfoW(hmonitor, &mut monitor_info).as_bool() {
                return None;
            }

            let work_area = &monitor_info.rcWork;
            Some(MonitorWorkArea {
                x: work_area.left,
                y: work_area.top,
                width: work_area.right - work_area.left,
                height: work_area.bottom - work_area.top,
            })
        }
    }

    fn is_window(&self, hwnd: HWND) -> bool {
        unsafe { IsWindow(Some(hwnd)).as_bool() }
    }

    fn get_window_title(&self, hwnd: HWND) -> Option<String> {
        unsafe {
            let mut title_buffer = [0u16; 256];
            let len = windows::Win32::UI::WindowsAndMessaging::GetWindowTextW(
                hwnd,
                &mut title_buffer,
            );
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
            PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0))
                .map_err(|e| anyhow::anyhow!("Failed to post WM_CLOSE: {}", e))?;
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
            WindowApi::restore_window(self, hwnd)?;
        }
        Ok(())
    }
}

/// Mock implementation for testing
#[cfg(test)]
pub struct MockWindowApi {
    pub foreground_window: RefCell<Option<HWND>>,
    pub window_rects: RefCell<HashMap<isize, WindowFrame>>,
    pub monitor_info: RefCell<HashMap<isize, MonitorInfo>>,
    pub window_states: RefCell<HashMap<isize, WindowStateDetail>>,
    pub operations_log: RefCell<Vec<WindowApiCall>>,
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

    pub fn get_operations(&self) -> Vec<WindowApiCall> {
        self.operations_log.borrow().clone()
    }

    pub fn clear_operations(&self) {
        self.operations_log.borrow_mut().clear();
    }

    fn log_operation(&self, op: WindowApiCall) {
        self.operations_log.borrow_mut().push(op);
    }
}

#[cfg(test)]
impl WindowApi for MockWindowApi {
    fn get_foreground_window(&self) -> Option<HWND> {
        self.log_operation(WindowApiCall::GetForegroundWindow);
        *self.foreground_window.borrow()
    }

    fn get_window_rect(&self, hwnd: HWND) -> Option<WindowFrame> {
        self.log_operation(WindowApiCall::GetWindowRect { hwnd });
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
        self.log_operation(WindowApiCall::SetWindowPos {
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
        self.log_operation(WindowApiCall::GetMonitorInfo { hwnd });
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
        self.log_operation(WindowApiCall::IsWindow { hwnd });
        self.window_rects.borrow().contains_key(&(hwnd.0 as isize))
    }

    fn get_window_title(&self, hwnd: HWND) -> Option<String> {
        self.log_operation(WindowApiCall::GetWindowTitle { hwnd });
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
        self.log_operation(WindowApiCall::MinimizeWindow { hwnd });
        let mut states = self.window_states.borrow_mut();
        states.entry(hwnd.0 as isize).or_default().minimized = true;
        Ok(())
    }

    fn maximize_window(&self, hwnd: HWND) -> Result<()> {
        self.log_operation(WindowApiCall::MaximizeWindow { hwnd });
        let mut states = self.window_states.borrow_mut();
        states.entry(hwnd.0 as isize).or_default().maximized = true;
        Ok(())
    }

    fn restore_window(&self, hwnd: HWND) -> Result<()> {
        self.log_operation(WindowApiCall::RestoreWindow { hwnd });
        let mut states = self.window_states.borrow_mut();
        if let Some(state) = states.get_mut(&(hwnd.0 as isize)) {
            state.minimized = false;
            state.maximized = false;
        }
        Ok(())
    }

    fn close_window(&self, hwnd: HWND) -> Result<()> {
        self.log_operation(WindowApiCall::CloseWindow { hwnd });
        self.window_rects.borrow_mut().remove(&(hwnd.0 as isize));
        self.window_states.borrow_mut().remove(&(hwnd.0 as isize));
        Ok(())
    }

    fn set_topmost(&self, hwnd: HWND, topmost: bool) -> Result<()> {
        self.log_operation(WindowApiCall::SetTopmost { hwnd, topmost });
        let mut states = self.window_states.borrow_mut();
        states.entry(hwnd.0 as isize).or_default().topmost = topmost;
        Ok(())
    }

    fn ensure_window_restored(&self, hwnd: HWND) -> Result<()> {
        self.log_operation(WindowApiCall::EnsureRestored { hwnd });
        if self.is_iconic(hwnd) || self.is_zoomed(hwnd) {
            WindowApi::restore_window(self, hwnd)?;
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
}

#[cfg(test)]
impl Default for MockWindowApi {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl WindowApiBase for MockWindowApi {
    type WindowId = HWND;

    fn get_foreground_window(&self) -> Option<Self::WindowId> {
        WindowApi::get_foreground_window(self)
    }

    fn set_window_pos(
        &self,
        window: Self::WindowId,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<()> {
        WindowApi::set_window_pos(self, window, x, y, width, height)
    }

    fn minimize_window(&self, window: Self::WindowId) -> Result<()> {
        WindowApi::minimize_window(self, window)
    }

    fn maximize_window(&self, window: Self::WindowId) -> Result<()> {
        WindowApi::maximize_window(self, window)
    }

    fn restore_window(&self, window: Self::WindowId) -> Result<()> {
        WindowApi::restore_window(self, window)
    }

    fn close_window(&self, window: Self::WindowId) -> Result<()> {
        WindowApi::close_window(self, window)
    }

    fn set_topmost(&self, window: Self::WindowId, topmost: bool) -> Result<()> {
        WindowApi::set_topmost(self, window, topmost)
    }

    fn get_monitors(&self) -> Vec<MonitorInfo> {
        let fg = WindowApi::get_foreground_window(self);
        fg.and_then(|hwnd| WindowApi::get_monitor_info(self, hwnd))
            .map(|info| vec![info])
            .unwrap_or_default()
    }

    fn is_window_valid(&self, window: Self::WindowId) -> bool {
        WindowApi::is_window(self, window)
    }

    fn is_minimized(&self, window: Self::WindowId) -> bool {
        WindowApi::is_iconic(self, window)
    }

    fn is_maximized(&self, window: Self::WindowId) -> bool {
        WindowApi::is_zoomed(self, window)
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
        assert!(matches!(ops[0], WindowApiCall::GetWindowRect { .. }));
    }

    #[test]
    fn test_mock_window_api_set_window_pos() {
        let api = MockWindowApi::new();
        let hwnd = test_hwnd(5678);

        WindowApi::set_window_pos(&api, hwnd, 50, 100, 1024, 768).unwrap();

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

        assert!(!api.is_iconic(hwnd));
        assert!(!api.is_zoomed(hwnd));

        WindowApi::minimize_window(&api, hwnd).unwrap();
        assert!(api.is_iconic(hwnd));
        assert!(!api.is_zoomed(hwnd));

        WindowApi::restore_window(&api, hwnd).unwrap();
        assert!(!api.is_iconic(hwnd));
        assert!(!api.is_zoomed(hwnd));

        WindowApi::maximize_window(&api, hwnd).unwrap();
        assert!(!api.is_iconic(hwnd));
        assert!(api.is_zoomed(hwnd));
    }

    #[test]
    fn test_mock_window_api_foreground_window() {
        let api = MockWindowApi::new();
        let hwnd = test_hwnd(1111);

        assert!(WindowApi::get_foreground_window(&api).is_none());

        api.set_foreground_window(hwnd);
        assert_eq!(
            WindowApi::get_foreground_window(&api).unwrap().0 as usize,
            1111
        );
    }
}
