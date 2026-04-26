//! Windows window preset implementation
#![cfg(target_os = "windows")]

use crate::platform::window_preset_common::WindowPresetManager as CommonWindowPresetManager;
use crate::platform::windows::window_manager::WindowManager;

pub type WindowPresetManager = CommonWindowPresetManager<
    WindowManager<crate::platform::windows::window_api::RealWindowApi>,
>;

impl Default for WindowPresetManager {
    fn default() -> Self {
        Self::new(WindowManager::new())
    }
}
