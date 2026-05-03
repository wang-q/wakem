//! Windows window API implementation
#![cfg(target_os = "windows")]

use anyhow::Result;
use windows::Win32::Foundation::{HWND, LPARAM, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{MonitorFromWindow, MONITOR_DEFAULTTONEAREST};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowRect, IsIconic, IsWindow, IsZoomed, SetWindowPos,
    ShowWindow, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOOWNERZORDER, SW_RESTORE,
};

use crate::platform::traits::WindowApiBase;
use crate::platform::types::{MonitorInfo, WindowFrame, WindowId};

/// Windows API abstract interface (legacy trait for backward compatibility)
pub trait WindowApi {
    fn get_foreground_window(&self) -> Option<HWND>;
    fn get_window_rect(&self, hwnd: HWND) -> Result<WindowFrame>;
    fn set_window_pos(
        &self,
        hwnd: HWND,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<()>;
    fn get_monitor_info(&self, hwnd: HWND) -> Option<MonitorInfo>;
    fn is_window(&self, hwnd: HWND) -> bool;
    fn get_window_title(&self, hwnd: HWND) -> Option<String>;
    fn is_iconic(&self, hwnd: HWND) -> bool;
    fn is_zoomed(&self, hwnd: HWND) -> bool;
    fn minimize_window(&self, hwnd: HWND) -> Result<()>;
    fn maximize_window(&self, hwnd: HWND) -> Result<()>;
    fn restore_window(&self, hwnd: HWND) -> Result<()>;
    fn close_window(&self, hwnd: HWND) -> Result<()>;
    fn set_topmost(&self, hwnd: HWND, topmost: bool) -> Result<()>;
    fn is_topmost(&self, hwnd: HWND) -> bool;
    fn ensure_window_restored(&self, hwnd: HWND) -> Result<()>;
}

/// Real Windows API implementation
pub struct RealWindowApi;

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

// Helper functions for HWND <-> WindowId conversion
fn hwnd_to_window_id(hwnd: HWND) -> WindowId {
    hwnd.0 as usize
}

fn window_id_to_hwnd(id: WindowId) -> HWND {
    HWND(id as *mut core::ffi::c_void)
}

/// Enumerate all monitors using EnumDisplayMonitors
pub(crate) unsafe fn enumerate_all_monitors() -> Vec<MonitorInfo> {
    use windows::Win32::Graphics::Gdi::{
        EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO,
    };
    use windows_core::BOOL;

    struct EnumData {
        monitors: Vec<MonitorInfo>,
    }

    unsafe extern "system" fn enum_callback(
        hmonitor: HMONITOR,
        _hdc: HDC,
        _rect: *mut RECT,
        lparam: LPARAM,
    ) -> BOOL {
        let data = &mut *(lparam.0 as *mut EnumData);

        let mut monitor_info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };

        if GetMonitorInfoW(hmonitor, &mut monitor_info).as_bool() {
            let work_area = &monitor_info.rcWork;
            data.monitors.push(MonitorInfo {
                x: work_area.left,
                y: work_area.top,
                width: work_area.right - work_area.left,
                height: work_area.bottom - work_area.top,
            });
        }

        BOOL(1)
    }

    let mut data = EnumData {
        monitors: Vec::new(),
    };

    let _ = EnumDisplayMonitors(
        None,
        None,
        Some(enum_callback),
        LPARAM(&mut data as *mut _ as isize),
    );

    data.monitors
}

// ============================================================================
// WindowApiBase Implementation
// ============================================================================

impl WindowApiBase for RealWindowApi {
    type WindowId = WindowId;

