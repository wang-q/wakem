//! Windows window preset implementation
#![cfg(target_os = "windows")]

use crate::platform::common::window_preset::WindowPresetManager as CommonWindowPresetManager;
use crate::platform::windows::window_manager::WindowManager;

pub type WindowPresetManager = CommonWindowPresetManager<WindowManager>;

impl Default for WindowPresetManager {
    fn default() -> Self {
        Self::new(WindowManager::new())
    }
}
