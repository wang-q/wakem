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
use super::window_api::{RealWindowApi, WindowApi};
#[allow(unused_imports)]
pub use crate::types::{Alignment, Edge};
// Import common window manager and shared types
use crate::platform::common::window_manager::CommonWindowApi;
use crate::platform::traits::{
    MonitorInfo, MonitorWorkArea, WindowFrame, WindowInfoProvider,
};

/// Monitor direction (for moving between displays)
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum MonitorDirection {
    Next,
    Prev,
    Index(i32),
}

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

/// Windows-specific window information
#[derive(Debug, Clone)]
pub struct WindowsWindowInfo {
    pub hwnd: HWND,
    pub title: String,
    pub frame: WindowFrame,
    pub work_area: MonitorWorkArea,
}

impl WindowInfoProvider for WindowsWindowInfo {
    fn x(&self) -> i32 {
        self.frame.x
    }

    fn y(&self) -> i32 {
        self.frame.y
    }

    fn width(&self) -> i32 {
        self.frame.width
    }

    fn height(&self) -> i32 {
        self.frame.height
    }
}

/// Window manager (generic version)
#[allow(dead_code)]
pub struct WindowManager<A: WindowApi> {
    api: A,
}

/// Type alias for window manager using real Windows API
pub type RealWindowManager = WindowManager<RealWindowApi>;

impl WindowManager<RealWindowApi> {
    /// Create a window manager using real Windows API
    pub fn new() -> Self {
        Self {
            api: RealWindowApi::new(),
        }
    }
}

impl Default for WindowManager<RealWindowApi> {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl<A: WindowApi> WindowManager<A> {
    /// Create a window manager with specified API implementation
    pub fn with_api(api: A) -> Self {
        Self { api }
    }

    /// Get API reference (for testing)
    pub fn api(&self) -> &A {
        &self.api
    }

    /// Get foreground window information
    pub fn get_foreground_window_info(&self) -> Result<WindowsWindowInfo> {
        let hwnd = self
            .api
            .get_foreground_window()
            .ok_or_else(|| anyhow::anyhow!("No foreground window"))?;
        self.get_window_info(hwnd)
    }

    /// Get specified window information
    pub fn get_window_info(&self, hwnd: HWND) -> Result<WindowsWindowInfo> {
        if !self.api.is_window(hwnd) {
            return Err(anyhow::anyhow!("Invalid window handle"));
        }

        // Get window title
        let title = self.api.get_window_title(hwnd).unwrap_or_default();

        // Get window position
        let frame = self
            .api
            .get_window_rect(hwnd)
            .ok_or_else(|| anyhow::anyhow!("Failed to get window rect"))?;

        // Get monitor work area
        let work_area = self
            .api
            .get_monitor_work_area(hwnd)
            .ok_or_else(|| anyhow::anyhow!("Failed to get monitor work area"))?;

        debug!(
            "Window info: hwnd={:?}, title={}, frame={:?}, work_area={:?}",
            hwnd, title, frame, work_area
        );

        Ok(WindowsWindowInfo {
            hwnd,
            title,
            frame,
            work_area,
        })
    }

    /// Get debug info string
    pub fn get_debug_info(&self) -> Result<String> {
        let info = self.get_foreground_window_info()?;

        Ok(format!(
            "Window: {}\nID: {:?}\nPosition: [{}, {}]\nSize: {} x {}\nMonitor: [{} x {}]",
            info.title,
            info.hwnd,
            info.frame.x,
            info.frame.y,
            info.frame.width,
            info.frame.height,
            info.work_area.width,
            info.work_area.height
        ))
    }

    /// Set window position and size
    pub fn set_window_frame(&self, hwnd: HWND, frame: &WindowFrame) -> Result<()> {
        self.api.ensure_window_restored(hwnd)?;
        self.api
            .set_window_pos(hwnd, frame.x, frame.y, frame.width, frame.height)?;

        debug!(
            "Window moved to: x={}, y={}, width={}, height={}",
            frame.x, frame.y, frame.width, frame.height
        );

        Ok(())
    }

    /// Minimize window
    pub fn minimize_window(&self, hwnd: HWND) -> Result<()> {
        self.api.minimize_window(hwnd)
    }

    /// Maximize window
    pub fn maximize_window(&self, hwnd: HWND) -> Result<()> {
        self.api.maximize_window(hwnd)
    }

    /// Restore window
    pub fn restore_window(&self, hwnd: HWND) -> Result<()> {
        self.api.restore_window(hwnd)
    }

    /// Close window
    pub fn close_window(&self, hwnd: HWND) -> Result<()> {
        self.api.close_window(hwnd)
    }
}

// Implement CommonWindowApi for WindowManager to use common window manager logic
impl<A: WindowApi + 'static> CommonWindowApi for WindowManager<A> {
    type WindowId = HWND;
    type WindowInfo = WindowsWindowInfo;

