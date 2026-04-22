//! macOS window preset management
//!
//! Provides window preset functionality for saving, loading, and automatically
//! applying window layouts based on configuration rules.
#![cfg(target_os = "macos")]

use crate::config::wildcard_match;
use crate::platform::macos::window_api::{MacosWindowApi, RealMacosWindowApi};
use crate::platform::traits::WindowInfo;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info};

/// Window preset definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowPreset {
    pub name: String,
    /// Process name pattern (supports wildcards)
    pub process_pattern: String,
    /// Window title pattern (optional, supports wildcards)
    #[serde(default)]
    pub title_pattern: Option<String>,
    /// Target x position
    pub x: i32,
    /// Target y position
    pub y: i32,
    /// Target width
    pub width: i32,
    /// Target height
    pub height: i32,
    /// Whether to maximize after positioning
    #[serde(default)]
    pub maximize: bool,
    /// Whether to minimize after positioning
    #[serde(default)]
    pub minimize: bool,
}

/// macOS window preset manager
pub struct MacosWindowPresetManager<A: MacosWindowApi> {
    api: A,
    presets: Vec<WindowPreset>,
    saved_presets: HashMap<String, (i32, i32, i32, i32)>,
}

impl<A: MacosWindowApi> MacosWindowPresetManager<A> {
    /// Create a new preset manager with the given API
    pub fn new(api: A) -> Self {
        Self {
            api,
            presets: Vec::new(),
            saved_presets: HashMap::new(),
        }
    }

    /// Load presets from a list of preset definitions
    pub fn load_presets(&mut self, presets: Vec<WindowPreset>) {
        info!("Loaded {} window presets", presets.len());
        self.presets = presets;
    }

    /// Get all loaded presets
    pub fn get_presets(&self) -> &[WindowPreset] {
        &self.presets
    }

    /// Save current foreground window as a named preset
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

    /// Load and apply a saved preset by name to the foreground window
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

    /// Get foreground window information
    pub fn get_foreground_window_info(&self) -> Option<Result<WindowInfo>> {
        let id = self.api.get_foreground_window()?;
        Some(self.api.get_window_info(id))
    }

    /// Check if a window matches a preset's conditions
    fn matches_preset(&self, info: &WindowInfo, preset: &WindowPreset) -> bool {
        if !wildcard_match(&info.process_name, &preset.process_pattern) {
            return false;
        }

        if let Some(ref title_pattern) = preset.title_pattern {
            if !wildcard_match(&info.title, title_pattern) {
                return false;
            }
        }

        true
    }

    /// Apply the first matching preset for the current foreground window
    pub fn apply_preset_for_window(&self) -> Result<Option<&WindowPreset>> {
        let id = match self.api.get_foreground_window() {
            Some(id) => id,
            None => return Ok(None),
        };

        let info = match self.api.get_window_info(id) {
            Ok(info) => info,
            Err(_) => return Ok(None),
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

                if preset.maximize {
                    let _ = self.api.maximize_window(id);
                } else if preset.minimize {
                    let _ = self.api.minimize_window(id);
                }

                debug!(
                    "Applied preset '{}' to window '{}' of process '{}'",
                    preset.name, info.title, info.process_name
                );

                return Ok(Some(preset));
            }
        }

        Ok(None)
    }

    /// Apply all matching presets (for batch operations)
    pub fn apply_matching_presets(&self) -> Result<Vec<&WindowPreset>> {
        let mut applied = Vec::new();

        if let Some(preset) = self.apply_preset_for_window()? {
            applied.push(preset);
        }

        Ok(applied)
    }

    /// Get count of loaded presets
    pub fn preset_count(&self) -> usize {
        self.presets.len()
    }

    /// Get count of saved presets
    pub fn saved_count(&self) -> usize {
        self.saved_presets.len()
    }

    /// Remove a saved preset by name
    pub fn remove_saved_preset(&mut self, name: &str) -> bool {
        self.saved_presets.remove(name).is_some()
    }

    /// Clear all loaded presets
    pub fn clear_presets(&mut self) {
        self.presets.clear();
    }

    /// Clear all saved presets
    pub fn clear_saved(&mut self) {
        self.saved_presets.clear();
    }
}

impl Default for MacosWindowPresetManager<RealMacosWindowApi> {
    fn default() -> Self {
        Self::new(RealMacosWindowApi::new())
    }
}

