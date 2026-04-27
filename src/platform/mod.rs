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

pub mod context;
pub mod input_device_common;
pub mod launcher_common;
pub mod mock;
pub mod output_helpers;
pub mod traits;
pub mod tray_common;
pub mod window_manager_common;
pub mod window_preset_common;

// Re-export PlatformContext for convenient access
#[allow(unused_imports)]
pub use context::PlatformContext;

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "windows")]
pub type CurrentPlatform = windows::WindowsPlatform;

#[cfg(target_os = "macos")]
pub type CurrentPlatform = macos::MacosPlatform;

#[cfg(target_os = "linux")]
pub type CurrentPlatform = linux::LinuxPlatform;

// Platform-agnostic type aliases using associated types
// These hide platform-specific types behind trait abstractions
// Note: These aliases are provided for future use when code needs to reference
// platform-specific types directly (e.g., for type annotations or struct fields)

/// Platform-agnostic input device type
#[allow(dead_code)]
pub type InputDevice = <CurrentPlatform as traits::PlatformFactory>::InputDevice;

/// Platform-agnostic output device type
#[allow(dead_code)]
pub type OutputDevice = <CurrentPlatform as traits::PlatformFactory>::OutputDevice;

/// Platform-agnostic window manager type
#[allow(dead_code)]
pub type WindowManager = <CurrentPlatform as traits::PlatformFactory>::WindowManager;

/// Platform-agnostic window preset manager type
#[allow(dead_code)]
pub type WindowPresetManager =
    <CurrentPlatform as traits::PlatformFactory>::WindowPresetManager;

/// Platform-agnostic notification service type
#[allow(dead_code)]
pub type NotificationService =
    <CurrentPlatform as traits::PlatformFactory>::NotificationService;

/// Platform-agnostic launcher type
#[allow(dead_code)]
pub type Launcher = <CurrentPlatform as traits::PlatformFactory>::Launcher;

/// Platform-agnostic window event hook type
#[allow(dead_code)]
pub type WindowEventHook = <CurrentPlatform as traits::PlatformFactory>::WindowEventHook;
