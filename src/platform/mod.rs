//! Platform abstraction layer
//!
//! This module provides cross-platform abstractions for:
//! - Input device capture (keyboard/mouse)
//! - Output device simulation (sending input events)
//! - Window management
//! - System tray integration
//!
//! The module uses conditional compilation to select the appropriate
//! platform-specific implementation.

pub mod mock;
pub mod output_helpers;
pub mod traits;

// Re-export mock implementations for testing
pub use mock::MockInputDevice;
pub use output_helpers::char_to_vk;

// Re-export specific items from traits
pub use traits::{
    InputDeviceTrait, MonitorInfo, OutputDeviceTrait, TrayIconTrait, WindowApiTrait,
    WindowContext, WindowId, WindowInfo, WindowManagerTrait,
};

// Platform-specific implementations
#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "macos")]
pub mod macos;

// Re-export platform-specific types (only on respective platforms)
#[cfg(target_os = "windows")]
pub use windows::{
    // Launcher
    Launcher,
    MonitorDirection,
    // Legacy exports for backward compatibility
    MonitorInfo as WindowsMonitorInfo,
    // Input/Output
    RawInputDevice,
    RealWindowApi,
    SendInputDevice,
    // Tray
    TrayIcon,
    // Context
    WindowContext as WindowsWindowContext,
    WindowEvent,
    WindowEventHook,
    WindowFrame,
    // Window management
    WindowManager,
    WindowPresetManager,
};

// Platform-specific type aliases
#[cfg(target_os = "windows")]
pub type PlatformInputDevice = windows::RawInputDevice;

#[cfg(target_os = "windows")]
pub type PlatformOutputDevice = windows::SendInputDevice;

#[cfg(target_os = "windows")]
pub type PlatformWindowManager = windows::WindowManager<windows::RealWindowApi>;

#[cfg(target_os = "windows")]
pub type PlatformTrayIcon = windows::TrayIcon;

#[cfg(all(target_os = "macos", not(test)))]
pub type PlatformInputDevice = macos::MacosInputDevice;

#[cfg(all(target_os = "macos", test))]
pub type PlatformInputDevice = macos::input_device::MockInputDevice;

#[cfg(all(target_os = "macos", not(test)))]
pub type PlatformOutputDevice = macos::MacosOutputDevice;

#[cfg(all(target_os = "macos", test))]
pub type PlatformOutputDevice = macos::output_device::MockMacosOutputDevice;

#[cfg(all(target_os = "macos", not(test)))]
pub type PlatformWindowManager = macos::MacosWindowManager<macos::RealMacosWindowApi>;

#[cfg(all(target_os = "macos", test))]
pub type PlatformWindowManager = macos::MacosWindowManager<macos::MockMacosWindowApi>;

#[cfg(all(target_os = "macos", not(test)))]
pub type PlatformTrayIcon = macos::TrayIcon<macos::RealTrayApi>;

#[cfg(all(target_os = "macos", test))]
pub type PlatformTrayIcon = macos::TrayIcon<macos::MockTrayApi>;

/// Get the current platform name
pub fn platform_name() -> &'static str {
    #[cfg(target_os = "windows")]
    return "windows";

    #[cfg(target_os = "macos")]
    return "macos";

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    return "unknown";
}

/// Check if running on Windows
pub const IS_WINDOWS: bool = cfg!(target_os = "windows");

/// Check if running on macOS
pub const IS_MACOS: bool = cfg!(target_os = "macos");
