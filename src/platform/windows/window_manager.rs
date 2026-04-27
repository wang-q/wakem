//! Windows window manager implementation
#![cfg(target_os = "windows")]

use anyhow::Result;
use tracing::debug;
use windows::Win32::Foundation::{HWND, LPARAM, RECT};
use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, GetForegroundWindow, GetWindowRect, GetWindowThreadProcessId,
    IsIconic, SetForegroundWindow, ShowWindow, GW_OWNER, SW_RESTORE,
};
use windows_core::BOOL;

// Import Edge and Alignment from types
use super::window_api::RealWindowApi;
pub use crate::types::{Alignment, Edge};
// Import common window manager and shared types
use crate::platform::traits::{
    MonitorDirection, MonitorInfo, WindowApiBase, WindowFrame, WindowInfo,
};
pub use crate::platform::window_manager_common::WindowManager;

// Re-export window preset manager for Windows
use crate::platform::window_preset_common::WindowPresetManager as CommonWindowPresetManager;

/// Window preset manager type for Windows platform
pub type WindowPresetManager = CommonWindowPresetManager<
    WindowManager<crate::platform::windows::window_api::RealWindowApi>,
>;

impl Default for WindowPresetManager {
    fn default() -> Self {
        Self::new(WindowManager::new())
    }
}

/// Type alias for window manager using real Windows API
pub type RealWindowManager = WindowManager<RealWindowApi>;

