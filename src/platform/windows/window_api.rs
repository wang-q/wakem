use anyhow::Result;
#[allow(unused_imports)]
use std::cell::RefCell;
#[allow(unused_imports)]
use std::collections::HashMap;
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Gdi::{MonitorFromWindow, MONITOR_DEFAULTTONEAREST};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowRect, GetWindowTextW, IsIconic, IsWindow, IsZoomed,
    SetWindowPos, ShowWindow, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOOWNERZORDER,
    SW_RESTORE,
};

use super::WindowFrame;

/// Monitor information
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct MonitorInfo {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// Monitor work area information
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct MonitorWorkArea {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// Window operation log
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
    SetOpacity {
        hwnd: HWND,
        opacity: u8,
    },
    EnsureRestored {
        hwnd: HWND,
    },
}

/// Window state
#[derive(Debug, Clone, Copy)]
pub struct WindowState {
    pub minimized: bool,
    pub maximized: bool,
    pub topmost: bool,
    pub opacity: u8,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            minimized: false,
            maximized: false,
            topmost: false,
            opacity: 255,
        }
    }
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
    /// Set transparency
    fn set_opacity(&self, hwnd: HWND, opacity: u8) -> Result<()>;
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

impl WindowApi for RealWindowApi {
    fn get_foreground_window(&self) -> Option<HWND> {
        let hwnd = unsafe { GetForegroundWindow() };
        if hwnd.0 == 0 {
            None
        } else {
            Some(hwnd)
        }
    }

    fn get_window_rect(&self, hwnd: HWND) -> Option<WindowFrame> {
        unsafe {
            let mut rect = RECT::default();
            GetWindowRect(hwnd, &mut rect).ok()?;
            Some(WindowFrame::from_rect(&rect))
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
        unsafe { IsWindow(hwnd).as_bool() }
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
            let _ = PostMessageW(hwnd, WM_CLOSE, None, None);
            Ok(())
        }
    }

    fn set_topmost(&self, hwnd: HWND, topmost: bool) -> Result<()> {
        unsafe {
            use windows::Win32::UI::WindowsAndMessaging::{
                SetWindowPos, HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOMOVE, SWP_NOSIZE,
            };
            let pos = if topmost {
                HWND_TOPMOST
            } else {
                HWND_NOTOPMOST
            };
            let _ = SetWindowPos(hwnd, pos, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE);
            Ok(())
        }
    }

    fn set_opacity(&self, hwnd: HWND, opacity: u8) -> Result<()> {
        unsafe {
            use windows::Win32::UI::WindowsAndMessaging::{
                GetWindowLongW, SetLayeredWindowAttributes, SetWindowLongW, GWL_EXSTYLE,
            };
            use windows::Win32::UI::WindowsAndMessaging::{LWA_ALPHA, WS_EX_LAYERED};

            let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
            let _ = SetWindowLongW(hwnd, GWL_EXSTYLE, ex_style | WS_EX_LAYERED.0 as i32);

            let _ = SetLayeredWindowAttributes(hwnd, None, opacity, LWA_ALPHA);
            Ok(())
        }
    }

    fn ensure_window_restored(&self, hwnd: HWND) -> Result<()> {
        if self.is_iconic(hwnd) || self.is_zoomed(hwnd) {
            self.restore_window(hwnd)?;
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
    pub window_states: RefCell<HashMap<isize, WindowState>>,
    pub operations_log: RefCell<Vec<WindowOperation>>,
}

#[cfg(test)]
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
        self.window_rects.borrow_mut().insert(hwnd.0, frame);
    }

    pub fn set_monitor_info(&self, hwnd: HWND, info: MonitorInfo) {
        self.monitor_info.borrow_mut().insert(hwnd.0, info);
    }

