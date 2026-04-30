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

use crate::platform::traits::{
    ApplicationControl, ContextProvider, LauncherTrait, NotificationService,
    PlatformFactory, PlatformUtilities, TrayLifecycle, WindowEventHookTrait,
    WindowPresetManagerTrait,
};
use std::sync::Arc;

pub use crate::platform::common::launcher::Launcher;
pub use input_device::RawInputDevice;
pub use output_device::SendInputDevice;
pub use window_api::RealWindowApi;
pub use window_event_hook::WindowEventHook;
pub use window_manager::WindowManager;

/// Concrete window manager type for Windows platform
pub type WindowsWindowManager = WindowManager<RealWindowApi>;
pub use window_preset::WindowPresetManager;

#[cfg(test)]
pub use window_api::MockWindowApi;

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

// ---- Trait implementations ----

/// Zero-sized type for implementing class-method traits on Windows
pub struct WindowsPlatform;

impl PlatformUtilities for WindowsPlatform {
    fn get_modifier_state() -> crate::types::ModifierState {
        get_modifier_state()
    }
}

impl ContextProvider for WindowsPlatform {
    fn get_current_context() -> Option<crate::platform::traits::WindowContext> {
        context::get_current()
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

impl WindowEventHookTrait for WindowEventHook {
    fn start_with_shutdown(
        &mut self,
        shutdown_flag: Arc<std::sync::atomic::AtomicBool>,
    ) -> anyhow::Result<()> {
        self.start_with_shutdown(shutdown_flag)
    }

    fn stop(&mut self) {
        self.stop()
    }

    fn shutdown_flag(&self) -> Arc<std::sync::atomic::AtomicBool> {
        self.shutdown_flag()
    }
}

impl LauncherTrait for Launcher {
    fn launch(&self, action: &crate::types::LaunchAction) -> anyhow::Result<()> {
        self.launch(action)
    }
}

impl WindowPresetManagerTrait for WindowPresetManager {
    fn load_presets(&mut self, presets: Vec<crate::config::WindowPreset>) {
        self.load_presets(presets)
    }

    fn save_preset(&mut self, name: String) -> anyhow::Result<()> {
        self.save_preset(name)
    }

    fn load_preset(&self, name: &str) -> anyhow::Result<()> {
        self.load_preset(name)
    }

    fn get_foreground_window_info(
        &self,
    ) -> Option<anyhow::Result<crate::platform::traits::WindowInfo>> {
        self.get_foreground_window_info()
    }

    fn apply_preset_for_window_by_id(
        &self,
        window_id: crate::platform::traits::WindowId,
    ) -> anyhow::Result<bool> {
        use windows::Win32::Foundation::HWND;
        let hwnd = HWND(window_id as *mut core::ffi::c_void);
        self.apply_preset_for_window_by_id(hwnd)
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

    /// Set the message window handle for showing tray notifications
    pub fn set_hwnd(&self, hwnd_value: isize) {
        if let Ok(mut guard) = self.hwnd.write() {
            *guard = Some(hwnd_value);
        }
    }
}

impl Default for WindowsNotificationService {
    fn default() -> Self {
        Self::new()
    }
}

impl NotificationService for WindowsNotificationService {
    fn show(&self, title: &str, message: &str) -> anyhow::Result<()> {
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

    fn set_message_window_handle(&self, hwnd: isize) {
        self.set_hwnd(hwnd);
    }
}

impl ApplicationControl for WindowsPlatform {
    fn detach_console() {
        use windows::Win32::System::Console::FreeConsole;
        unsafe {
            let _ = FreeConsole();
        }
    }

    fn terminate_application() {
        <Self as TrayLifecycle>::stop_tray()
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
    type WindowManager = WindowsWindowManager;
    type WindowPresetManager = WindowPresetManager;
    type NotificationService = WindowsNotificationService;
    type Launcher = Launcher;
    type WindowEventHook = WindowEventHook;

    fn create_input_device(
        config: crate::platform::traits::InputDeviceConfig,
        sender: Option<std::sync::mpsc::Sender<crate::types::InputEvent>>,
    ) -> anyhow::Result<Self::InputDevice> {
        match sender {
            Some(tx) => RawInputDevice::with_sender(tx),
            None => RawInputDevice::new(config),
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
