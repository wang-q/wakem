//! macOS platform implementation
//!
//! This module provides macOS-specific implementations of the platform traits
//! using Core Graphics, Cocoa, and Accessibility APIs.

pub mod context;
pub mod input;
pub mod input_device;
pub mod native_api;
pub mod output_device;
pub mod tray;
pub mod window_api;
pub mod window_event_hook;
pub mod window_manager;
pub mod window_preset;

// Re-export common types (aligned with Windows platform)
pub use crate::platform::launcher_common::Launcher;
pub use input_device::RawInputDevice;
pub use output_device::SendInputDevice;
pub use tray::TrayIcon;
pub use window_api::RealWindowApi;

#[cfg(test)]
pub use window_api::MockWindowApi;

pub use window_event_hook::WindowEventHook;
pub use window_manager::{MonitorDirection, WindowManager};
pub use window_preset::WindowPresetManager;

use crate::platform::traits::{
    ApplicationControl, ContextProvider, InputDeviceConfig, LauncherTrait,
    PlatformFactory, PlatformUtilities, TrayLifecycle, WindowEventHookTrait,
    WindowPresetManagerTrait,
};
use crate::types::ModifierState;

/// macOS platform utilities
pub struct MacosPlatform;

impl PlatformUtilities for MacosPlatform {
    fn get_modifier_state() -> ModifierState {
        use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

        let mut modifiers = ModifierState::default();

        if let Ok(source) = CGEventSource::new(CGEventSourceStateID::HIDSystemState) {
            if let Ok(event) = core_graphics::event::CGEvent::new(source) {
                let flags = event.get_flags();

                if flags.contains(core_graphics::event::CGEventFlags::CGEventFlagShift) {
                    modifiers.shift = true;
                }
                if flags.contains(core_graphics::event::CGEventFlags::CGEventFlagControl)
                {
                    modifiers.ctrl = true;
                }
                if flags
                    .contains(core_graphics::event::CGEventFlags::CGEventFlagAlternate)
                {
                    modifiers.alt = true;
                }
                if flags.contains(core_graphics::event::CGEventFlags::CGEventFlagCommand)
                {
                    modifiers.meta = true;
                }
            }
        }

        modifiers
    }

    fn get_process_name_by_pid(pid: u32) -> anyhow::Result<String> {
        let path = get_process_path(pid)?;
        let process_name = path.split('/').next_back().unwrap_or("").to_string();

        if process_name.is_empty() {
            return Err(anyhow::anyhow!("Failed to extract process name from path"));
        }

        Ok(process_name)
    }

    fn get_executable_path_by_pid(pid: u32) -> anyhow::Result<String> {
        get_process_path(pid)
    }
}

impl ContextProvider for MacosPlatform {
    fn get_current_context() -> Option<crate::platform::traits::WindowContext> {
        context::get_current()
    }
}

/// macOS notification service using native notification center API
pub struct MacosNotificationService;

impl MacosNotificationService {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MacosNotificationService {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::platform::traits::NotificationService for MacosNotificationService {
    fn show(&self, title: &str, message: &str) -> anyhow::Result<()> {
        use crate::platform::macos::native_api::notification::show_notification;

        match show_notification(title, message) {
            Ok(()) => {
                tracing::info!("Notification shown: {} - {}", title, message);
                Ok(())
            }
            Err(e) => {
                tracing::warn!("Failed to show notification: {}", e);
                Ok(())
            }
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

impl WindowPresetManagerTrait for WindowPresetManager {
    fn load_presets(&mut self, presets: Vec<crate::config::WindowPreset>) {
        WindowPresetManager::load_presets(self, presets);
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
        self.apply_preset_for_window_by_id(window_id)
    }

    fn apply_preset_for_window(&self) -> anyhow::Result<bool> {
        self.apply_preset_for_window()
    }
}

impl LauncherTrait for Launcher {
    fn launch(&self, action: &crate::types::LaunchAction) -> anyhow::Result<()> {
        self.launch(action)
    }

    fn open(&self, path: &str) -> anyhow::Result<()> {
        self.open(path)
    }
}

impl TrayLifecycle for MacosPlatform {
    fn run_tray_message_loop(
        callback: Box<dyn Fn(crate::platform::traits::AppCommand) + Send>,
    ) -> anyhow::Result<()> {
        tray::run_tray_message_loop(callback)
    }

    fn stop_tray() {
        tray::stop_tray()
    }
}

impl ApplicationControl for MacosPlatform {
    fn detach_console() {
        // macOS has no console to detach
    }

    fn terminate_application() {
        #[allow(deprecated)]
        unsafe {
            use cocoa::base::nil;
            use objc::runtime::Class;
            use objc::{msg_send, sel, sel_impl};

            let app_class = Class::get("NSApplication").unwrap();
            let app: *mut objc::runtime::Object =
                msg_send![app_class, sharedApplication];
            if app != nil {
                let _: () = msg_send![app, terminate: nil];
            }
        }
    }

    fn open_folder(path: &std::path::Path) -> anyhow::Result<()> {
        std::process::Command::new("open").arg(path).spawn()?;
        Ok(())
    }

    fn force_kill_instance(_instance_id: u32) -> anyhow::Result<()> {
        // macOS doesn't have the same multi-instance model as Windows
        Ok(())
    }
}

impl PlatformFactory for MacosPlatform {
    fn create_input_device(
        _config: InputDeviceConfig,
        sender: Option<std::sync::mpsc::Sender<crate::types::InputEvent>>,
    ) -> anyhow::Result<Box<dyn crate::platform::traits::InputDeviceTrait>> {
        let device = match sender {
            Some(tx) => RawInputDevice::with_sender(tx)?,
            None => RawInputDevice::new(InputDeviceConfig::default())?,
        };
        Ok(Box::new(device))
    }

    fn create_output_device(
    ) -> Box<dyn crate::platform::traits::OutputDeviceTrait + Send + Sync> {
        Box::new(SendInputDevice::new())
    }

    fn create_window_manager() -> Box<dyn crate::platform::traits::WindowManagerTrait> {
        Box::new(WindowManager::new())
    }

    fn create_window_preset_manager(
    ) -> Box<dyn crate::platform::traits::WindowPresetManagerTrait> {
        Box::new(WindowPresetManager::new(WindowManager::new()))
    }

    fn create_notification_service(
    ) -> Box<dyn crate::platform::traits::NotificationService> {
        Box::new(MacosNotificationService::new())
    }

    fn create_launcher() -> Box<dyn crate::platform::traits::LauncherTrait> {
        Box::new(Launcher::new())
    }

    fn create_window_event_hook(
        sender: std::sync::mpsc::Sender<crate::platform::traits::PlatformWindowEvent>,
    ) -> Box<dyn crate::platform::traits::WindowEventHookTrait> {
        Box::new(WindowEventHook::new(sender))
    }
}

/// Get full executable path for a process using proc_pidpath (internal helper)
#[allow(dead_code)]
fn get_process_path(pid: u32) -> anyhow::Result<String> {
    use libc::proc_pidpath;
    use std::ffi::CStr;

    let mut path_buf = [0u8; 4096];
    let path_len =
        unsafe { proc_pidpath(pid as i32, path_buf.as_mut_ptr() as *mut _, 4096) };

    if path_len <= 0 {
        return Err(anyhow::anyhow!(
            "Failed to get process path for pid {}",
            pid
        ));
    }

    Ok(unsafe { CStr::from_ptr(path_buf.as_ptr() as *const _) }
        .to_string_lossy()
        .to_string())
}
