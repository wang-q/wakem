//! Windows window preset implementation
#![cfg(target_os = "windows")]

use crate::platform::traits::{WindowApiBase, WindowInfo};
use crate::platform::window_preset_common::{
    WindowPresetApi, WindowPresetManager as CommonWindowPresetManager,
};
use crate::platform::windows::window_manager::WindowManager;
use anyhow::Result;
use windows::Win32::Foundation::HWND;

impl<A: WindowApiBase<WindowId = HWND>> WindowPresetApi for WindowManager<A> {
    type WindowId = HWND;

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

pub type WindowPresetManager = CommonWindowPresetManager<
    WindowManager<crate::platform::windows::window_api::RealWindowApi>,
>;

impl Default for WindowPresetManager {
    fn default() -> Self {
        Self::new(WindowManager::new())
    }
}
