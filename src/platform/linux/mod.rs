//! Linux platform implementation (placeholder)
//!
//! This module provides placeholder implementations for Linux platform support.
//! It uses the Wayland protocol for window management and input handling.
//!
//! ## Implementation Status
//!
//! This is a work-in-progress placeholder. The actual implementation will require:
//! - Wayland client library integration (wayland-client crate)
//! - EVDEV input event handling for keyboard/mouse capture
//! - XDG Desktop Portal integration for window management
//! - DBus for system tray and notifications
//!
//! ## Architecture Notes
//!
//! Linux implementation differs from Windows/macOS in several ways:
//! - Wayland is the primary target (X11 support may be added later)
//! - Input capture requires special permissions (evdev access)
//! - Window management is more decentralized (compositor-dependent)
//! - System integration uses DBus/Portals instead of native APIs
//!
//! The module structure mirrors Windows and macOS for consistency.

pub mod context;
pub mod input_device;
pub mod output_device;
pub mod tray;
pub mod window_api;
pub mod window_manager;

pub use crate::platform::launcher_common::Launcher;
pub use input_device::LinuxInputDevice;
pub use output_device::LinuxOutputDevice;
pub use window_api::LinuxWindowEventHook;
pub use window_manager::{LinuxWindowManager, LinuxWindowPresetManager};

use crate::platform::traits::{
    ApplicationControl, ContextProvider, LauncherTrait, NotificationService,
    PlatformFactory, PlatformUtilities, TrayLifecycle,
};
use crate::types::ModifierState;
use anyhow::Result;

/// Linux platform type marker.
///
/// Implements all platform-level traits, serving as the single entry point
/// for platform-specific functionality — same pattern as [`WindowsPlatform`]
/// and [`MacosPlatform`].
pub struct LinuxPlatform;

// ---------------------------------------------------------------------------
// PlatformUtilities
// ---------------------------------------------------------------------------

impl PlatformUtilities for LinuxPlatform {
    fn get_modifier_state() -> ModifierState {
        ModifierState::default()
    }

    fn get_process_name_by_pid(_pid: u32) -> anyhow::Result<String> {
        Err(anyhow::anyhow!(
            "Linux process name lookup not yet implemented"
        ))
    }

    fn get_executable_path_by_pid(_pid: u32) -> anyhow::Result<String> {
        Err(anyhow::anyhow!(
            "Linux executable path lookup not yet implemented"
        ))
    }
}

// ---------------------------------------------------------------------------
// ContextProvider
// ---------------------------------------------------------------------------

impl ContextProvider for LinuxPlatform {
    crate::impl_context_provider!();
}

// ---------------------------------------------------------------------------
// NotificationService
// ---------------------------------------------------------------------------

/// Placeholder notification service for Linux
crate::decl_notification_service!(LinuxNotificationService);

impl NotificationService for LinuxNotificationService {
    fn show(&self, _title: &str, _message: &str) -> Result<()> {
        Err(anyhow::anyhow!(
            "Linux notification service not yet implemented. DBus/Portal required."
        ))
    }
}

// ---------------------------------------------------------------------------
// Launcher
// ---------------------------------------------------------------------------

/// Placeholder launcher for Linux
pub struct LinuxLauncher;

impl LinuxLauncher {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LinuxLauncher {
    fn default() -> Self {
        Self::new()
    }
}

impl LauncherTrait for LinuxLauncher {
    fn launch(&self, _action: &crate::types::LaunchAction) -> Result<()> {
        Err(anyhow::anyhow!(
            "Linux launcher not yet implemented. XDG Desktop Portal required."
        ))
    }
}

// ---------------------------------------------------------------------------
// WindowEventHookTrait
// ---------------------------------------------------------------------------

impl crate::platform::traits::WindowEventHookTrait for LinuxWindowEventHook {
    crate::impl_window_event_hook!();
}

// ---------------------------------------------------------------------------
// TrayLifecycle
// ---------------------------------------------------------------------------

impl TrayLifecycle for LinuxPlatform {
    crate::impl_tray_lifecycle!();
}

// ---------------------------------------------------------------------------
// ApplicationControl
// ---------------------------------------------------------------------------

impl ApplicationControl for LinuxPlatform {
    fn detach_console() {
        // No-op on Linux (no console to detach)
    }

    fn terminate_application() {
        std::process::exit(0);
    }

    fn open_folder(_path: &std::path::Path) -> Result<()> {
        Err(anyhow::anyhow!(
            "Linux folder open not yet implemented. xdg-open required."
        ))
    }

    fn force_kill_instance(_instance_id: u32) -> Result<()> {
        Err(anyhow::anyhow!("Linux force kill not yet implemented"))
    }
}

// ---------------------------------------------------------------------------
// PlatformFactory
// ---------------------------------------------------------------------------

impl PlatformFactory for LinuxPlatform {
    type InputDevice = LinuxInputDevice;
    type OutputDevice = LinuxOutputDevice;
    type WindowManager = LinuxWindowManager;
    type WindowPresetManager = LinuxWindowPresetManager;
    type NotificationService = LinuxNotificationService;
    type Launcher = LinuxLauncher;
    type WindowEventHook = LinuxWindowEventHook;

    crate::impl_platform_factory_methods!(
        Self,
        LinuxInputDevice,
        LinuxOutputDevice,
        LinuxWindowManager,
        LinuxWindowPresetManager,
        LinuxNotificationService,
        LinuxLauncher,
        LinuxWindowEventHook
    );
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linux_platform_factory() {
        let _: &dyn PlatformFactory = &LinuxPlatform;
    }

    #[test]
    fn test_linux_window_manager_trait_impl() {
        let wm = LinuxWindowManager::new();
        let _: &dyn crate::platform::traits::WindowManagerTrait = &wm;
    }

    #[test]
    fn test_placeholder_error_messages() {
        let wm = LinuxWindowManager::new();
        let result = wm.get_window_info(0);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Linux"));
        assert!(err_msg.contains("not yet implemented"));
    }
}
