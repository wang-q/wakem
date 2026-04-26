//! macOS window preset management

use crate::platform::macos::window_api::RealWindowApi;
use crate::platform::macos::window_manager::WindowManager;
use crate::platform::traits::{WindowApiBase, WindowId, WindowInfo};
use crate::platform::window_preset_common::{
    WindowPresetApi, WindowPresetManager as CommonWindowPresetManager,
};
use anyhow::Result;

impl<A: WindowApiBase<WindowId = WindowId>> WindowPresetApi for WindowManager<A> {
    type WindowId = WindowId;

    fn get_foreground_window(&self) -> Option<Self::WindowId> {
        self.api().get_foreground_window()
    }

    fn get_window_info(&self, window: Self::WindowId) -> Result<WindowInfo> {
        self.api().get_window_info(window)
    }

    fn set_window_pos(
        &self,
        window: Self::WindowId,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
    ) -> Result<()> {
        let frame = crate::platform::traits::WindowFrame::new(x, y, w, h);
        self.set_window_frame(window, &frame)
    }

    fn minimize_window(&self, window: Self::WindowId) -> Result<()> {
        self.api().minimize_window(window)
    }

    fn maximize_window(&self, window: Self::WindowId) -> Result<()> {
        self.api().maximize_window(window)
    }
}

/// Window preset manager type alias for macOS
pub type WindowPresetManager = CommonWindowPresetManager<WindowManager<RealWindowApi>>;

impl Default for WindowPresetManager {
    fn default() -> Self {
        Self::new(WindowManager::new())
    }
}
