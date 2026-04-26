//! Windows platform implementation

pub mod context;
pub mod input;
pub mod input_device;
pub mod output_device;
pub mod tray;
pub mod window_api;
pub mod window_event_hook;
pub mod window_manager;
pub mod window_preset;

pub use crate::platform::launcher_common::Launcher;
pub use input_device::RawInputDevice;
pub use output_device::SendInputDevice;
pub use tray::{run_tray_message_loop, stop_tray, TrayIcon};
pub use window_api::RealWindowApi;
pub use window_event_hook::WindowEventHook;
pub use window_manager::{MonitorDirection, WindowManager};
pub use window_preset::WindowPresetManager;

#[cfg(test)]
pub use window_api::MockWindowApi;

use crate::platform::traits::{ContextProvider, PlatformUtilities};
use crate::types::ModifierState;

/// Windows platform utilities
pub struct WindowsPlatform;

impl PlatformUtilities for WindowsPlatform {
    fn get_modifier_state() -> ModifierState {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            GetAsyncKeyState, VK_CONTROL, VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_MENU,
            VK_RCONTROL, VK_RMENU, VK_RSHIFT, VK_SHIFT,
        };

        let mut modifiers = ModifierState::default();

        unsafe {
            if GetAsyncKeyState(VK_SHIFT.0 as i32) < 0
                || GetAsyncKeyState(VK_LSHIFT.0 as i32) < 0
                || GetAsyncKeyState(VK_RSHIFT.0 as i32) < 0
            {
                modifiers.shift = true;
            }

            if GetAsyncKeyState(VK_CONTROL.0 as i32) < 0
                || GetAsyncKeyState(VK_LCONTROL.0 as i32) < 0
                || GetAsyncKeyState(VK_RCONTROL.0 as i32) < 0
            {
                modifiers.ctrl = true;
            }

            if GetAsyncKeyState(VK_MENU.0 as i32) < 0
                || GetAsyncKeyState(VK_LMENU.0 as i32) < 0
                || GetAsyncKeyState(VK_RMENU.0 as i32) < 0
            {
                modifiers.alt = true;
            }

            if GetAsyncKeyState(0x5B) < 0 || GetAsyncKeyState(0x5C) < 0 {
                modifiers.meta = true;
            }
        }

        modifiers
    }

    fn get_process_name_by_pid(pid: u32) -> anyhow::Result<String> {
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::System::ProcessStatus::GetModuleBaseNameW;
        use windows::Win32::System::Threading::{
            OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ,
        };

        unsafe {
            let handle =
                OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid)
                    .map_err(|e| anyhow::anyhow!("Failed to open process: {}", e))?;

            let mut buffer = [0u16; 260];
            let len = GetModuleBaseNameW(handle, None, &mut buffer);

            CloseHandle(handle).ok();

            if len == 0 {
                return Err(anyhow::anyhow!("Failed to get process name"));
            }

            Ok(String::from_utf16_lossy(&buffer[..len as usize]))
        }
    }

    fn get_executable_path_by_pid(pid: u32) -> anyhow::Result<String> {
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::System::ProcessStatus::GetModuleFileNameExW;
        use windows::Win32::System::Threading::{
            OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ,
        };

        unsafe {
            let handle =
                OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid)
                    .map_err(|e| anyhow::anyhow!("Failed to open process: {}", e))?;

            let mut buffer = [0u16; 260];
            let len = GetModuleFileNameExW(Some(handle), None, &mut buffer);

            CloseHandle(handle).ok();

            if len == 0 {
                return Err(anyhow::anyhow!("Failed to get executable path"));
            }

            Ok(String::from_utf16_lossy(&buffer[..len as usize]))
        }
    }
}

impl ContextProvider for WindowsPlatform {
    fn get_current_context() -> Option<crate::platform::traits::WindowContext> {
        context::get_current()
    }
}

/// Windows notification service using tray icon
///
/// Wraps the tray icon notification functionality to implement
/// the cross-platform [NotificationService] trait.
#[allow(dead_code)]
pub struct WindowsNotificationService {
    message_window_hwnd: std::sync::RwLock<Option<isize>>,
}

#[allow(dead_code)]
impl WindowsNotificationService {
    pub fn new() -> Self {
        Self {
            message_window_hwnd: std::sync::RwLock::new(None),
        }
    }

    pub fn set_message_window_hwnd(&self, hwnd: isize) {
        let mut handle = self.message_window_hwnd.write().unwrap();
        *handle = Some(hwnd);
    }
}

impl Default for WindowsNotificationService {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::platform::traits::NotificationService for WindowsNotificationService {
    fn show(&self, title: &str, message: &str) -> Result<()> {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::Shell::{
            NIF_INFO, NIM_MODIFY, NOTIFYICONDATAW, NOTIFY_ICON_INFOTIP_FLAGS,
        };

        let hwnd_value = {
            let handle = self.message_window_hwnd.read().unwrap();
            *handle
        };

        match hwnd_value {
            Some(hwnd_isize) => {
                let hwnd = HWND(hwnd_isize as *mut std::ffi::c_void);

                let mut nid = NOTIFYICONDATAW {
                    cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                    hWnd: hwnd,
                    uID: 1,
                    uFlags: NIF_INFO,
                    ..Default::default()
                };

                let title_wide: Vec<u16> =
                    title.encode_utf16().chain(std::iter::once(0)).collect();
                let message_wide: Vec<u16> =
                    message.encode_utf16().chain(std::iter::once(0)).collect();

                let title_len = title_wide.len().min(64);
                let message_len = message_wide.len().min(256);

                nid.szInfoTitle[..title_len].copy_from_slice(&title_wide[..title_len]);
                nid.szInfo[..message_len].copy_from_slice(&message_wide[..message_len]);

                nid.dwInfoFlags = NOTIFY_ICON_INFOTIP_FLAGS(0);

                unsafe {
                    windows::Win32::UI::Shell::Shell_NotifyIconW(NIM_MODIFY, &nid)
                        .map_err(|e| {
                            anyhow::anyhow!("Failed to show notification: {}", e)
                        })?;
                }

                tracing::info!("Notification shown: {} - {}", title, message);
                Ok(())
            }
            None => {
                tracing::debug!("Message window not registered, skipping notification");
                Ok(())
            }
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
