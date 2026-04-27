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

pub mod input_device_common;
pub mod launcher_common;
#[cfg(test)]
pub mod mock;
pub mod output_helpers;
pub mod traits;
pub mod tray_common;
pub mod window_manager_common;
pub mod window_preset_common;

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
