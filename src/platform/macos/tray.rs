//! macOS system tray implementation using NSStatusBar

use crate::platform::traits::TrayIcon;
use anyhow::Result;

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

impl TrayIcon for MacosTrayIcon {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self::new()
    }

    fn show(&mut self) -> Result<()> {
        // TODO: Implement using NSStatusBar
        // 1. Get system status bar
        // 2. Create status item
        // 3. Set image/icon
        // 4. Setup menu

        self.visible = true;
        Ok(())
    }

    fn hide(&mut self) -> Result<()> {
        // TODO: Remove status item from status bar
        self.visible = false;
        Ok(())
    }

    fn show_notification(&mut self, title: &str, message: &str) -> Result<()> {
        // TODO: Implement using NSUserNotification
        // or a third-party crate like `notify-rust`

        let _ = (title, message);
        Ok(())
    }

    fn show_menu(&mut self) -> Result<()> {
        // TODO: Show context menu
        // NSStatusItem.popUpStatusItemMenu
        Ok(())
    }
}

/// Menu item action type
pub type MenuAction = Box<dyn Fn() + Send + 'static>;

/// Menu item
pub struct MenuItem {
    pub title: String,
    pub action: Option<MenuAction>,
    pub separator: bool,
}

impl MenuItem {
    /// Create a new menu item
    pub fn new(title: impl Into<String>, action: impl Fn() + Send + 'static) -> Self {
        Self {
            title: title.into(),
            action: Some(Box::new(action)),
            separator: false,
        }
    }

    /// Create a separator item
    pub fn separator() -> Self {
        Self {
            title: String::new(),
            action: None,
            separator: true,
        }
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