    fn get_foreground_window_inner(&self) -> Option<Self::WindowId> {
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.0.is_null() {
                None
            } else {
                Some(hwnd_to_window_id(hwnd))
            }
        }
    }

    fn set_window_pos_inner(
        &self,
        window: Self::WindowId,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<()> {
        unsafe {
            let hwnd = window_id_to_hwnd(window);
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

    fn minimize_window_inner(&self, window: Self::WindowId) -> Result<()> {
        unsafe {
            let hwnd = window_id_to_hwnd(window);
            ShowWindow(hwnd, windows::Win32::UI::WindowsAndMessaging::SW_MINIMIZE)
                .ok()
                .map_err(|e| anyhow::anyhow!("Failed to minimize window: {}", e))?;
            Ok(())
        }
    }

    fn maximize_window_inner(&self, window: Self::WindowId) -> Result<()> {
        unsafe {
            let hwnd = window_id_to_hwnd(window);
            ShowWindow(hwnd, windows::Win32::UI::WindowsAndMessaging::SW_MAXIMIZE)
                .ok()
                .map_err(|e| anyhow::anyhow!("Failed to maximize window: {}", e))?;
            Ok(())
        }
    }

    fn restore_window_inner(&self, window: Self::WindowId) -> Result<()> {
        unsafe {
            let hwnd = window_id_to_hwnd(window);
            ShowWindow(hwnd, SW_RESTORE)
                .ok()
                .map_err(|e| anyhow::anyhow!("Failed to restore window: {}", e))?;
            Ok(())
        }
    }

    fn close_window_inner(&self, window: Self::WindowId) -> Result<()> {
        unsafe {
            let hwnd = window_id_to_hwnd(window);
            use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_CLOSE};
            PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0))
                .map_err(|e| anyhow::anyhow!("Failed to post WM_CLOSE: {}", e))?;
            Ok(())
        }
    }

    fn set_topmost_inner(&self, window: Self::WindowId, topmost: bool) -> Result<()> {
        unsafe {
            let hwnd = window_id_to_hwnd(window);
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

    fn is_topmost_inner(&self, window: Self::WindowId) -> bool {
        unsafe {
            let hwnd = window_id_to_hwnd(window);
            use windows::Win32::UI::WindowsAndMessaging::WS_EX_TOPMOST;
            use windows::Win32::UI::WindowsAndMessaging::{
                GetWindowLongW, IsWindow, GWL_EXSTYLE,
            };

            if !IsWindow(Some(hwnd)).as_bool() {
                return false;
            }

            let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
            (ex_style as u32) & WS_EX_TOPMOST.0 != 0
        }
    }

    fn is_window_valid_inner(&self, window: Self::WindowId) -> bool {
        unsafe { IsWindow(Some(window_id_to_hwnd(window))).as_bool() }
    }

    fn is_minimized_inner(&self, window: Self::WindowId) -> bool {
        unsafe { IsIconic(window_id_to_hwnd(window)).as_bool() }
    }

    fn is_maximized_inner(&self, window: Self::WindowId) -> bool {
        unsafe { IsZoomed(window_id_to_hwnd(window)).as_bool() }
    }

    fn get_window_title_inner(&self, window: Self::WindowId) -> Option<String> {
        unsafe {
            let hwnd = window_id_to_hwnd(window);
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

    fn get_window_rect_inner(&self, window: Self::WindowId) -> Result<WindowFrame> {
        unsafe {
            let hwnd = window_id_to_hwnd(window);
            let mut rect = RECT::default();
            GetWindowRect(hwnd, &mut rect)?;
            Ok(WindowFrame::new(
                rect.left,
                rect.top,
                rect.right - rect.left,
                rect.bottom - rect.top,
            ))
        }
    }

    fn get_monitors_inner(&self) -> Vec<MonitorInfo> {
        unsafe { enumerate_all_monitors() }
    }
}

// ============================================================================
// Legacy WindowApi Implementation (for backward compatibility)
// ============================================================================

impl WindowApi for RealWindowApi {
    fn get_foreground_window(&self) -> Option<HWND> {
        self.get_foreground_window_inner().map(window_id_to_hwnd)
    }

    fn get_window_rect(&self, hwnd: HWND) -> Result<WindowFrame> {
        self.get_window_rect_inner(hwnd_to_window_id(hwnd))
    }

    fn set_window_pos(
        &self,
        hwnd: HWND,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<()> {
        self.set_window_pos_inner(hwnd_to_window_id(hwnd), x, y, width, height)
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

            let work_area = &monitor_info.rcWork;
            Some(MonitorInfo {
                x: work_area.left,
                y: work_area.top,
                width: work_area.right - work_area.left,
                height: work_area.bottom - work_area.top,
            })
        }
    }

    fn is_window(&self, hwnd: HWND) -> bool {
        self.is_window_valid_inner(hwnd_to_window_id(hwnd))
    }

    fn get_window_title(&self, hwnd: HWND) -> Option<String> {
        self.get_window_title_inner(hwnd_to_window_id(hwnd))
    }

    fn is_iconic(&self, hwnd: HWND) -> bool {
        self.is_minimized_inner(hwnd_to_window_id(hwnd))
    }

    fn is_zoomed(&self, hwnd: HWND) -> bool {
        self.is_maximized_inner(hwnd_to_window_id(hwnd))
    }

    fn minimize_window(&self, hwnd: HWND) -> Result<()> {
        self.minimize_window_inner(hwnd_to_window_id(hwnd))
    }

    fn maximize_window(&self, hwnd: HWND) -> Result<()> {
        self.maximize_window_inner(hwnd_to_window_id(hwnd))
    }

    fn restore_window(&self, hwnd: HWND) -> Result<()> {
        self.restore_window_inner(hwnd_to_window_id(hwnd))
    }

    fn close_window(&self, hwnd: HWND) -> Result<()> {
        self.close_window_inner(hwnd_to_window_id(hwnd))
    }

    fn set_topmost(&self, hwnd: HWND, topmost: bool) -> Result<()> {
        self.set_topmost_inner(hwnd_to_window_id(hwnd), topmost)
    }

    fn is_topmost(&self, hwnd: HWND) -> bool {
        self.is_topmost_inner(hwnd_to_window_id(hwnd))
    }

    fn ensure_window_restored(&self, hwnd: HWND) -> Result<()> {
        WindowApiBase::ensure_window_restored(self, hwnd_to_window_id(hwnd))
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_real_window_api_conversions() {
        let api = RealWindowApi::new();

        // Test that conversions work correctly
        let hwnd = HWND(12345 as *mut core::ffi::c_void);
        let id = hwnd_to_window_id(hwnd);
        let hwnd2 = window_id_to_hwnd(id);
        assert_eq!(hwnd.0, hwnd2.0);
    }

    #[test]
    fn test_window_api_base_default_methods() {
        let api = RealWindowApi::new();

        // Test that default delegation methods work (they just call inner methods)
        // We can't test actual window operations in unit tests, but we can verify
        // the API structure is correct
        let _: &dyn WindowApiBase<WindowId = WindowId> = &api;
    }
}
