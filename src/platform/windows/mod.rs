//! Windows platform implementation
#![cfg(target_os = "windows")]

pub mod app_control;
pub mod context;
pub mod input;
pub mod input_device;
pub mod notification;
pub mod output_device;
pub mod platform_utils;
pub mod tray;
pub mod window_api;
pub mod window_event_hook;
pub mod window_manager;
pub mod window_preset;

use crate::platform::traits::{
    Launcher as LauncherTrait, PlatformFactory, TrayLifecycle,
};
use anyhow::Result;

pub use crate::platform::common::launcher::Launcher;
pub use input_device::RawInputDevice;
pub use notification::WindowsNotificationService;
pub use output_device::SendInputDevice;
pub use platform_utils::WindowsPlatform;
pub use window_api::RealWindowApi;
pub use window_event_hook::WindowEventHook;
pub use window_manager::WindowManager;
pub use window_preset::WindowPresetManager;

impl TrayLifecycle for WindowsPlatform {
    crate::impl_tray_lifecycle!();
}

impl PlatformFactory for WindowsPlatform {
    type InputDevice = RawInputDevice;
    type OutputDevice = SendInputDevice;
    type WindowManager = WindowManager;
    type WindowPresetManager = WindowPresetManager;
    type NotificationService = WindowsNotificationService;
    type Launcher = Launcher;
    type WindowEventHook = WindowEventHook;

    crate::impl_platform_factory_methods!(
        Self,
        RawInputDevice,
        SendInputDevice,
        WindowManager,
        WindowPresetManager,
        WindowsNotificationService,
        Launcher,
        WindowEventHook
    );
}

impl LauncherTrait for Launcher {
    fn launch(&self, action: &crate::types::LaunchAction) -> Result<()> {
        self.launch(action)
    }
}

pub fn get_process_name_by_pid(pid: u32) -> anyhow::Result<String> {
    platform_utils::get_process_name_by_pid(pid)
}

pub fn get_executable_path_by_pid(pid: u32) -> anyhow::Result<String> {
    platform_utils::get_executable_path_by_pid(pid)
}
