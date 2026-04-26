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

#[cfg(target_os = "windows")]
#[allow(unused_imports)]
pub use windows::{
    Launcher, MonitorDirection, RawInputDevice, RealWindowApi, SendInputDevice,
    TrayIcon, WindowEventHook, WindowManager, WindowPresetManager,
};

// Platform-specific type aliases for easier cross-platform code
#[cfg(target_os = "windows")]
pub mod platform_types {
    #![allow(unused_imports)]
    pub use super::windows::{
        Launcher, RawInputDevice as InputDevice, SendInputDevice as OutputDevice,
        WindowManager, WindowPresetManager,
    };
}

#[cfg(target_os = "macos")]
pub mod platform_types {
    #![allow(unused_imports)]
    pub use super::macos::{
        Launcher, RawInputDevice, SendInputDevice, WindowManager, WindowPresetManager,
    };
    // Type aliases for compatibility
    pub type InputDevice = super::macos::RawInputDevice;
    pub type OutputDevice = super::macos::SendInputDevice;
}
