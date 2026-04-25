//! macOS window preset management
//!
//! Provides window preset functionality for saving, loading, and automatically
//! applying window layouts based on configuration rules.
//!
//! The core logic lives in [crate::platform::window_preset_common::WindowPresetManager];
//! this module adds the macOS-specific [WindowPresetApi] implementation
//! built on top of [MacosWindowApi].
#![cfg(target_os = "macos")]

use crate::platform::macos::window_api::{MacosWindowApi, RealMacosWindowApi};
use crate::platform::traits::WindowInfo;
use crate::platform::window_preset_common::{WindowPresetApi, WindowPresetManager};
use anyhow::Result;

impl<A: MacosWindowApi> WindowPresetApi
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

/// macOS window preset manager (type alias for the common manager)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::WindowPreset;
    use crate::platform::macos::window_api::MockMacosWindowApi;
    use crate::platform::macos::window_manager::MacosWindowManager;

    fn create_test_manager(
    ) -> WindowPresetManager<MacosWindowManager<MockMacosWindowApi>> {
        let mock = MockMacosWindowApi::new();
        let wm = MacosWindowManager::new(mock);
        WindowPresetManager::new(wm)
    }

    #[test]
    fn test_preset_manager_creation() {
        let mgr = create_test_manager();
        assert_eq!(mgr.preset_count(), 0);
        assert_eq!(mgr.saved_count(), 0);
    }

    #[test]
    fn test_load_presets() {
        let mut mgr = create_test_manager();
        mgr.load_presets(vec![
            WindowPreset {
                name: "Editor Left".to_string(),
                process_name: Some("TestApp".to_string()),
                executable_path: None,
                title_pattern: None,
                x: 0,
                y: 0,
                width: 960,
                height: 1080,
            },
            WindowPreset {
                name: "Browser Right".to_string(),
                process_name: Some("*Chrome*".to_string()),
                executable_path: None,
                title_pattern: Some("*GitHub*".to_string()),
                x: 960,
                y: 0,
                width: 960,
                height: 1080,
            },
        ]);
        assert_eq!(mgr.preset_count(), 2);
        assert_eq!(mgr.get_presets()[0].name, "Editor Left");
        assert_eq!(mgr.get_presets()[1].name, "Browser Right");
    }

    #[test]
    fn test_save_and_load_preset() {
        let mut mgr = create_test_manager();
        mgr.save_preset("My Layout".to_string()).unwrap();
        assert_eq!(mgr.saved_count(), 1);
        mgr.load_preset("My Layout").unwrap();
    }

    #[test]
    fn test_apply_preset_for_window() {
        let mut mgr = create_test_manager();
        mgr.load_presets(vec![WindowPreset {
            name: "TestApp Preset".to_string(),
            process_name: Some("TestApp".to_string()),
            executable_path: None,
            title_pattern: None,
            x: 100,
            y: 200,
            width: 1024,
            height: 768,
        }]);
        let result = mgr.apply_preset_for_window().unwrap();
        assert!(result);
    }

    #[test]
    fn test_no_matching_preset() {
        let mut mgr = create_test_manager();
        mgr.load_presets(vec![WindowPreset {
            name: "Firefox Only".to_string(),
            process_name: Some("Firefox".to_string()),
            executable_path: None,
            title_pattern: None,
            x: 0,
            y: 0,
            width: 800,
            height: 600,
        }]);
        let result = mgr.apply_preset_for_window().unwrap();
        assert!(!result);
    }

    #[test]
    fn test_wildcard_matching() {
        let mut mgr = create_test_manager();
        mgr.load_presets(vec![WindowPreset {
            name: "All Apps".to_string(),
            process_name: Some("*".to_string()),
            executable_path: None,
            title_pattern: None,
            x: 50,
            y: 50,
            width: 800,
            height: 600,
        }]);
        let result = mgr.apply_preset_for_window().unwrap();
        assert!(result);
    }

    #[test]
    fn test_remove_saved_preset() {
        let mut mgr = create_test_manager();
        mgr.save_preset("Layout1".to_string()).unwrap();
        mgr.save_preset("Layout2".to_string()).unwrap();
        assert_eq!(mgr.saved_count(), 2);
        assert!(mgr.remove_saved_preset("Layout1"));
        assert_eq!(mgr.saved_count(), 1);
        assert!(!mgr.remove_saved_preset("NonExistent"));
    }

    #[test]
    fn test_clear_presets() {
        let mut mgr = create_test_manager();
        mgr.load_presets(vec![WindowPreset {
            name: "Test".to_string(),
            process_name: Some("*".to_string()),
            executable_path: None,
            title_pattern: None,
            x: 0,
            y: 0,
            width: 800,
            height: 600,
        }]);
        assert_eq!(mgr.preset_count(), 1);
        mgr.clear_presets();
        assert_eq!(mgr.preset_count(), 0);
    }

    #[test]
    fn test_get_foreground_window_info() {
        let mgr = create_test_manager();
        let info_result = mgr.get_foreground_window_info();
        assert!(info_result.is_some());
        let info = info_result.unwrap().unwrap();
        assert_eq!(info.process_name, "TestApp");
    }

    #[test]
    fn test_default_manager() {
        let _mgr: MacosWindowPresetManager = MacosWindowPresetManager::default();
    }
}
