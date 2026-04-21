//! macOS system tray implementation using NSStatusBar

use crate::platform::traits::TrayIconTrait;
use anyhow::Result;
use tracing::info;

/// macOS tray icon implementation using NSStatusBar
pub struct MacosTrayIcon {
    visible: bool,
}

impl MacosTrayIcon {
    /// Create a new macOS tray icon
    pub fn new() -> Self {
        Self { visible: false }
    }
}

impl Default for MacosTrayIcon {
    fn default() -> Self {
        Self::new()
    }
}

impl TrayIconTrait for MacosTrayIcon {
    fn show(&mut self) -> Result<()> {
        // For now, just log that the tray icon would be shown
        // Full implementation would require:
        // 1. Using cocoa crate to create NSStatusBar and NSStatusItem
        // 2. Setting up menu items with callbacks
        // 3. Running the app in accessory mode (no dock icon)
        info!("Tray icon show requested (macOS implementation placeholder)");
        self.visible = true;
        Ok(())
    }

    fn hide(&mut self) -> Result<()> {
        info!("Tray icon hide requested");
        self.visible = false;
        Ok(())
    }

    fn show_notification(&mut self, title: &str, message: &str) -> Result<()> {
        // Use AppleScript to show notification
        let script = format!(
            r#"display notification "{}" with title "{}""#,
            message.replace('"', "\\\""),
            title.replace('"', "\\\"")
        );

        std::process::Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .spawn()
            .ok();

        info!("Notification: {} - {}", title, message);
        Ok(())
    }

    fn show_menu(&mut self) -> Result<()> {
        // The menu is automatically shown when clicking the status item
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macos_tray_icon_creation() {
        let tray = MacosTrayIcon::new();
        assert!(!tray.visible);
    }
}
