//! macOS platform implementation
//!
//! This module provides macOS-specific implementations of the platform traits
//! using Core Graphics, Cocoa, and Accessibility APIs.
#![cfg(target_os = "macos")]

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

use crate::platform::traits::{
    ApplicationControl, ContextProvider, LauncherTrait, NotificationService,
    PlatformFactory, PlatformUtilities, TrayLifecycle, WindowEventHookTrait,
    WindowPresetManagerTrait,
};

pub use crate::platform::common::launcher::Launcher;
pub use crate::platform::traits::InputDeviceConfig;

pub use input_device::MacosInputDeviceExt;
pub type InputDevice = MacosInputDeviceExt;

pub use output_device::MacosOutputDevice;

pub use tray::{
    run_tray_event_loop, run_tray_message_loop, stop_tray, AppCommand, RealTrayApi,
    TrayIconWrapper as TrayIcon, TrayManager,
};

pub use window_api::{MacosWindowApi, RealMacosWindowApi};
pub use window_event_hook::MacosWindowEventHook;

pub use window_manager::{MacosWindowManager, RealMacosWindowManager as WindowManager};

pub use window_preset::WindowPresetManager;

#[cfg(test)]
pub use window_api::MockMacosWindowApi;

/// Get current modifier state for macOS using CGEventSource
pub fn get_modifier_state() -> crate::types::ModifierState {
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    let mut modifiers = crate::types::ModifierState::default();

    if let Ok(source) = CGEventSource::new(CGEventSourceStateID::HIDSystemState) {
        if let Ok(event) = core_graphics::event::CGEvent::new(source) {
            let flags = event.get_flags();

            if flags.contains(core_graphics::event::CGEventFlags::CGEventFlagShift) {
                modifiers.shift = true;
            }
            if flags.contains(core_graphics::event::CGEventFlags::CGEventFlagControl) {
                modifiers.ctrl = true;
            }
            if flags.contains(core_graphics::event::CGEventFlags::CGEventFlagAlternate) {
                modifiers.alt = true;
            }
            if flags.contains(core_graphics::event::CGEventFlags::CGEventFlagCommand) {
                modifiers.meta = true;
            }
        }
    }

    modifiers
}

pub fn get_process_name_by_pid(pid: u32) -> anyhow::Result<String> {
    ns_workspace::get_process_name_by_pid(pid)
}

pub fn get_executable_path_by_pid(pid: u32) -> anyhow::Result<String> {
    ns_workspace::get_executable_path_by_pid(pid)
}

// ---- Trait implementations ----

/// Zero-sized type for implementing class-method traits on macOS
pub struct MacosPlatform;

impl PlatformUtilities for MacosPlatform {
    fn get_modifier_state() -> crate::types::ModifierState {
        get_modifier_state()
    }
}

impl ContextProvider for MacosPlatform {
    crate::impl_context_provider!();
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
        self.apply_preset_for_window_by_id(window_id)
    }
}

crate::decl_notification_service!(MacosNotificationService);

impl NotificationService for MacosNotificationService {
    fn show(&self, title: &str, message: &str) -> anyhow::Result<()> {
        crate::platform::macos::native_api::notification::show_notification(
            title, message,
        )
    }
}

impl ApplicationControl for MacosPlatform {
    fn detach_console() {}

    fn terminate_application() {
        <Self as TrayLifecycle>::stop_tray()
    }

    fn open_folder(path: &std::path::Path) -> anyhow::Result<()> {
        std::process::Command::new("open").arg(path).spawn()?;
        Ok(())
    }

    fn force_kill_instance(instance_id: u32) -> anyhow::Result<()> {
        use std::process::{Command, Stdio};

        let process_name = if instance_id == 0 {
            "wakemd".to_string()
        } else {
            format!("wakemd-instance{}", instance_id)
        };

        let output = Command::new("pkill")
            .args(["-f", &process_name])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output();

        match output {
            Ok(result) if result.status.success() => {
                tracing::info!("Successfully killed daemon instance {}", instance_id);
                Ok(())
            }
            _ => {
                anyhow::bail!("Failed to kill daemon instance {}", instance_id)
            }
        }
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