    pub fn set_window_state(&self, hwnd: HWND, state: WindowState) {
        self.window_states.borrow_mut().insert(hwnd.0, state);
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
        self.window_rects.borrow().get(&hwnd.0).copied()
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
        rects.insert(hwnd.0, WindowFrame::new(x, y, width, height));

        // Update window state
        let mut states = self.window_states.borrow_mut();
        if let Some(state) = states.get_mut(&hwnd.0) {
            state.minimized = false;
            state.maximized = false;
        }

        Ok(())
    }

    fn get_monitor_info(&self, hwnd: HWND) -> Option<MonitorInfo> {
        self.log_operation(WindowOperation::GetMonitorInfo { hwnd });
        self.monitor_info.borrow().get(&hwnd.0).cloned()
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
        self.window_rects.borrow().contains_key(&hwnd.0)
    }

    fn get_window_title(&self, hwnd: HWND) -> Option<String> {
        self.log_operation(WindowOperation::GetWindowTitle { hwnd });
        Some(format!("Window {:?}", hwnd.0))
    }

    fn is_iconic(&self, hwnd: HWND) -> bool {
        self.window_states
            .borrow()
            .get(&hwnd.0)
            .map(|s| s.minimized)
            .unwrap_or(false)
    }

    fn is_zoomed(&self, hwnd: HWND) -> bool {
        self.window_states
            .borrow()
            .get(&hwnd.0)
            .map(|s| s.maximized)
            .unwrap_or(false)
    }

    fn minimize_window(&self, hwnd: HWND) -> Result<()> {
        self.log_operation(WindowOperation::MinimizeWindow { hwnd });
        let mut states = self.window_states.borrow_mut();
        states.entry(hwnd.0).or_default().minimized = true;
        Ok(())
    }

    fn maximize_window(&self, hwnd: HWND) -> Result<()> {
        self.log_operation(WindowOperation::MaximizeWindow { hwnd });
        let mut states = self.window_states.borrow_mut();
        states.entry(hwnd.0).or_default().maximized = true;
        Ok(())
    }

    fn restore_window(&self, hwnd: HWND) -> Result<()> {
        self.log_operation(WindowOperation::RestoreWindow { hwnd });
        let mut states = self.window_states.borrow_mut();
        if let Some(state) = states.get_mut(&hwnd.0) {
            state.minimized = false;
            state.maximized = false;
        }
        Ok(())
    }

    fn close_window(&self, hwnd: HWND) -> Result<()> {
        self.log_operation(WindowOperation::CloseWindow { hwnd });
        self.window_rects.borrow_mut().remove(&hwnd.0);
        self.window_states.borrow_mut().remove(&hwnd.0);
        Ok(())
    }

    fn set_topmost(&self, hwnd: HWND, topmost: bool) -> Result<()> {
        self.log_operation(WindowOperation::SetTopmost { hwnd, topmost });
        let mut states = self.window_states.borrow_mut();
        states.entry(hwnd.0).or_default().topmost = topmost;
        Ok(())
    }

    fn set_opacity(&self, hwnd: HWND, opacity: u8) -> Result<()> {
        self.log_operation(WindowOperation::SetOpacity { hwnd, opacity });
        let mut states = self.window_states.borrow_mut();
        states.entry(hwnd.0).or_default().opacity = opacity;
        Ok(())
    }

    fn ensure_window_restored(&self, hwnd: HWND) -> Result<()> {
        self.log_operation(WindowOperation::EnsureRestored { hwnd });
        if self.is_iconic(hwnd) || self.is_zoomed(hwnd) {
            self.restore_window(hwnd)?;
        }
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

    #[test]
    fn test_mock_window_api_basic() {
        let api = MockWindowApi::new();
        let hwnd = HWND(1234);

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
        let hwnd = HWND(5678);

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
        let hwnd = HWND(9999);

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
        let hwnd = HWND(1111);

        // Initially empty
        assert!(api.get_foreground_window().is_none());

        // Set foreground window
        api.set_foreground_window(hwnd);
        assert_eq!(api.get_foreground_window().unwrap().0, 1111);
    }
}