// Type alias for convenience
pub type RealMacosWindowPresetManager = MacosWindowPresetManager<RealMacosWindowApi>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::macos::window_api::MockMacosWindowApi;

    fn create_test_manager() -> MacosWindowPresetManager<MockMacosWindowApi> {
        MacosWindowPresetManager::new(MockMacosWindowApi::new())
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
                process_pattern: "Code".to_string(),
                title_pattern: None,
                x: 0,
                y: 0,
                width: 960,
                height: 1080,
                maximize: false,
                minimize: false,
            },
            WindowPreset {
                name: "Browser Right".to_string(),
                process_pattern: "*Chrome*".to_string(),
                title_pattern: Some("*GitHub*".to_string()),
                x: 960,
                y: 0,
                width: 960,
                height: 1080,
                maximize: false,
                minimize: false,
            },
        ]);

        assert_eq!(mgr.preset_count(), 2);
        assert_eq!(mgr.get_presets()[0].name, "Editor Left");
        assert_eq!(mgr.get_presets()[1].name, "Browser Right");
    }

    #[test]
    fn test_save_and_load_preset() {
        let mut mgr = create_test_manager();

        // Save current window as preset
        mgr.save_preset("My Layout".to_string()).unwrap();
        assert_eq!(mgr.saved_count(), 1);

        // Load it back (should apply to foreground window)
        mgr.load_preset("My Layout").unwrap();
    }

    #[test]
    fn test_apply_preset_for_window() {
        let mut mgr = create_test_manager();

        mgr.load_presets(vec![WindowPreset {
            name: "TestApp Preset".to_string(),
            process_pattern: "TestApp".to_string(),
            title_pattern: None,
            x: 100,
            y: 200,
            width: 1024,
            height: 768,
            maximize: false,
            minimize: false,
        }]);

        // The mock has TestApp as the foreground window
        let result = mgr.apply_preset_for_window().unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "TestApp Preset");

        // Verify window was repositioned
        let info = mgr.api.get_window_info(1).unwrap();
        assert_eq!(info.x, 100);
        assert_eq!(info.y, 200);
        assert_eq!(info.width, 1024);
        assert_eq!(info.height, 768);
    }

    #[test]
    fn test_no_matching_preset() {
        let mut mgr = create_test_manager();

        mgr.load_presets(vec![WindowPreset {
            name: "Firefox Only".to_string(),
            process_pattern: "Firefox".to_string(),
            title_pattern: None,
            x: 0,
            y: 0,
            width: 800,
            height: 600,
            maximize: false,
            minimize: false,
        }]);

        // Mock foreground is TestApp, not Firefox - no match expected
        let result = mgr.apply_preset_for_window().unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_wildcard_matching() {
        let mut mgr = create_test_manager();

        mgr.load_presets(vec![WindowPreset {
            name: "All Apps".to_string(),
            process_pattern: "*".to_string(),
            title_pattern: None,
            x: 50,
            y: 50,
            width: 800,
            height: 600,
            maximize: false,
            minimize: false,
        }]);

        // Wildcard * should match any app including TestApp
        let result = mgr.apply_preset_for_window().unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().process_pattern, "*");
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
        assert_eq!(mgr.saved_count(), 1);
    }

    #[test]
    fn test_clear_presets() {
        let mut mgr = create_test_manager();

        mgr.load_presets(vec![WindowPreset {
            name: "Test".to_string(),
            process_pattern: "*".to_string(),
            title_pattern: None,
            x: 0,
            y: 0,
            width: 800,
            height: 600,
            maximize: false,
            minimize: false,
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
    fn test_maximize_in_preset() {
        let mut mgr = create_test_manager();

        mgr.load_presets(vec![WindowPreset {
            name: "Maximized".to_string(),
            process_pattern: "TestApp".to_string(),
            title_pattern: None,
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
            maximize: true,
            minimize: false,
        }]);

        let result = mgr.apply_preset_for_window().unwrap();
        assert!(result.unwrap().maximize);

        // Should be maximized (fills screen)
        let info = mgr.api.get_window_info(1).unwrap();
        assert_eq!(info.width, 1920);
        assert_eq!(info.height, 1080);
    }

    #[test]
    fn test_default_manager() {
        let _mgr: RealMacosWindowPresetManager = MacosWindowPresetManager::default();
    }
}
