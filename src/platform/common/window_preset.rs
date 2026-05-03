//! Common window preset management logic shared across platforms
//!
//! Provides a generic [WindowPresetManager] that works with any platform
//! implementing the [WindowPresetApi] trait. Eliminates duplicated preset
//! logic previously found in both `windows/window_preset.rs` and
//! `macos/window_preset.rs`.

use crate::config::wildcard_match;
use crate::config::WindowPreset;
use crate::platform::types::WindowInfo;
use anyhow::Result;
use std::collections::HashMap;
use tracing::{debug, info};

/// Platform API needed by the common window preset manager.
///
/// Each platform implements this trait to provide window query and
/// manipulation primitives. The common manager then builds all
/// higher-level preset operations on top of these primitives.
pub trait WindowPresetApi {
    type WindowId: Copy;

    fn get_foreground_window(&self) -> Option<Self::WindowId>;
    fn get_window_info(&self, window: Self::WindowId) -> Result<WindowInfo>;
    fn set_window_pos(
        &self,
        window: Self::WindowId,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
    ) -> Result<()>;
    fn minimize_window(&self, window: Self::WindowId) -> Result<()>;
    fn maximize_window(&self, window: Self::WindowId) -> Result<()>;
}

/// Generic window preset manager.
///
/// Manages a collection of [WindowPreset] definitions and saved presets,
/// providing save/load/apply operations that work identically on all
/// platforms through the [WindowPresetApi] trait.
pub struct WindowPresetManager<A: WindowPresetApi> {
    api: A,
    presets: Vec<WindowPreset>,
    saved_presets: HashMap<String, (i32, i32, i32, i32)>,
}

impl<A: WindowPresetApi> WindowPresetManager<A> {
    pub fn new(api: A) -> Self {
        Self {
            api,
            presets: Vec::new(),
            saved_presets: HashMap::new(),
        }
    }

    pub fn load_presets(&mut self, presets: Vec<WindowPreset>) {
        if !presets.is_empty() {
            info!("Loaded {} window presets", presets.len());
        } else {
            debug!("No window presets to load");
        }
        self.presets = presets;
    }

    pub fn get_presets(&self) -> &[WindowPreset] {
        &self.presets
    }

    pub fn save_preset(&mut self, name: String) -> Result<()> {
        let id = self
            .api
            .get_foreground_window()
            .ok_or_else(|| anyhow::anyhow!("No foreground window to save"))?;

        let info = self.api.get_window_info(id)?;
        self.saved_presets
            .insert(name.clone(), (info.x, info.y, info.width, info.height));

        debug!(
            "Saved preset '{}': {}x{} at ({}, {})",
            name, info.width, info.height, info.x, info.y
        );
        Ok(())
    }

    pub fn load_preset(&self, name: &str) -> Result<()> {
        let frame = self
            .saved_presets
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("Preset '{}' not found", name))?;

        let id = self
            .api
            .get_foreground_window()
            .ok_or_else(|| anyhow::anyhow!("No foreground window"))?;

        self.api
            .set_window_pos(id, frame.0, frame.1, frame.2, frame.3)?;

