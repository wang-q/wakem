//! Linux window manager and preset manager (placeholder)

use crate::platform::traits::{
    ForegroundWindowOperations, MonitorInfo, MonitorOperations, WindowId, WindowInfo,
    WindowManagerTrait, WindowOperations, WindowPresetManagerTrait,
    WindowStateQueries,
};
use anyhow::Result;

// ---------------------------------------------------------------------------
// Window Manager
// ---------------------------------------------------------------------------

pub struct LinuxWindowManager;

impl LinuxWindowManager {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LinuxWindowManager {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowManagerTrait for LinuxWindowManager {}

impl WindowOperations for LinuxWindowManager {
    fn get_window_info(&self, _window: WindowId) -> Result<WindowInfo> {
        Err(anyhow::anyhow!(
            "Linux window manager not yet implemented. Wayland toplevel management required."
        ))
    }

    fn set_window_pos(
        &self,
        _window: WindowId,
        _x: i32,
        _y: i32,
        _width: i32,
        _height: i32,
    ) -> Result<()> {
        Err(anyhow::anyhow!(
            "Linux window manager not yet implemented. Wayland toplevel management required."
        ))
    }

    fn minimize_window(&self, _window: WindowId) -> Result<()> {
        Err(anyhow::anyhow!(
            "Linux window manager not yet implemented. Wayland toplevel management required."
        ))
    }

    fn maximize_window(&self, _window: WindowId) -> Result<()> {
        Err(anyhow::anyhow!(
            "Linux window manager not yet implemented. Wayland toplevel management required."
        ))
    }

    fn restore_window(&self, _window: WindowId) -> Result<()> {
        Err(anyhow::anyhow!(
            "Linux window manager not yet implemented. Wayland toplevel management required."
        ))
    }

    fn close_window(&self, _window: WindowId) -> Result<()> {
        Err(anyhow::anyhow!(
            "Linux window manager not yet implemented. Wayland toplevel management required."
        ))
    }
}

impl WindowStateQueries for LinuxWindowManager {
    fn is_window_valid(&self, _window: WindowId) -> bool {
        false
    }

    fn is_minimized(&self, _window: WindowId) -> bool {
        false
    }

    fn is_maximized(&self, _window: WindowId) -> bool {
        false
    }

    fn is_topmost(&self, _window: WindowId) -> bool {
        false
    }
}

impl MonitorOperations for LinuxWindowManager {
    fn get_monitors(&self) -> Vec<MonitorInfo> {
        Vec::new()
    }

    fn move_to_monitor(
        &self,
        _window: WindowId,
        _monitor_index: usize,
    ) -> Result<()> {
        Err(anyhow::anyhow!(
            "Linux window manager not yet implemented. Wayland output management required."
        ))
    }
}

impl ForegroundWindowOperations for LinuxWindowManager {
    fn get_foreground_window(&self) -> Option<WindowId> {
        None
    }

    fn set_topmost(&self, _window: WindowId, _topmost: bool) -> Result<()> {
        Err(anyhow::anyhow!(
            "Linux window manager not yet implemented."
        ))
    }
}

// ---------------------------------------------------------------------------
// Window Preset Manager
// ---------------------------------------------------------------------------

pub struct LinuxWindowPresetManager;

impl LinuxWindowPresetManager {
    pub fn new(_wm: LinuxWindowManager) -> Self {
        Self
    }
}

impl WindowPresetManagerTrait for LinuxWindowPresetManager {
    fn load_presets(&mut self, _presets: Vec<crate::config::WindowPreset>) {}

    fn save_preset(&mut self, _name: String) -> Result<()> {
        Err(anyhow::anyhow!(
            "Linux window preset manager not yet implemented"
        ))
    }

    fn load_preset(&self, _name: &str) -> Result<()> {
        Err(anyhow::anyhow!(
            "Linux window preset manager not yet implemented"
        ))
    }

    fn get_foreground_window_info(&self) -> Option<Result<WindowInfo>> {
        None
    }

    fn apply_preset_for_window_by_id(&self, _window_id: WindowId) -> Result<bool> {
        Ok(false)
    }
}
