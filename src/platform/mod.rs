//! Platform abstraction layer
//!
//! This module provides a three-layer architecture for cross-platform support:
//!
//! 1. **Traits** (`traits.rs`): Platform-agnostic interfaces
//! 2. **Common** (`common/`): Cross-platform implementations shared between platforms
//! 3. **Platform-specific** (`windows/`, `macos/`): Platform-specific implementations
//!
//! # Module Organization
//!
//! - `traits.rs` - Core trait definitions (WindowApi, InputDevice, etc.)
//! - `types.rs` - Shared platform types
//! - `macros.rs` - Shared macros
//! - `mock.rs` - Mock implementations for testing
//! - `common/` - Cross-platform implementations
//!   - `input_device.rs`
//!   - `launcher.rs`
//!   - `output_helpers.rs`
//!   - `tray.rs`
//!   - `window_preset.rs`
//! - `windows/` - Windows-specific implementations
//! - `macos/` - macOS-specific implementations

// Core modules
pub mod common;
pub mod macros;
pub mod mock;
pub mod traits;
pub mod types;

// Platform-specific modules
#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "macos")]
pub mod macos;

// Re-export common types for convenience
#[allow(unused_imports)]
pub use common::{
    input_device::{InputDevice, InputDeviceBase, PlatformInputDevice},
    launcher::Launcher,
    output_helpers::char_to_vk,
    tray::{
        menu_id_to_action, menu_ids, MockTrayApi, TrayApi, TrayIconWrapper, TrayManager,
    },
    window_preset::{WindowPresetApi, WindowPresetManager},
};

// Backward compatibility: re-export from common modules
#[allow(unused_imports)]
pub use common::input_device;
#[allow(unused_imports)]
pub use common::launcher;
#[allow(unused_imports)]
pub use common::tray;
#[allow(unused_imports)]
pub use common::window_preset;

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
        InputDevice, InputDeviceConfig, Launcher, MacosOutputDevice as OutputDevice,
        WindowManager, WindowPresetManager,
    };
}

/// Current platform's factory type
///
/// Use `CurrentPlatform::create_*()` to create platform-specific objects
/// without conditional compilation in business logic.
#[cfg(target_os = "windows")]
pub type CurrentPlatform = windows::WindowsPlatform;

#[cfg(target_os = "macos")]
pub type CurrentPlatform = macos::MacosPlatform;
