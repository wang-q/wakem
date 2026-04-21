//! macOS window API implementation using AppleScript
//!
//! This module uses AppleScript to manipulate windows on macOS.

use crate::platform::traits::{MonitorInfo, WindowApiTrait, WindowId, WindowInfo};
use anyhow::Result;
use core_graphics::display::{CGDisplay, CGDisplayBounds};
use std::process::Command;
use tracing::{debug, warn};

/// macOS window API implementation
pub struct MacosWindowApi;

impl MacosWindowApi {
    /// Create a new macOS window API instance
    pub fn new() -> Self {
        Self
    }

    /// Execute AppleScript and return result
    fn run_applescript(&self, script: &str) -> Result<String> {
        let output = Command::new("osascript")
            .arg("-e")
            .arg(script)
            .output()
            .map_err(|e| anyhow::anyhow!("Failed to execute AppleScript: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("AppleScript error: {}", stderr));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}

impl Default for MacosWindowApi {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowApiTrait for MacosWindowApi {
    fn get_foreground_window(&self) -> Option<WindowId> {
        // On macOS, we use a simple counter as window ID
        // Full implementation would use Accessibility API
        Some(1)
    }

    fn get_window_info(&self, _window: WindowId) -> Result<WindowInfo> {
        // Get frontmost window info using AppleScript
        let script = r#"
            tell application "System Events"
                set frontApp to first application process whose frontmost is true
                set appName to name of frontApp
                try
                    set winTitle to name of first window of frontApp
                on error
                    set winTitle to ""
                end try
                return {appName, winTitle}
            end tell
        "#;

        let result = self.run_applescript(script)?;
        let parts: Vec<&str> = result.split(", ").collect();

        let process_name = parts.get(0).unwrap_or(&"Unknown").to_string();
        let window_title = parts.get(1).unwrap_or(&"").to_string();

        Ok(WindowInfo {
            id: _window,
            title: window_title,
            process_name,
            executable_path: None,
            x: 0,
            y: 0,
            width: 800,
            height: 600,
        })
    }

    fn set_window_pos(
        &self,
        _window: WindowId,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<()> {
        let script = format!(
            r#"tell application "System Events"
                set position of first window of (first application process whose frontmost is true) to {{{}, {}}}
                set size of first window of (first application process whose frontmost is true) to {{{}, {}}}
            end tell"#,
            x, y, width, height
        );

        self.run_applescript(&script)?;
        debug!("Set window position: {}x{} at {}, {}", width, height, x, y);
        Ok(())
    }

    fn minimize_window(&self, _window: WindowId) -> Result<()> {
        let script = r#"tell application "System Events"
            set value of attribute "AXMinimized" of first window of (first application process whose frontmost is true) to true
        end tell"#;

        self.run_applescript(script)?;
        debug!("Minimized window");
        Ok(())
    }

    fn maximize_window(&self, _window: WindowId) -> Result<()> {
        // macOS doesn't have a direct "maximize" concept, but we can zoom the window
        let script = r#"tell application "System Events"
            click button 2 of first window of (first application process whose frontmost is true)
        end tell"#;

        self.run_applescript(script)?;
        debug!("Maximized (zoomed) window");
        Ok(())
    }

    fn restore_window(&self, _window: WindowId) -> Result<()> {
        let script = r#"tell application "System Events"
            set value of attribute "AXMinimized" of first window of (first application process whose frontmost is true) to false
        end tell"#;

        self.run_applescript(script)?;
        debug!("Restored window");
        Ok(())
    }

    fn close_window(&self, _window: WindowId) -> Result<()> {
        let script = r#"tell application "System Events"
            click button 1 of first window of (first application process whose frontmost is true)
        end tell"#;

        self.run_applescript(script)?;
        debug!("Closed window");
        Ok(())
    }

    fn set_topmost(&self, _window: WindowId, _topmost: bool) -> Result<()> {
        // Window topmost on macOS requires private APIs
        warn!("set_topmost not fully implemented on macOS");
        Ok(())
    }

    fn set_opacity(&self, _window: WindowId, _opacity: u8) -> Result<()> {
        // Window opacity on macOS requires private APIs
        warn!("set_opacity not implemented on macOS");
        Ok(())
    }

    fn get_monitors(&self) -> Vec<MonitorInfo> {
        let mut monitors = Vec::new();

        // Get all active displays
        let display_ids = CGDisplay::active_displays().unwrap_or_default();

        for display_id in display_ids {
            let bounds = CGDisplayBounds(display_id);
            monitors.push(MonitorInfo {
                x: bounds.origin.x as i32,
                y: bounds.origin.y as i32,
                width: bounds.size.width as i32,
                height: bounds.size.height as i32,
            });
        }

        // Fallback to primary display if no monitors found
        if monitors.is_empty() {
            let main = CGDisplay::main();
            let bounds = CGDisplayBounds(main.id);
            monitors.push(MonitorInfo {
                x: bounds.origin.x as i32,
                y: bounds.origin.y as i32,
                width: bounds.size.width as i32,
                height: bounds.size.height as i32,
            });
        }

        monitors
    }

    fn move_to_monitor(&self, _window: WindowId, monitor_index: usize) -> Result<()> {
        let monitors = self.get_monitors();
        if monitor_index >= monitors.len() {
            return Err(anyhow::anyhow!("Invalid monitor index: {}", monitor_index));
        }

        let monitor = &monitors[monitor_index];
        let info = self.get_window_info(_window)?;

        // Move window to the target monitor, keeping the same size
        self.set_window_pos(_window, monitor.x, monitor.y, info.width, info.height)?;

        debug!("Moved window to monitor {}", monitor_index);
        Ok(())
    }

    fn is_window_valid(&self, window: WindowId) -> bool {
        window != 0
    }

    fn is_minimized(&self, _window: WindowId) -> bool {
        // Would need Accessibility API to check this properly
        false
    }

    fn is_maximized(&self, _window: WindowId) -> bool {
        // macOS doesn't have a clear "maximized" state
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macos_window_api_creation() {
        let api = MacosWindowApi::new();
        let _ = api;
    }

    #[test]
    fn test_get_monitors() {
        let api = MacosWindowApi::new();
        let monitors = api.get_monitors();
        assert!(!monitors.is_empty(), "Should have at least one monitor");
    }
}
