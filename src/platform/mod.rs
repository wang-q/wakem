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

// Backward compatibility modules - re-export from common
pub mod window_manager_common {
    pub use crate::platform::common::window_manager::*;
}

// Legacy re-exports (deprecated, use common::* instead)
#[deprecated(since = "0.2.0", note = "Use `common::input_device` instead")]
pub mod input_device_common {
    pub use crate::platform::common::input_device::*;
}

#[deprecated(since = "0.2.0", note = "Use `common::launcher` instead")]
pub mod launcher_common {
    pub use crate::platform::common::launcher::*;
}

#[deprecated(since = "0.2.0", note = "Use `common::output_helpers` instead")]
pub mod output_helpers {
    pub use crate::platform::common::output_helpers::*;
}

#[deprecated(since = "0.2.0", note = "Use `common::tray` instead")]
pub mod tray_common {
    pub use crate::platform::common::tray::*;
}

#[deprecated(since = "0.2.0", note = "Use `common::window_preset` instead")]
pub mod window_preset_common {
    pub use crate::platform::common::window_preset::*;
}

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
