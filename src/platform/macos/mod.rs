//! macOS platform implementation
//!
//! This module provides macOS-specific implementations of the platform traits
//! using Core Graphics, Cocoa, and Accessibility APIs.
#![cfg(target_os = "macos")]

pub mod app_control;
pub mod context;
pub mod input;
pub mod input_device;
pub mod native_api;
pub mod notification;
pub mod output_device;
pub mod platform_utils;
pub mod tray;
pub mod window_api;
pub mod window_event_hook;
pub mod window_manager;
pub mod window_preset;

use crate::platform::traits::{
    LauncherTrait, PlatformFactory, TrayLifecycle, WindowEventHookTrait,
};

pub use crate::platform::common::launcher::Launcher;
pub use crate::platform::traits::InputDeviceConfig;

pub use input_device::MacosInputDeviceExt;
pub type InputDevice = MacosInputDeviceExt;

pub use notification::MacosNotificationService;
pub use output_device::MacosOutputDevice;

pub use platform_utils::MacosPlatform;

pub use tray::{
    run_tray_event_loop, run_tray_message_loop, stop_tray, AppCommand, RealTrayApi,
    TrayIconWrapper as TrayIcon, TrayManager,
};

pub use window_api::{MacosWindowApi, RealMacosWindowApi};
pub use window_event_hook::MacosWindowEventHook;

pub use window_manager::WindowManager;

pub use window_preset::WindowPresetManager;

#[cfg(test)]
pub use window_api::MockMacosWindowApi;

pub fn get_process_name_by_pid(pid: u32) -> anyhow::Result<String> {
    platform_utils::get_process_name_by_pid(pid)
}

pub fn get_executable_path_by_pid(pid: u32) -> anyhow::Result<String> {
    platform_utils::get_executable_path_by_pid(pid)
}

impl TrayLifecycle for MacosPlatform {
    crate::impl_tray_lifecycle!();
}

impl WindowEventHookTrait for MacosWindowEventHook {
    crate::impl_window_event_hook!();
}

impl LauncherTrait for Launcher {
    fn launch(&self, action: &crate::types::LaunchAction) -> anyhow::Result<()> {
        self.launch(action)
    }
}

impl PlatformFactory for MacosPlatform {
    type InputDevice = InputDevice;
    type OutputDevice = MacosOutputDevice;
    type WindowManager = WindowManager;
    type WindowPresetManager = WindowPresetManager;
    type NotificationService = MacosNotificationService;
    type Launcher = Launcher;
    type WindowEventHook = MacosWindowEventHook;

    crate::impl_platform_factory_methods!(
        Self,
        InputDevice,
        MacosOutputDevice,
        WindowManager,
        WindowPresetManager,
        MacosNotificationService,
        Launcher,
        MacosWindowEventHook
    );
}
