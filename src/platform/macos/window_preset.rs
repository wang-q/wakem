//! macOS window preset management
#![cfg(target_os = "macos")]

use crate::platform::common::window_preset::{
    WindowPresetApi, WindowPresetManager as CommonWindowPresetManager,
};
use crate::platform::macos::window_manager::WindowManager;
use crate::platform::traits::{WindowId, WindowInfo};
use anyhow::Result;

impl WindowPresetApi for WindowManager {
    type WindowId = WindowId;

    fn get_foreground_window(&self) -> Option<Self::WindowId> {
        use crate::platform::traits::ForegroundWindowOperations;
        ForegroundWindowOperations::get_foreground_window(self)
    }

    fn get_window_info(&self, window: Self::WindowId) -> Result<WindowInfo> {
        use crate::platform::traits::WindowOperations;
        WindowOperations::get_window_info(self, window)
    }

    fn set_window_pos(
        &self,
        window: Self::WindowId,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
    ) -> Result<()> {
        use crate::platform::traits::WindowOperations;
        WindowOperations::set_window_pos(self, window, x, y, w, h)
    }

    fn minimize_window(&self, window: Self::WindowId) -> Result<()> {
        use crate::platform::traits::WindowOperations;
        WindowOperations::minimize_window(self, window)
    }

    fn maximize_window(&self, window: Self::WindowId) -> Result<()> {
        use crate::platform::traits::WindowOperations;
        WindowOperations::maximize_window(self, window)
    }
}

pub type WindowPresetManager = CommonWindowPresetManager<WindowManager>;

impl Default for WindowPresetManager {
    fn default() -> Self {
        Self::new(WindowManager::new())
    }
}
