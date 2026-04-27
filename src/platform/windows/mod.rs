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

use crate::platform::traits::{
    ApplicationControl, ContextProvider, InputDeviceConfig, LauncherTrait,
    NotificationService, PlatformFactory, PlatformUtilities, TrayLifecycle,
    WindowEventHookTrait, WindowPresetManagerTrait,
};
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
pub struct WindowsNotificationService {
    message_window_hwnd: std::sync::RwLock<Option<isize>>,
}

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

    fn initialize(&self, ctx: &crate::platform::traits::NotificationInitContext) {
        if let Some(h) = ctx.native_handle {
            self.set_message_window_hwnd(h as isize);
        }
    }
}

impl WindowEventHookTrait for WindowEventHook {
    fn start_with_shutdown(
        &mut self,
        shutdown_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) -> anyhow::Result<()> {
        self.start_with_shutdown(shutdown_flag)
    }

    fn stop(&mut self) {
        self.stop()
    }

    fn shutdown_flag(&self) -> std::sync::Arc<std::sync::atomic::AtomicBool> {
        self.shutdown_flag()
    }
}

impl TrayLifecycle for WindowsPlatform {
    fn run_tray_message_loop(
        callback: Box<dyn Fn(crate::platform::traits::AppCommand) + Send>,
    ) -> anyhow::Result<()> {
        tray::run_tray_message_loop(callback)
    }

    fn stop_tray() {
        tray::stop_tray()
    }
}

impl ApplicationControl for WindowsPlatform {
    fn detach_console() {
        unsafe {
            let _ = windows::Win32::System::Console::FreeConsole();
        }
    }

    fn terminate_application() {
        // Windows tray mode uses PostQuitMessage
        // This is handled by the tray message loop
    }

    fn open_folder(path: &std::path::Path) -> anyhow::Result<()> {
        std::process::Command::new("explorer").arg(path).spawn()?;
        Ok(())
    }

    fn force_kill_instance(instance_id: u32) -> anyhow::Result<()> {
        use std::process::{Command, Stdio};

        let window_title = if instance_id == 0 {
            "wakemd".to_string()
        } else {
            format!("wakemd-instance{}", instance_id)
        };

        let output = Command::new("taskkill")
            .args(["/F", "/FI", &format!("WINDOWTITLE eq {}", window_title)])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output();

        match output {
            Ok(result) if result.status.success() => {
                tracing::info!("Successfully killed daemon instance {}", instance_id);
                Ok(())
            }
            _ => {
                tracing::warn!(
                    "Could not kill by window title, trying PowerShell fallback"
                );
                let ps_script = if instance_id == 0 {
                    r#"Get-Process wakem -ErrorAction SilentlyContinue | Where-Object { $_.CommandLine -notmatch '--instance' } | Stop-Process -Force"#.to_string()
                } else {
                    format!(
                        r#"Get-Process wakem -ErrorAction SilentlyContinue | Where-Object {{ $_.CommandLine -match '--instance {}' }} | Stop-Process -Force"#,
                        instance_id
                    )
                };

                let output = Command::new("powershell")
                    .args(["-NoProfile", "-NonInteractive", "-Command", &ps_script])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .output();

                match output {
                    Ok(result) if result.status.success() => {
                        tracing::info!(
                            "Successfully killed daemon instance {} via PowerShell",
                            instance_id
                        );
                        Ok(())
                    }
                    _ => Err(anyhow::anyhow!(
                        "Failed to kill daemon instance {}",
                        instance_id
                    )),
                }
            }
        }
    }
}

impl PlatformFactory for WindowsPlatform {
    type InputDevice = RawInputDevice;
    type OutputDevice = SendInputDevice;
    type WindowManager = WindowManager<window_api::RealWindowApi>;
    type WindowPresetManager = WindowPresetManager;
    type NotificationService = WindowsNotificationService;
    type Launcher = Launcher;
    type WindowEventHook = WindowEventHook;

    fn create_input_device(
        _config: InputDeviceConfig,
        sender: Option<std::sync::mpsc::Sender<crate::types::InputEvent>>,
    ) -> anyhow::Result<Self::InputDevice> {
        match sender {
            Some(tx) => RawInputDevice::with_sender(tx),
            None => RawInputDevice::new(InputDeviceConfig::default()),
        }
    }

    fn create_output_device() -> Self::OutputDevice {
        SendInputDevice::new()
    }

    fn create_window_manager() -> Self::WindowManager {
        WindowManager::new()
    }

    fn create_window_preset_manager() -> Self::WindowPresetManager {
        WindowPresetManager::new(WindowManager::new())
    }

    fn create_notification_service() -> Self::NotificationService {
        WindowsNotificationService::new()
    }

    fn create_launcher() -> Self::Launcher {
        Launcher::new()
    }

    fn create_window_event_hook(
        sender: std::sync::mpsc::Sender<crate::platform::traits::PlatformWindowEvent>,
    ) -> Self::WindowEventHook {
        WindowEventHook::new(sender)
    }
}
