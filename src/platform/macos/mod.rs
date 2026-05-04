//! macOS platform implementation
//!
//! This module provides macOS-specific implementations of the platform traits
//! using Core Graphics, Cocoa, and Accessibility APIs.
//!
//! # Implementation Status
//!
//! | Component | Status | Notes |
//! |-----------|--------|-------|
//! | InputDevice | ✅ | CGEvent tap for keyboard/mouse capture |
//! | OutputDevice | ✅ | CGEventPost for event injection |
//! | WindowManager | ✅ | Core Graphics window operations |
//! | WindowPresetManager | ✅ | Preset save/load/apply |
//! | NotificationService | ✅ | UserNotifications framework |
//! | Launcher | ✅ | NSWorkspace for app launching |
//! | WindowEventHook | ⚠️ | Basic implementation, needs testing |
//! | Tray | ⚠️ | Menu bar icon, needs polish |
//! | AppControl | ✅ | NSWorkspace, NSApplication APIs |
//!
//! # Known Limitations
//!
//! - Some window operations require Accessibility permissions
//! - Window event hook may miss rapid window switches
//! - Tray menu styling differs from native macOS apps
//!
//! # Required Permissions
//!
//! - Accessibility: For window management and input capture
//! - Input Monitoring: For keyboard/mouse event taps
//! - Automation: For window event notifications
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

use crate::platform::traits::{PlatformFactory, TrayLifecycle, WindowEventHook};

pub use crate::platform::common::launcher::Launcher;
pub use input_device::MacosInputDeviceExt;
pub type InputDevice = MacosInputDeviceExt;
pub use notification::MacosNotificationService;
pub use output_device::MacosOutputDevice;
pub use platform_utils::MacosPlatform;
pub use window_api::RealMacosWindowApi;
pub use window_event_hook::MacosWindowEventHook;
pub use window_manager::WindowManager;
pub use window_preset::WindowPresetManager;

crate::impl_platform_utils_delegates!();

impl TrayLifecycle for MacosPlatform {
    crate::impl_tray_lifecycle!();
}

impl WindowEventHook for MacosWindowEventHook {
    crate::impl_window_event_hook!();
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
