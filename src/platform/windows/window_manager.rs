//! Windows window manager implementation
#![cfg(target_os = "windows")]

use windows::Win32::Foundation::{LPARAM, RECT};
use windows_core::BOOL;

use super::window_api::RealWindowApi;
use crate::platform::traits::MonitorInfo;
pub use crate::platform::window_manager_common::WindowManager;

crate::impl_window_manager_types!();

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

#[cfg(test)]
mod tests {
    use super::super::MockWindowApi;
    use super::*;
    use crate::platform::traits::{WindowApiBase, WindowFrame};
    use crate::types::{Alignment, Edge};
    use windows::Win32::Foundation::HWND;

    fn test_hwnd(value: usize) -> HWND {
        HWND(value as *mut core::ffi::c_void)
    }

    #[test]
    fn test_window_manager_creation() {
        let api = MockWindowApi::new();
        let wm = WindowManager::with_api(api);

        // Verify creation success
        assert!(!wm.api().is_window_valid(test_hwnd(0)));
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
        api.set_foreground_window(hwnd);

        let wm = WindowManager::with_api(api);
        let info = wm.get_window_info(hwnd).unwrap();

        assert_eq!(info.x, 100);
        assert_eq!(info.y, 200);
        assert_eq!(info.width, 800);
        assert_eq!(info.height, 600);
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
        api.set_foreground_window(hwnd);

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
        api.set_foreground_window(hwnd);

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
        api.set_foreground_window(hwnd);

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
        api.set_foreground_window(hwnd);

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
        api.set_foreground_window(hwnd);

        let wm = WindowManager::with_api(api);

        // Test 4:3 ratio, 100% scale
        wm.set_fixed_ratio(hwnd, 4.0 / 3.0, None).unwrap();

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
        assert!(wm.api().is_minimized(hwnd));

        // Test restore
        wm.restore_window(hwnd).unwrap();
        assert!(!wm.api().is_minimized(hwnd));

        // Test maximize
        wm.maximize_window(hwnd).unwrap();
        assert!(wm.api().is_maximized(hwnd));
    }

    #[test]
    fn test_close_window() {
        let api = MockWindowApi::new();
        let hwnd = test_hwnd(1234);

        api.set_window_rect(hwnd, WindowFrame::new(100, 100, 800, 600));

        let wm = WindowManager::with_api(api);
        assert!(wm.api().is_window_valid(hwnd));

        wm.close_window(hwnd).unwrap();

        // Window should be removed
        assert!(!wm.api().is_window_valid(hwnd));
    }
}