        debug!(
            "Applied preset '{}': {}x{} at ({}, {})",
            name, frame.2, frame.3, frame.0, frame.1
        );
        Ok(())
    }

    pub fn get_foreground_window_info(&self) -> Option<Result<WindowInfo>> {
        let id = self.api.get_foreground_window()?;
        Some(self.api.get_window_info(id))
    }

    fn matches_preset(&self, info: &WindowInfo, preset: &WindowPreset) -> bool {
        if let Some(ref pattern) = preset.process_name {
            if !wildcard_match(&info.process_name, pattern) {
                return false;
            }
        }

        if let Some(ref title_pattern) = preset.title_pattern {
            if !wildcard_match(&info.title, title_pattern) {
                return false;
            }
        }

        if let Some(ref exec_pattern) = preset.executable_path {
            let path = info.executable_path.as_deref().unwrap_or("");
            if !wildcard_match(path, exec_pattern) {
                return false;
            }
        }

        preset.process_name.is_some()
            || preset.executable_path.is_some()
            || preset.title_pattern.is_some()
    }

    pub fn apply_preset_for_window(&self) -> Result<bool> {
        let id = match self.api.get_foreground_window() {
            Some(id) => id,
            None => return Ok(false),
        };
        self.apply_preset_for_window_by_id(id)
    }

    pub fn apply_preset_for_window_by_id(&self, id: A::WindowId) -> Result<bool> {
        let info = match self.api.get_window_info(id) {
            Ok(info) => info,
            Err(_) => return Ok(false),
        };

        for preset in &self.presets {
            if self.matches_preset(&info, preset) {
                self.api.set_window_pos(
                    id,
                    preset.x,
                    preset.y,
                    preset.width,
                    preset.height,
                )?;

                debug!(
                    "Applied preset '{}' to window '{}' of process '{}'",
                    preset.name, info.title, info.process_name
                );

                return Ok(true);
            }
        }

        Ok(false)
    }

    pub fn apply_matching_presets(&self) -> Result<bool> {
        self.apply_preset_for_window()
    }

    pub fn preset_count(&self) -> usize {
        self.presets.len()
    }

    pub fn saved_count(&self) -> usize {
        self.saved_presets.len()
    }

    pub fn remove_saved_preset(&mut self, name: &str) -> bool {
        self.saved_presets.remove(name).is_some()
    }

    pub fn clear_presets(&mut self) {
        self.presets.clear();
    }

    pub fn clear_saved(&mut self) {
        self.saved_presets.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockApi {
        windows: std::collections::HashMap<usize, WindowInfo>,
        foreground: Option<usize>,
    }

    impl MockApi {
        fn new() -> Self {
            let mut windows = std::collections::HashMap::new();
            windows.insert(
                1,
                WindowInfo {
                    id: 1,
                    title: "Test Window".to_string(),
                    process_name: "TestApp".to_string(),
                    executable_path: Some("/usr/bin/testapp".to_string()),
                    x: 100,
                    y: 100,
                    width: 800,
                    height: 600,
                },
            );
            Self {
                windows,
                foreground: Some(1),
            }
        }
    }

    impl WindowPresetApi for MockApi {
        type WindowId = usize;

        fn get_foreground_window(&self) -> Option<Self::WindowId> {
            self.foreground
        }

        fn get_window_info(&self, window: Self::WindowId) -> Result<WindowInfo> {
            self.windows
                .get(&window)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Window not found"))
        }

        fn set_window_pos(
            &self,
            _window: Self::WindowId,
            _x: i32,
            _y: i32,
            _w: i32,
            _h: i32,
        ) -> Result<()> {
            Ok(())
        }

        fn minimize_window(&self, _window: Self::WindowId) -> Result<()> {
            Ok(())
        }

        fn maximize_window(&self, _window: Self::WindowId) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_preset_manager_creation() {
        let mgr = WindowPresetManager::new(MockApi::new());
        assert_eq!(mgr.preset_count(), 0);
        assert_eq!(mgr.saved_count(), 0);
    }

    #[test]
    fn test_load_presets() {
        let mut mgr = WindowPresetManager::new(MockApi::new());
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
    }

    #[test]
    fn test_save_and_load_preset() {
        let mut mgr = WindowPresetManager::new(MockApi::new());
        mgr.save_preset("My Layout".to_string()).unwrap();
        assert_eq!(mgr.saved_count(), 1);
        mgr.load_preset("My Layout").unwrap();
    }

    #[test]
    fn test_apply_preset_for_window() {
        let mut mgr = WindowPresetManager::new(MockApi::new());
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
        let mut mgr = WindowPresetManager::new(MockApi::new());
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
        let mut mgr = WindowPresetManager::new(MockApi::new());
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
        let mut mgr = WindowPresetManager::new(MockApi::new());
        mgr.save_preset("Layout1".to_string()).unwrap();
        mgr.save_preset("Layout2".to_string()).unwrap();
        assert_eq!(mgr.saved_count(), 2);
        assert!(mgr.remove_saved_preset("Layout1"));
        assert_eq!(mgr.saved_count(), 1);
        assert!(!mgr.remove_saved_preset("NonExistent"));
    }

    #[test]
    fn test_clear_presets() {
        let mut mgr = WindowPresetManager::new(MockApi::new());
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
        let mgr = WindowPresetManager::new(MockApi::new());
        let info_result = mgr.get_foreground_window_info();
        assert!(info_result.is_some());
        let info = info_result.unwrap().unwrap();
        assert_eq!(info.process_name, "TestApp");
    }

    #[test]
    fn test_executable_path_matching() {
        let mut mgr = WindowPresetManager::new(MockApi::new());
        mgr.load_presets(vec![WindowPreset {
            name: "By Path".to_string(),
            process_name: None,
            executable_path: Some("*/bin/testapp".to_string()),
            title_pattern: None,
            x: 0,
            y: 0,
            width: 800,
            height: 600,
        }]);
        let result = mgr.apply_preset_for_window().unwrap();
        assert!(result);
        assert_eq!(result, true);
    }
}
