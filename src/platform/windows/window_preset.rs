//! Windows window preset implementation
#![cfg(target_os = "windows")]

use crate::platform::common::window_preset::{
    WindowPresetApi, WindowPresetManager as CommonWindowPresetManager,
};
use crate::platform::traits::{
    ForegroundWindowOperations, WindowOperations,
    WindowPresetManager as WindowPresetManagerTrait,
};
use crate::platform::types::{WindowId, WindowInfo};
use crate::platform::windows::window_manager::WindowManager;
use anyhow::Result;

impl WindowPresetApi for WindowManager {
    type WindowId = WindowId;

    fn get_foreground_window(&self) -> Option<Self::WindowId> {
        ForegroundWindowOperations::get_foreground_window(self)
    }

    fn get_window_info(&self, window: Self::WindowId) -> Result<WindowInfo> {
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
        WindowOperations::set_window_pos(self, window, x, y, w, h)
    }

    fn minimize_window(&self, window: Self::WindowId) -> Result<()> {
        WindowOperations::minimize_window(self, window)
    }

    fn maximize_window(&self, window: Self::WindowId) -> Result<()> {
        WindowOperations::maximize_window(self, window)
    }
}

pub type WindowPresetManager = CommonWindowPresetManager<WindowManager>;

impl Default for WindowPresetManager {
    fn default() -> Self {
        Self::new(WindowManager::new())
    }
}

impl WindowPresetManagerTrait for WindowPresetManager {
    fn load_presets(&mut self, presets: Vec<crate::config::WindowPreset>) {
        self.load_presets(presets)
    }

    fn save_preset(&mut self, name: String) -> Result<()> {
        self.save_preset(name)
    }

    fn load_preset(&self, name: &str) -> Result<()> {
        self.load_preset(name)
    }

    fn get_foreground_window_info(&self) -> Option<Result<WindowInfo>> {
        self.get_foreground_window_info()
    }

    fn apply_preset_for_window_by_id(&self, window_id: WindowId) -> Result<bool> {
        self.apply_preset_for_window_by_id(window_id)
    }
}
