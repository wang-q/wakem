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
#[allow(unused_imports)]
pub use crate::types::{Alignment, Edge};
// Import common window manager and shared types
pub use crate::platform::window_manager_common::WindowManager;
use crate::platform::traits::{
    MonitorDirection, MonitorInfo, WindowApiBase, WindowFrame, WindowInfo,
};

/// Type alias for window manager using real Windows API
pub type RealWindowManager = WindowManager<RealWindowApi>;

/// Create WindowFrame from RECT
#[allow(dead_code)]
fn window_frame_from_rect(rect: &RECT) -> WindowFrame {
    WindowFrame::new(
        rect.left,
        rect.top,
        rect.right - rect.left,
        rect.bottom - rect.top,
    )
}

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

/// Platform-specific CommonWindowApi implementation for Windows
impl<A: WindowApiBase<WindowId = HWND> + 'static> crate::platform::window_manager_common::CommonWindowApi for WindowManager<A> {
    type WindowId = HWND;
    type WindowInfo = WindowInfo;

    fn api(&self) -> &dyn std::any::Any {
        self
    }

    fn get_window_info(&self, window: Self::WindowId) -> Result<Self::WindowInfo> {
        self.api().get_window_info(window)
    }

    fn set_window_pos(
        &self,
        window: Self::WindowId,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<()> {
        let frame = WindowFrame::new(x, y, width, height);
        self.set_window_frame(window, &frame)
    }

    fn get_monitors(&self) -> Vec<MonitorInfo> {
        #[cfg(not(test))]
        {
            unsafe { enumerate_all_monitors() }
        }
        #[cfg(test)]
        {
            if let Some(hwnd) = self.api().get_foreground_window() {
                if let Some(monitor) = self.api().get_monitors().first().cloned() {
                    return vec![monitor];
                }
            }
            vec![MonitorInfo {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            }]
        }
    }

    fn is_window_valid(&self, window: Self::WindowId) -> bool {
        self.api().is_window_valid(window)
    }

    fn is_maximized(&self, window: Self::WindowId) -> bool {
        self.api().is_maximized(window)
    }

    fn is_topmost(&self, window: Self::WindowId) -> bool {
        self.api().is_topmost(window)
    }

    fn set_topmost(&self, window: Self::WindowId, topmost: bool) -> Result<()> {
        self.api().set_topmost(window, topmost)
    }
}

/// Features requiring real Windows API (cross-monitor movement, window switching, etc.)
impl RealWindowManager {
    /// Move window to another monitor
    pub fn move_to_monitor(&self, hwnd: HWND, direction: MonitorDirection) -> Result<()> {
        use crate::platform::window_manager_common::CommonWindowManager;
        CommonWindowManager::move_to_monitor(self, hwnd, direction)
    }

    /// Switch to next window of same application (Alt+` function)
    ///
    /// Uses process image name (e.g., "explorer.exe") instead of PID because
    /// Windows Explorer and some other apps run each window in a separate process.
    /// Falls back to PID matching if process name cannot be obtained (e.g., access denied).
    pub fn switch_to_next_window_of_same_process(&self) -> Result<()> {
        unsafe {
            let current_hwnd = GetForegroundWindow();
            if current_hwnd.0.is_null() {
                return Err(anyhow::anyhow!("No foreground window"));
            }

            let current_pid = self.get_window_process_id(current_hwnd)?;

            // Try to get process name, fall back to PID matching if access denied
            let windows = match self.get_process_name_by_pid(current_pid) {
                Ok(process_name) => {
                    debug!(
                        "[SwitchWindow] PID={}, process={}",
                        current_pid, process_name
                    );
                    self.get_app_visible_windows(&process_name)
                }
                Err(e) => {
                    debug!(
                        "[SwitchWindow] PID={}, failed to get process name ({}), falling back to PID matching",
                        current_pid, e
                    );
                    self.get_process_visible_windows(current_pid)
                }
            };

            debug!("[SwitchWindow] Found {} windows", windows.len());

            if windows.len() < 2 {
                debug!(
                    "[SwitchWindow] Only {} window(s), need >= 2. Skipping.",
                    windows.len()
                );
                return Ok(());
            }

            let sorted_windows = self.sort_windows_by_zorder(windows);

            let current_index = sorted_windows
                .iter()
                .position(|&hwnd| hwnd == current_hwnd)
                .unwrap_or(0);

            let next_index = (current_index + 1) % sorted_windows.len();
            let next_hwnd = sorted_windows[next_index];

            debug!(
                "[SwitchWindow] Switching index {} -> {} (total {})",
                current_index,
                next_index,
                sorted_windows.len()
            );

            self.activate_window(next_hwnd)?;
            Ok(())
        }
    }

