//! Windows window API implementation
#![cfg(target_os = "windows")]

use anyhow::Result;
use windows::Win32::Foundation::{HWND, LPARAM, RECT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowRect, IsIconic, IsWindow, IsZoomed, SetWindowPos,
    ShowWindow, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOOWNERZORDER, SW_RESTORE,
};

use crate::platform::traits::WindowApiBase;
use crate::platform::types::{MonitorInfo, WindowFrame, WindowId};

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

impl WindowApiBase for RealWindowApi {
    type WindowId = WindowId;

    fn get_foreground_window(&self) -> Option<Self::WindowId> {
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.0.is_null() {
                None
            } else {
                Some(hwnd_to_window_id(hwnd))
            }
        }
    }

    fn set_window_pos(
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

    fn minimize_window(&self, window: Self::WindowId) -> Result<()> {
        unsafe {
            let hwnd = window_id_to_hwnd(window);
            ShowWindow(hwnd, windows::Win32::UI::WindowsAndMessaging::SW_MINIMIZE)
                .ok()
                .map_err(|e| anyhow::anyhow!("Failed to minimize window: {}", e))?;
            Ok(())
        }
    }

    fn maximize_window(&self, window: Self::WindowId) -> Result<()> {
        unsafe {
            let hwnd = window_id_to_hwnd(window);
            ShowWindow(hwnd, windows::Win32::UI::WindowsAndMessaging::SW_MAXIMIZE)
                .ok()
                .map_err(|e| anyhow::anyhow!("Failed to maximize window: {}", e))?;
            Ok(())
        }
    }

    fn restore_window(&self, window: Self::WindowId) -> Result<()> {
        unsafe {
            let hwnd = window_id_to_hwnd(window);
            ShowWindow(hwnd, SW_RESTORE)
                .ok()
                .map_err(|e| anyhow::anyhow!("Failed to restore window: {}", e))?;
            Ok(())
        }
    }

    fn close_window(&self, window: Self::WindowId) -> Result<()> {
        unsafe {
            let hwnd = window_id_to_hwnd(window);
            use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_CLOSE};
            PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0))
                .map_err(|e| anyhow::anyhow!("Failed to post WM_CLOSE: {}", e))?;
            Ok(())
        }
    }

    fn set_topmost(&self, window: Self::WindowId, topmost: bool) -> Result<()> {
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

    fn is_topmost(&self, window: Self::WindowId) -> bool {
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

    fn is_window_valid(&self, window: Self::WindowId) -> bool {
        unsafe { IsWindow(Some(window_id_to_hwnd(window))).as_bool() }
    }

    fn is_minimized(&self, window: Self::WindowId) -> bool {
        unsafe { IsIconic(window_id_to_hwnd(window)).as_bool() }
    }

    fn is_maximized(&self, window: Self::WindowId) -> bool {
        unsafe { IsZoomed(window_id_to_hwnd(window)).as_bool() }
    }

    fn get_window_title(&self, window: Self::WindowId) -> Option<String> {
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

    fn get_window_rect(&self, window: Self::WindowId) -> Result<WindowFrame> {
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

    fn get_monitors(&self) -> Vec<MonitorInfo> {
        unsafe { enumerate_all_monitors() }
    }

    fn get_monitor_work_area(
        &self,
        monitor_index: usize,
    ) -> Option<crate::platform::types::MonitorWorkArea> {
        let monitors = self.get_monitors();
        monitors.get(monitor_index).map(|m| (*m).into())
    }

    fn get_process_name(&self, window: WindowId) -> Option<String> {
        unsafe {
            let hwnd = window_id_to_hwnd(window);
            let mut pid: u32 = 0;
            windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(
                hwnd,
                Some(&mut pid),
            );
            if pid == 0 {
                return None;
            }
            super::get_process_name_by_pid(pid).ok()
        }
    }

    fn get_executable_path(&self, window: WindowId) -> Option<String> {
        unsafe {
            let hwnd = window_id_to_hwnd(window);
            let mut pid: u32 = 0;
            windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(
                hwnd,
                Some(&mut pid),
            );
            if pid == 0 {
                return None;
            }
            super::get_executable_path_by_pid(pid).ok()
        }
    }

    fn switch_to_next_window_of_same_process(&self) -> Result<()> {
        unsafe {
            let current_hwnd = GetForegroundWindow();
            if current_hwnd.0.is_null() {
                anyhow::bail!("No foreground window");
            }

            let current_pid = self.get_window_process_id(current_hwnd)?;

            let mut windows = match super::get_process_name_by_pid(current_pid) {
                Ok(process_name) => {
                    tracing::debug!(
                        "[SwitchWindow] PID={}, process={}, current_hwnd={:?}",
                        current_pid,
                        process_name,
                        current_hwnd
                    );
                    self.get_app_visible_windows(&process_name)
                }
                Err(e) => {
                    tracing::debug!(
                        "[SwitchWindow] PID={}, failed to get process name ({}), falling back to PID matching",
                        current_pid, e
                    );
                    self.get_process_visible_windows(current_pid)
                }
            };

            // Remove duplicates (same hwnd appearing multiple times)
            let original_len = windows.len();
            windows.sort_by_key(|hwnd| hwnd_to_window_id(*hwnd));
            windows.dedup_by_key(|hwnd| hwnd_to_window_id(*hwnd));
            if windows.len() != original_len {
                tracing::debug!(
                    "[SwitchWindow] Removed {} duplicates",
                    original_len - windows.len()
                );
            }

            // Ensure current window is in the list
            if !windows.contains(&current_hwnd) {
                tracing::debug!(
                    "[SwitchWindow] Current window {:?} not in list, adding it",
                    current_hwnd
                );
                windows.push(current_hwnd);
            }

            tracing::debug!("[SwitchWindow] Total unique windows: {}", windows.len());

            if windows.len() < 2 {
                tracing::debug!(
                    "[SwitchWindow] Only {} window(s), skipping switch",
                    windows.len()
                );
                return Ok(());
            }

            // Sort by WindowId for stable ordering (not by z-order)
            // WindowId is based on platform-native window handle, providing stable ordering
            // This ensures consistent cycling regardless of current z-order
            let mut sorted_windows = windows;
            sorted_windows.sort_by_key(|hwnd| hwnd_to_window_id(*hwnd));

            // Find current window index
            let current_index = sorted_windows
                .iter()
                .position(|&hwnd| hwnd == current_hwnd)
                .unwrap_or(0);

            let next_index = (current_index + 1) % sorted_windows.len();
            let next_hwnd = sorted_windows[next_index];

            tracing::info!(
                "[SwitchWindow] {} windows, current={:?} (idx={}), next={:?} (idx={})",
                sorted_windows.len(),
                current_hwnd,
                current_index,
                next_hwnd,
                next_index
            );

            self.activate_window(next_hwnd)?;
            Ok(())
        }
    }
}

impl RealWindowApi {
    unsafe fn get_window_process_id(&self, hwnd: HWND) -> Result<u32> {
        let mut pid: u32 = 0;
        windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(
            hwnd,
            Some(&mut pid),
        );

        if pid == 0 {
            anyhow::bail!("Failed to get process ID");
        }

        Ok(pid)
    }

    pub fn get_app_visible_windows(&self, target_process_name: &str) -> Vec<HWND> {
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::System::ProcessStatus::GetModuleBaseNameW;
        use windows::Win32::System::Threading::{
            OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ,
        };
        use windows::Win32::UI::WindowsAndMessaging::{
            EnumWindows, GetClassNameW, GetWindow, GetWindowTextW, IsWindowVisible,
            GW_OWNER,
        };
        use windows_core::BOOL;

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

    fn get_process_visible_windows(&self, target_pid: u32) -> Vec<HWND> {
        use windows::Win32::UI::WindowsAndMessaging::{
            EnumWindows, GetWindow, GetWindowTextW, IsWindowVisible, GW_OWNER,
        };
        use windows_core::BOOL;

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

    unsafe fn activate_window(&self, hwnd: HWND) -> Result<()> {
        use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
        use windows::Win32::UI::WindowsAndMessaging::{
            BringWindowToTop, GetForegroundWindow, SetForegroundWindow,
        };

        let foreground_hwnd = GetForegroundWindow();
        let foreground_thread =
            windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(
                foreground_hwnd,
                None,
            );
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
    use super::*;

    #[test]
    fn test_real_window_api_conversions() {
        let hwnd = HWND(12345 as *mut core::ffi::c_void);
        let id = hwnd_to_window_id(hwnd);
        let hwnd2 = window_id_to_hwnd(id);
        assert_eq!(hwnd.0, hwnd2.0);
    }

    #[test]
    fn test_window_api_base_default_methods() {
        let api = RealWindowApi::new();
        let _: &dyn WindowApiBase<WindowId = WindowId> = &api;
    }
}
