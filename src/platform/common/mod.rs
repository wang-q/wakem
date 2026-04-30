//! Platform common modules
//!
//! This module contains cross-platform implementations that can be shared
//! between Windows and macOS platform layers.

pub mod input_device;
pub mod launcher;
pub mod output_helpers;
pub mod tray;
pub mod window_manager;
pub mod window_preset;

// Re-export commonly used types for public API
#[allow(unused_imports)]
pub use input_device::{InputDevice, InputDeviceBase, PlatformInputDevice};
#[allow(unused_imports)]
pub use launcher::Launcher;
#[allow(unused_imports)]
pub use output_helpers::char_to_vk;
#[allow(unused_imports)]
pub use tray::{
    menu_id_to_action, menu_ids, MockTrayApi, TrayApi, TrayIconWrapper, TrayManager,
};
#[allow(unused_imports)]
pub use window_manager::{CommonWindowApi, CommonWindowManager};
#[allow(unused_imports)]
pub use window_preset::{WindowPresetApi, WindowPresetManager};