/// Enumerate all monitors using EnumDisplayMonitors.
///
/// Returns monitor rectangles (full area including taskbar) as `MonitorInfo`.
pub(crate) unsafe fn enumerate_all_monitors() -> Vec<MonitorInfo> {
    use windows::Win32::Graphics::Gdi::{
        EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO,
    };

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
            let monitor_rect = &monitor_info.rcMonitor;
            data.monitors.push(MonitorInfo {
                x: monitor_rect.left,
                y: monitor_rect.top,
                width: monitor_rect.right - monitor_rect.left,
                height: monitor_rect.bottom - monitor_rect.top,
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

impl WindowManager<RealWindowApi> {
    /// Create a window manager using real Windows API
    pub fn new() -> Self {
        Self::with_api(RealWindowApi::new())
    }
}

impl Default for RealWindowManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Features requiring real Windows API
impl RealWindowManager {
    /// Get all visible windows belonging to the same application (by process name)
    ///
    /// Filters out:
    /// - Invisible windows
    /// - Owned/child popup windows (GW_OWNER check)
    /// - Windows with empty titles
    /// - System shell windows ("Program Manager" / Progman class)
    pub fn get_app_visible_windows(&self, target_process_name: &str) -> Vec<HWND> {
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::System::ProcessStatus::GetModuleBaseNameW;
        use windows::Win32::System::Threading::{
            OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ,
        };
        use windows::Win32::UI::WindowsAndMessaging::{
            EnumWindows, GetClassNameW, GetWindow, GetWindowTextW, IsWindowVisible,
        };

        struct EnumData<'a> {
            target_process_name: &'a str,
            windows: Vec<HWND>,
        }

        unsafe extern "system" fn enum_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
            let data = &mut *(lparam.0 as *mut EnumData);

            if !IsWindowVisible(hwnd).as_bool() {
                return BOOL(1);
            }

            let owner = GetWindow(hwnd, GW_OWNER).unwrap_or_default();
            if !owner.0.is_null() {
                return BOOL(1);
            }

            let mut title = [0u16; 256];
            let len = GetWindowTextW(hwnd, &mut title);
            if len == 0 {
                return BOOL(1);
            }
            let title_str = String::from_utf16_lossy(&title[..len as usize]);

            if title_str == "Program Manager" {
                return BOOL(1);
            }

            let mut class_name = [0u16; 256];
            let class_len = GetClassNameW(hwnd, &mut class_name);
            let class_str = String::from_utf16_lossy(&class_name[..class_len as usize]);

            if class_str == "Progman"
                || class_str == "WorkerW"
                || class_str == "Shell_TrayWnd"
            {
                return BOOL(1);
            }

            let mut pid: u32 = 0;
            windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(
                hwnd,
                Some(&mut pid),
            );
            if pid == 0 {
                return BOOL(1);
            }

            let handle = match OpenProcess(
                PROCESS_QUERY_INFORMATION | PROCESS_VM_READ,
                false,
                pid,
            ) {
                Ok(h) => h,
                Err(_) => return BOOL(1),
            };

            let mut name_buf = [0u16; 260];
            let name_len = GetModuleBaseNameW(handle, None, &mut name_buf);
            CloseHandle(handle).ok();

            if name_len == 0 {
                return BOOL(1);
            }

            let proc_name = String::from_utf16_lossy(&name_buf[..name_len as usize]);
            if !proc_name.eq_ignore_ascii_case(data.target_process_name) {
                return BOOL(1);
            }

            data.windows.push(hwnd);
            BOOL(1)
        }

        unsafe {
            let mut data = EnumData {
                target_process_name,
                windows: Vec::new(),
            };

            let _ =
                EnumWindows(Some(enum_callback), LPARAM(&mut data as *mut _ as isize));

            data.windows
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::MockWindowApi;
    use super::*;

    fn test_hwnd(value: usize) -> HWND {
        HWND(value as *mut core::ffi::c_void)
    }

    #[test]
    fn test_window_manager_creation() {
        let api = MockWindowApi::new();
        let wm = WindowManager::with_api(api);

        // Verify creation success
        assert!(wm.api().is_window(test_hwnd(0)) == false);
    }

    #[test]
    fn test_get_window_info() {
        let api = MockWindowApi::new();
        let hwnd = test_hwnd(1234);

        // Set test data
        api.set_window_rect(hwnd, WindowFrame::new(100, 200, 800, 600));
        api.set_monitor_info(
            hwnd,
            MonitorInfo {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
        );

        let wm = WindowManager::with_api(api);
        let info = wm.get_window_info(hwnd).unwrap();

        assert_eq!(info.frame.x, 100);
        assert_eq!(info.frame.y, 200);
        assert_eq!(info.frame.width, 800);
        assert_eq!(info.frame.height, 600);
    }

    #[test]
    fn test_move_to_center() {
        let api = MockWindowApi::new();
        let hwnd = test_hwnd(1234);

        // Set test data - 800x600 window on 1920x1080 monitor
        api.set_window_rect(hwnd, WindowFrame::new(0, 0, 800, 600));
        api.set_monitor_info(
            hwnd,
            MonitorInfo {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
        );

        let wm = WindowManager::with_api(api);
        wm.move_to_center(hwnd).unwrap();

        // Verify window position (1920-800)/2 = 560, (1080-600)/2 = 240
        let frame = wm.api().get_window_rect(hwnd).unwrap();
        assert_eq!(frame.x, 560);
        assert_eq!(frame.y, 240);
    }

    #[test]
    fn test_move_to_edge() {
        let api = MockWindowApi::new();
        let hwnd = test_hwnd(1234);

        api.set_window_rect(hwnd, WindowFrame::new(100, 100, 800, 600));
        api.set_monitor_info(
            hwnd,
            MonitorInfo {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
        );

        let wm = WindowManager::with_api(api);

        // Test left edge
        wm.move_to_edge(hwnd, Edge::Left).unwrap();
        let frame = wm.api().get_window_rect(hwnd).unwrap();
        assert_eq!(frame.x, 0);

        // Test right edge
        wm.move_to_edge(hwnd, Edge::Right).unwrap();
        let frame = wm.api().get_window_rect(hwnd).unwrap();
        assert_eq!(frame.x, 1920 - 800);
    }

    #[test]
    fn test_set_half_screen() {
        let api = MockWindowApi::new();
        let hwnd = test_hwnd(1234);

        api.set_window_rect(hwnd, WindowFrame::new(100, 100, 800, 600));
        api.set_monitor_info(
            hwnd,
            MonitorInfo {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
        );

        let wm = WindowManager::with_api(api);

        // Test left half screen
        wm.set_half_screen(hwnd, Edge::Left).unwrap();
        let frame = wm.api().get_window_rect(hwnd).unwrap();
        assert_eq!(frame.x, 0);
        assert_eq!(frame.y, 0);
        assert_eq!(frame.width, 960); // 1920 / 2
        assert_eq!(frame.height, 1080);

        // Test right half screen
        wm.set_half_screen(hwnd, Edge::Right).unwrap();
        let frame = wm.api().get_window_rect(hwnd).unwrap();
        assert_eq!(frame.x, 960);
        assert_eq!(frame.width, 960);
    }

    #[test]
    fn test_loop_width() {
        let api = MockWindowApi::new();
        let hwnd = test_hwnd(1234);

        // Set all data before creating WindowManager
        api.set_monitor_info(
            hwnd,
            MonitorInfo {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
        );
        api.set_window_rect(hwnd, WindowFrame::new(0, 0, 960, 600));

        let wm = WindowManager::with_api(api);

        // Test cycle from 50%
        wm.loop_width(hwnd, Alignment::Left).unwrap();

        let frame = wm.api().get_window_rect(hwnd).unwrap();
        // 50% -> 40% = 768
        assert_eq!(frame.width, 768);
    }

    #[test]
    fn test_set_fixed_ratio() {
        let api = MockWindowApi::new();
        let hwnd = test_hwnd(1234);

        // Set all data before creating WindowManager
        api.set_monitor_info(
            hwnd,
            MonitorInfo {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
        );
        // Need to set an initial window size first
        api.set_window_rect(hwnd, WindowFrame::new(100, 100, 800, 600));

        let wm = WindowManager::with_api(api);

        // Test 4:3 ratio, 100% scale
        wm.set_fixed_ratio(hwnd, 4.0 / 3.0).unwrap();

        let frame = wm.api().get_window_rect(hwnd).unwrap();
        // Based on smaller side 1080, 4:3 ratio, width = 1080 * 4/3 = 1440
        assert_eq!(frame.width, 1440);
        assert_eq!(frame.height, 1080);
    }

    #[test]
    fn test_window_state_operations() {
        let api = MockWindowApi::new();
        let hwnd = test_hwnd(1234);

        api.set_window_rect(hwnd, WindowFrame::new(100, 100, 800, 600));

        let wm = WindowManager::with_api(api);

        // Test minimize
        wm.minimize_window(hwnd).unwrap();
        assert!(wm.api().is_iconic(hwnd));

        // Test restore
        wm.restore_window(hwnd).unwrap();
        assert!(!wm.api().is_iconic(hwnd));

        // Test maximize
        wm.maximize_window(hwnd).unwrap();
        assert!(wm.api().is_zoomed(hwnd));
    }

    #[test]
    fn test_close_window() {
        let api = MockWindowApi::new();
        let hwnd = test_hwnd(1234);

        api.set_window_rect(hwnd, WindowFrame::new(100, 100, 800, 600));

        let wm = WindowManager::with_api(api);
        assert!(wm.api().is_window(hwnd));

        wm.close_window(hwnd).unwrap();

        // Window should be removed
        assert!(!wm.api().is_window(hwnd));
    }
}