    fn get_foreground_window(&self) -> Option<Self::WindowId> {
        self.api.get_foreground_window()
    }

    fn api(&self) -> &dyn std::any::Any {
        self
    }

    fn get_window_info(&self, window: Self::WindowId) -> Result<Self::WindowInfo> {
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
        let frame = WindowFrame::new(x, y, width, height);
        self.set_window_frame(window, &frame)
    }

    fn minimize_window(&self, window: Self::WindowId) -> Result<()> {
        self.api.minimize_window(window)
    }

    fn maximize_window(&self, window: Self::WindowId) -> Result<()> {
        self.api.maximize_window(window)
    }

    fn restore_window(&self, window: Self::WindowId) -> Result<()> {
        self.api.restore_window(window)
    }

    fn close_window(&self, window: Self::WindowId) -> Result<()> {
        self.api.close_window(window)
    }

    fn get_monitors(&self) -> Vec<MonitorInfo> {
        #[cfg(not(test))]
        {
            unsafe { enumerate_all_monitors() }
        }
        #[cfg(test)]
        {
            if let Some(hwnd) = self.api.get_foreground_window() {
                if let Some(monitor) = self.api.get_monitor_info(hwnd) {
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
        self.api.is_window(window)
    }

    fn is_maximized(&self, window: Self::WindowId) -> bool {
        self.api.is_zoomed(window)
    }

    fn is_topmost(&self, window: Self::WindowId) -> bool {
        self.api.is_topmost(window)
    }

    fn set_topmost(&self, window: Self::WindowId, topmost: bool) -> Result<()> {
        self.api.set_topmost(window, topmost)
    }
}

/// Features requiring real Windows API (cross-monitor movement, window switching, etc.)
impl RealWindowManager {
    /// Move window to another monitor
    pub fn move_to_monitor(
        &self,
        hwnd: HWND,
        direction: MonitorDirection,
    ) -> Result<()> {
        unsafe {
            // Get all monitors
            let monitors = enumerate_all_monitors();
            if monitors.len() < 2 {
                debug!("Only one monitor, nothing to do");
                return Ok(());
            }

            // Get current window's monitor index
            let current_monitor_index =
                self.get_current_monitor_index(hwnd, &monitors)?;

            // Calculate target monitor index
            let target_index = match direction {
                MonitorDirection::Next => (current_monitor_index + 1) % monitors.len(),
                MonitorDirection::Prev => {
                    if current_monitor_index == 0 {
                        monitors.len() - 1
                    } else {
                        current_monitor_index - 1
                    }
                }
                MonitorDirection::Index(idx) => {
                    let idx = idx as usize;
                    if idx >= monitors.len() {
                        return Err(anyhow::anyhow!("Invalid monitor index: {}", idx));
                    }
                    idx
                }
            };

            let target_monitor = &monitors[target_index];
            let current_monitor = &monitors[current_monitor_index];

            // Get current window info
            let info = self.get_window_info(hwnd)?;

            // Calculate relative position ratio
            let rel_x =
                (info.frame.x - current_monitor.x) as f32 / current_monitor.width as f32;
            let rel_y = (info.frame.y - current_monitor.y) as f32
                / current_monitor.height as f32;
            let rel_width = info.frame.width as f32 / current_monitor.width as f32;
            let rel_height = info.frame.height as f32 / current_monitor.height as f32;

            // Calculate new position (maintain relative position and size ratio)
            let new_x = target_monitor.x + (rel_x * target_monitor.width as f32) as i32;
            let new_y = target_monitor.y + (rel_y * target_monitor.height as f32) as i32;
            let new_width = (rel_width * target_monitor.width as f32) as i32;
            let new_height = (rel_height * target_monitor.height as f32) as i32;

            let new_frame = WindowFrame::new(new_x, new_y, new_width, new_height);
            self.set_window_frame(hwnd, &new_frame)?;

            debug!(
                "Moved window from monitor {} to monitor {}: {:?}",
                current_monitor_index, target_index, new_frame
            );

            Ok(())
        }
    }

    /// Get the index of the monitor where the window is currently located
    unsafe fn get_current_monitor_index(
        &self,
        hwnd: HWND,
        monitors: &[MonitorInfo],
    ) -> Result<usize> {
        let mut rect = RECT::default();
        GetWindowRect(hwnd, &mut rect)?;

        let window_center_x = rect.left + (rect.right - rect.left) / 2;
        let window_center_y = rect.top + (rect.bottom - rect.top) / 2;

        for (i, monitor) in monitors.iter().enumerate() {
            if window_center_x >= monitor.x
                && window_center_x < monitor.x + monitor.width
                && window_center_y >= monitor.y
                && window_center_y < monitor.y + monitor.height
            {
                return Ok(i);
            }
        }

        // Default to first monitor
        Ok(0)
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
