//! Windows window manager implementation

use anyhow::Result;
use tracing::debug;
use windows::Win32::Foundation::{HWND, LPARAM, RECT};
use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, GetForegroundWindow, GetWindowRect, GetWindowThreadProcessId,
    IsIconic, SetForegroundWindow, ShowWindow, GW_OWNER, SW_RESTORE,
};
use windows_core::BOOL;

use crate::platform::traits::WindowSwitching;
use crate::platform::types::{MonitorInfo, WindowFrame, WindowId};
use crate::platform::windows::window_api::RealWindowApi;

pub type WindowManager =
    crate::platform::common::window_manager::WindowManager<RealWindowApi>;

/// Monitor direction (for moving between displays)
#[derive(Debug, Clone, Copy)]
pub enum MonitorDirection {
    Next,
    Prev,
    Index(i32),
}

fn hwnd_to_window_id(hwnd: HWND) -> WindowId {
    hwnd.0 as usize
}

impl WindowManager {
    pub fn new() -> Self {
        Self::with_api(RealWindowApi::new())
    }

    pub fn move_to_monitor_hwnd(
        &self,
        hwnd: HWND,
        direction: MonitorDirection,
    ) -> Result<()> {
        unsafe {
            use crate::platform::traits::MonitorOperations;
            let monitors = MonitorOperations::get_monitors(self);
            if monitors.len() < 2 {
                debug!("Only one monitor, nothing to do");
                return Ok(());
            }

            let current_monitor_index =
                self.get_current_monitor_index(hwnd, &monitors)?;

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

            let mut rect = RECT::default();
            GetWindowRect(hwnd, &mut rect)?;
            let frame = WindowFrame::new(
                rect.left,
                rect.top,
                rect.right - rect.left,
                rect.bottom - rect.top,
            );

            let rel_x =
                (frame.x - current_monitor.x) as f32 / current_monitor.width as f32;
            let rel_y =
                (frame.y - current_monitor.y) as f32 / current_monitor.height as f32;
            let rel_width = frame.width as f32 / current_monitor.width as f32;
            let rel_height = frame.height as f32 / current_monitor.height as f32;

            let new_x = target_monitor.x + (rel_x * target_monitor.width as f32) as i32;
            let new_y = target_monitor.y + (rel_y * target_monitor.height as f32) as i32;
            let new_width = (rel_width * target_monitor.width as f32) as i32;
            let new_height = (rel_height * target_monitor.height as f32) as i32;

            let window_id = hwnd_to_window_id(hwnd);
            use crate::platform::traits::WindowOperations;
            WindowOperations::set_window_pos(
                self, window_id, new_x, new_y, new_width, new_height,
            )?;

            debug!(
                "Moved window from monitor {} to monitor {}",
                current_monitor_index, target_index
            );

            Ok(())
        }
    }

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

        Ok(0)
    }

    fn switch_to_next_window_of_same_process_inner(&self) -> Result<()> {
        unsafe {
            let current_hwnd = GetForegroundWindow();
            if current_hwnd.0.is_null() {
                return Err(anyhow::anyhow!("No foreground window"));
            }

            let current_pid = self.get_window_process_id(current_hwnd)?;

            let windows = match super::get_process_name_by_pid(current_pid) {
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
                "[SwitchWindow] Switching index {} -> {}",
                current_index, next_index
            );

            self.activate_window(next_hwnd)?;
            Ok(())
        }
    }

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

    fn sort_windows_by_zorder(&self, windows: Vec<HWND>) -> Vec<HWND> {
        use windows::Win32::UI::WindowsAndMessaging::{EnumWindows, IsWindowVisible};

        unsafe {
            let mut zorder_map: std::collections::HashMap<isize, usize> =
                std::collections::HashMap::new();

            struct EnumData<'a> {
                target_windows: &'a [HWND],
                zorder_map: &'a mut std::collections::HashMap<isize, usize>,
                z_index: usize,
            }

            unsafe extern "system" fn enum_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
                let data = &mut *(lparam.0 as *mut EnumData);

                if !IsWindowVisible(hwnd).as_bool() {
                    return BOOL(1);
                }

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

impl Default for WindowManager {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowSwitching for WindowManager {
    fn switch_to_next_window_of_same_process(&self) -> Result<()> {
        self.switch_to_next_window_of_same_process_inner()
    }
}
