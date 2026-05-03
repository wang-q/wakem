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

use crate::platform::traits::PlatformFactory;
use crate::platform::types::*;
use anyhow::Result;
use std::sync::Arc;

pub use crate::platform::common::launcher::Launcher;
pub use input_device::RawInputDevice;
pub use output_device::SendInputDevice;
pub use window_api::RealWindowApi;
pub use window_event_hook::WindowEventHook;
pub use window_manager::WindowManager;
pub use window_preset::WindowPresetManager;

/// Windows platform type
pub struct WindowsPlatform;

impl WindowsPlatform {
    /// Get current modifier state
    pub fn get_modifier_state() -> crate::types::ModifierState {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            GetAsyncKeyState, VK_CONTROL, VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_MENU,
            VK_RCONTROL, VK_RMENU, VK_RSHIFT, VK_SHIFT,
        };

        let mut modifiers = crate::types::ModifierState::default();

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

    /// Get current window context
    pub fn get_current_context() -> Option<WindowContext> {
        context::get_current()
    }

    /// Run tray message loop
    pub fn run_tray_message_loop(
        callback: Box<dyn Fn(AppCommand) + Send>,
    ) -> Result<()> {
        tray::run_tray_message_loop(callback)
    }

    /// Stop tray
    pub fn stop_tray() {
        tray::stop_tray()
    }

    /// Detach console
    pub fn detach_console() {
        use windows::Win32::System::Console::FreeConsole;
        unsafe {
            let _ = FreeConsole();
        }
    }

    /// Terminate application
    pub fn terminate_application() {
        Self::stop_tray()
    }

    /// Open folder in explorer
    pub fn open_folder(path: &std::path::Path) -> Result<()> {
        std::process::Command::new("explorer").arg(path).spawn()?;
        Ok(())
    }

    /// Force kill daemon instance
    pub fn force_kill_instance(instance_id: u32) -> Result<()> {
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
                return Ok(());
            }
            _ => {
                tracing::debug!("Could not kill by window title, trying PowerShell");
            }
        }

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
            _ => {
                anyhow::bail!("Failed to kill daemon instance {}", instance_id)
            }
        }
    }
}

impl PlatformFactory for WindowsPlatform {
    type InputDevice = RawInputDevice;
    type OutputDevice = SendInputDevice;
    type WindowManager = WindowManager;
    type WindowPresetManager = WindowPresetManager;
    type NotificationService = WindowsNotificationService;
    type Launcher = Launcher;
    type WindowEventHook = WindowEventHook;

    fn create_input_device(
        _config: InputDeviceConfig,
        sender: Option<std::sync::mpsc::Sender<crate::types::InputEvent>>,
    ) -> Result<Self::InputDevice> {
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
        sender: std::sync::mpsc::Sender<PlatformWindowEvent>,
    ) -> Self::WindowEventHook {
        WindowEventHook::new(sender)
    }
}

/// Windows notification service using tray balloon notifications
pub struct WindowsNotificationService {
    hwnd: Arc<std::sync::RwLock<Option<isize>>>,
}

impl WindowsNotificationService {
    pub fn new() -> Self {
        Self {
            hwnd: Arc::new(std::sync::RwLock::new(None)),
        }
    }

    pub fn set_hwnd(&self, hwnd_value: isize) {
        if let Ok(mut guard) = self.hwnd.write() {
            *guard = Some(hwnd_value);
        }
    }

    /// Show notification
    pub fn show(&self, title: &str, message: &str) -> Result<()> {
        let hwnd_value = self.hwnd.read().ok().and_then(|guard| *guard);

        let Some(hwnd_value) = hwnd_value else {
            tracing::debug!("Message window not registered, skipping notification");
            return Ok(());
        };

        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::Shell::{
            NIF_INFO, NIM_MODIFY, NOTIFYICONDATAW, NOTIFY_ICON_INFOTIP_FLAGS,
        };

        let hwnd = HWND(hwnd_value as *mut std::ffi::c_void);

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
            let result = windows::Win32::UI::Shell::Shell_NotifyIconW(NIM_MODIFY, &nid);
            if !result.as_bool() {
                return Err(anyhow::anyhow!("Failed to show notification"));
            }
        }

        tracing::info!("Notification shown: {} - {}", title, message);
        Ok(())
    }

    /// Initialize with context
    pub fn initialize(&self, ctx: &NotificationInitContext) {
        if let Some(handle) = ctx.native_handle {
            self.set_hwnd(handle as isize);
        }
    }
}

impl Default for WindowsNotificationService {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::platform::traits::NotificationService for WindowsNotificationService {
    fn show(&self, title: &str, message: &str) -> Result<()> {
        self.show(title, message)
    }

    fn initialize(&self, ctx: &NotificationInitContext) {
        self.initialize(ctx)
    }
}

impl crate::platform::traits::Launcher for Launcher {
    fn launch(&self, action: &crate::types::LaunchAction) -> Result<()> {
        self.launch(action)
    }
}

/// Helper functions
pub fn get_process_name_by_pid(pid: u32) -> anyhow::Result<String> {
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

pub fn get_executable_path_by_pid(pid: u32) -> anyhow::Result<String> {
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