    /// Get window process ID
    unsafe fn get_window_process_id(&self, hwnd: HWND) -> Result<u32> {
        let mut pid: u32 = 0;
        windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(
            hwnd,
            Some(&mut pid),
        );

        if pid == 0 {
            return Err(anyhow::anyhow!("Failed to get process ID"));
        }

        Ok(pid)
    }

    unsafe fn get_process_name_by_pid(&self, pid: u32) -> Result<String> {
        super::get_process_name_by_pid(pid)
    }

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

    /// Get all visible windows of specified process (by PID)
    /// Used as fallback when process name cannot be obtained due to access restrictions
    fn get_process_visible_windows(&self, target_pid: u32) -> Vec<HWND> {
        use windows::Win32::UI::WindowsAndMessaging::{
            EnumWindows, GetWindow, GetWindowTextW, IsWindowVisible,
        };

        struct EnumData {
            target_pid: u32,
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

            let mut pid: u32 = 0;
            windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(
                hwnd,
                Some(&mut pid),
            );

            if pid == data.target_pid {
                data.windows.push(hwnd);
            }

            BOOL(1)
        }

        unsafe {
            let mut data = EnumData {
                target_pid,
                windows: Vec::new(),
            };

            let _ =
                EnumWindows(Some(enum_callback), LPARAM(&mut data as *mut _ as isize));

            data.windows
        }
    }

    /// Sort windows by Z-Order (from front to back)
    fn sort_windows_by_zorder(&self, windows: Vec<HWND>) -> Vec<HWND> {
        use windows::Win32::UI::WindowsAndMessaging::{EnumWindows, IsWindowVisible};

        unsafe {
            // Get Z-Order positions of all windows
            // Method: Enumerate all windows in Z-Order (top to bottom), recording position of each window
            let mut zorder_map: std::collections::HashMap<isize, usize> =
                std::collections::HashMap::new();

            // Use EnumWindows to get windows in Z-Order (topmost first)
            struct EnumData<'a> {
                target_windows: &'a [HWND],
                zorder_map: &'a mut std::collections::HashMap<isize, usize>,
                z_index: usize,
            }

            unsafe extern "system" fn enum_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
                let data = &mut *(lparam.0 as *mut EnumData);

                // Only consider visible windows
                if !IsWindowVisible(hwnd).as_bool() {
                    return BOOL(1);
                }

                // If this window is in our target list, record its Z-Order
                if data.target_windows.contains(&hwnd) {
                    data.zorder_map.insert(hwnd.0 as isize, data.z_index);
                }

                data.z_index += 1;
                BOOL(1)
            }

            let mut data = EnumData {
                target_windows: &windows,
                zorder_map: &mut zorder_map,
                z_index: 0,
            };

            let _ =
                EnumWindows(Some(enum_callback), LPARAM(&mut data as *mut _ as isize));

            // Sort by Z-Order (lower index = higher in Z-Order = more recently used)
            let mut sorted = windows;
            sorted.sort_by_key(|hwnd| {
                zorder_map
                    .get(&(hwnd.0 as isize))
                    .copied()
                    .unwrap_or(usize::MAX)
            });

            sorted
        }
    }

    /// Activate window (switch to foreground)
    ///
    /// Uses AttachThreadInput workaround to bypass Windows' restriction that
    /// only the foreground process can call SetForegroundWindow successfully.
    /// Without this, a background daemon process like wakem would have its
    /// SetForegroundWindow calls silently ignored or rejected by the OS.
    unsafe fn activate_window(&self, hwnd: HWND) -> Result<()> {
        let foreground_hwnd = GetForegroundWindow();
        let foreground_thread = GetWindowThreadProcessId(foreground_hwnd, None);
        let current_thread = GetCurrentThreadId();

        let attached = foreground_thread != current_thread && foreground_thread != 0;
        if attached {
            AttachThreadInput(foreground_thread, current_thread, true)
                .ok()
                .map_err(|e| anyhow::anyhow!("Failed to attach thread input: {}", e))?;
        }

        let result = (|| -> Result<()> {
            if IsIconic(hwnd).as_bool() {
                ShowWindow(hwnd, SW_RESTORE)
                    .ok()
                    .map_err(|e| anyhow::anyhow!("Failed to restore window: {}", e))?;
            }

            let _ = BringWindowToTop(hwnd);

            SetForegroundWindow(hwnd).ok().map_err(|e| {
                anyhow::anyhow!("Failed to set foreground window: {}", e)
            })?;

            Ok(())
        })();

        if attached {
            let _ = AttachThreadInput(foreground_thread, current_thread, false);
        }

        result
    }
}

