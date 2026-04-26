//! macOS window preset management

use crate::platform::macos::window_api::RealWindowApi;
use crate::platform::macos::window_manager::WindowManager;
use crate::platform::window_preset_common::WindowPresetManager as CommonWindowPresetManager;

pub type WindowPresetManager = CommonWindowPresetManager<WindowManager<RealWindowApi>>;

impl Default for WindowPresetManager {
    fn default() -> Self {
        Self::new(WindowManager::new())
    }
}
