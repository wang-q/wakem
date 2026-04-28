//! macOS platform implementation
//!
//! This module provides the macOS-specific implementations of all platform traits.

pub mod context;
pub mod input;
pub mod input_device;
pub mod native_api;
pub mod output_device;
pub mod tray;
pub mod window_api;
pub mod window_manager;

// Re-export common types (aligned with Windows platform)
// These are public API for users who need platform-specific types
#[allow(unused_imports)]
pub use crate::platform::launcher_common::Launcher;
#[allow(unused_imports)]
pub use input_device::RawInputDevice;
#[allow(unused_imports)]
pub use output_device::SendInputDevice;
#[allow(unused_imports)]
pub use window_api::{RealWindowApi, WindowEventHook};

#[cfg(test)]
#[allow(unused_imports)]
pub use window_api::MockWindowApi;

#[allow(unused_imports)]
pub use window_manager::{MonitorDirection, WindowManager, WindowPresetManager};

use crate::platform::traits::{
    ApplicationControl, ContextProvider, PlatformFactory, PlatformUtilities,
    TrayLifecycle, WindowEventHookTrait,
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
        use crate::platform::macos::native_api::ns_workspace;
        ns_workspace::get_app_path(pid)
            .and_then(|path| {
                std::path::Path::new(&path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.to_string())
            })
            .ok_or_else(|| anyhow::anyhow!("Failed to get process name for pid {}", pid))
    }

    fn get_executable_path_by_pid(pid: u32) -> anyhow::Result<String> {
        use crate::platform::macos::native_api::ns_workspace;
        ns_workspace::get_app_path(pid).ok_or_else(|| {
            anyhow::anyhow!("Failed to get executable path for pid {}", pid)
        })
    }
}

impl ContextProvider for MacosPlatform {
    crate::impl_context_provider!();
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
    fn start_with_shutdown_inner(&mut self, sf: std::sync::Arc<std::sync::atomic::AtomicBool>) -> Result<()> { self.start_with_shutdown_inner(sf) }
    fn stop_inner(&mut self) { self.stop_inner() }
    fn shutdown_flag_inner(&self) -> std::sync::Arc<std::sync::atomic::AtomicBool> { self.shutdown_flag_inner() }
}

impl TrayLifecycle for MacosPlatform {
    crate::impl_tray_lifecycle!();
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
    type InputDevice = RawInputDevice;
    type OutputDevice = SendInputDevice;
    type WindowManager = WindowManager<window_api::RealWindowApi>;
    type WindowPresetManager = WindowPresetManager;
    type NotificationService = MacosNotificationService;
    type Launcher = Launcher;
    type WindowEventHook = WindowEventHook;

    crate::impl_platform_factory_methods!(
        Self,
        RawInputDevice,
        SendInputDevice,
        WindowManager<window_api::RealWindowApi>,
        WindowPresetManager,
        MacosNotificationService,
        Launcher,
        WindowEventHook
    );
}