/// Implement WindowManagerTrait for Windows RealWindowManager
///
/// This bridges the platform-specific HWND type to the unified WindowId (usize)
/// used by the cross-platform trait abstraction.
impl crate::platform::traits::WindowManagerTrait for RealWindowManager {
    fn get_foreground_window(&self) -> Option<crate::platform::traits::WindowId> {
        self.api()
            .get_foreground_window()
            .map(|hwnd| hwnd.0 as usize)
    }

    fn get_window_info(&self, window: crate::platform::traits::WindowId) -> Result<crate::platform::traits::WindowInfo> {
        let hwnd = HWND(window as *mut std::ffi::c_void);
        let info = self.api().get_window_info(hwnd)?;
        Ok(crate::platform::traits::WindowInfo {
            id: window,
            title: info.title,
            process_name: info.process_name,
            executable_path: info.executable_path,
            x: info.frame.x,
            y: info.frame.y,
            width: info.frame.width,
            height: info.frame.height,
        })
    }

    fn set_window_pos(
        &self,
        window: crate::platform::traits::WindowId,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<()> {
        let hwnd = HWND(window as *mut std::ffi::c_void);
        let frame = WindowFrame::new(x, y, width, height);
        self.set_window_frame(hwnd, &frame)
    }

    fn minimize_window(&self, window: crate::platform::traits::WindowId) -> Result<()> {
        let hwnd = HWND(window as *mut std::ffi::c_void);
        self.minimize_window(hwnd)
    }

    fn maximize_window(&self, window: crate::platform::traits::WindowId) -> Result<()> {
        let hwnd = HWND(window as *mut std::ffi::c_void);
        self.maximize_window(hwnd)
    }

    fn restore_window(&self, window: crate::platform::traits::WindowId) -> Result<()> {
        let hwnd = HWND(window as *mut std::ffi::c_void);
        self.restore_window(hwnd)
    }

    fn close_window(&self, window: crate::platform::traits::WindowId) -> Result<()> {
        let hwnd = HWND(window as *mut std::ffi::c_void);
        self.close_window(hwnd)
    }

    fn set_topmost(
        &self,
        window: crate::platform::traits::WindowId,
        topmost: bool,
    ) -> Result<()> {
        let hwnd = HWND(window as *mut std::ffi::c_void);
        self.api().set_topmost(hwnd, topmost)
    }

    fn get_monitors(&self) -> Vec<crate::platform::traits::MonitorInfo> {
        unsafe { enumerate_all_monitors() }
    }

    fn move_to_monitor(
        &self,
        window: crate::platform::traits::WindowId,
        monitor_index: usize,
    ) -> Result<()> {
        let hwnd = HWND(window as *mut std::ffi::c_void);
        let direction = MonitorDirection::Index(monitor_index as i32);
        self.move_to_monitor(hwnd, direction)
    }

    fn is_window_valid(&self, window: crate::platform::traits::WindowId) -> bool {
        let hwnd = HWND(window as *mut std::ffi::c_void);
        self.api().is_window_valid(hwnd)
    }

    fn is_minimized(&self, window: crate::platform::traits::WindowId) -> bool {
        let hwnd = HWND(window as *mut std::ffi::c_void);
        self.api().is_minimized(hwnd)
    }

    fn is_maximized(&self, window: crate::platform::traits::WindowId) -> bool {
        let hwnd = HWND(window as *mut std::ffi::c_void);
        self.api().is_maximized(hwnd)
    }

    fn is_topmost(&self, window: crate::platform::traits::WindowId) -> bool {
        let hwnd = HWND(window as *mut std::ffi::c_void);
        self.api().is_topmost(hwnd)
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
