//! macOS window preset management

use crate::platform::macos::window_api::{MacosWindowApi, RealMacosWindowApi};
use crate::platform::traits::WindowInfo;
use crate::platform::window_preset_common::{WindowPresetApi, WindowPresetManager};
use anyhow::Result;

impl<A: MacosWindowApi + Clone> WindowPresetApi
    for crate::platform::macos::window_manager::MacosWindowManager<A>
{
    type WindowId = crate::platform::traits::WindowId;

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
        self.api().set_window_pos(window, x, y, w, h)
    }

    fn minimize_window(&self, window: Self::WindowId) -> Result<()> {
        self.api().minimize_window(window)
    }

    fn maximize_window(&self, window: Self::WindowId) -> Result<()> {
        self.api().maximize_window(window)
    }
}

pub type MacosWindowPresetManager = WindowPresetManager<
    crate::platform::macos::window_manager::MacosWindowManager<RealMacosWindowApi>,
>;

impl Default for MacosWindowPresetManager {
    fn default() -> Self {
        let api = crate::platform::macos::window_manager::MacosWindowManager::<
            RealMacosWindowApi,
        >::new_real();
        Self::new(api)
    }
}
