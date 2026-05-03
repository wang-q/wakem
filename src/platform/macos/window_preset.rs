//! macOS window preset management
#![cfg(target_os = "macos")]

use crate::platform::common::window_preset::WindowPresetManager as CommonWindowPresetManager;
use crate::platform::macos::window_manager::WindowManager;

pub type WindowPresetManager = CommonWindowPresetManager<WindowManager>;

impl Default for WindowPresetManager {
    fn default() -> Self {
        Self::new(WindowManager::new())
    }
}
