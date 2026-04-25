//! Common tray manager implementation
//!
//! This module provides a platform-agnostic tray manager that works
//! with any platform's TrayApi implementation.

use crate::platform::traits::MenuAction;
use anyhow::Result;
use async_trait::async_trait;

/// Tray API trait - abstracts platform-specific tray operations
#[async_trait]
pub trait TrayApi: Send + Sync {
    /// Register tray icon
    async fn register(&self, hwnd: isize) -> Result<()>;

    /// Unregister tray icon
    async fn unregister(&self) -> Result<()>;

    /// Show balloon notification
    async fn show_notification(&self, title: &str, message: &str) -> Result<()>;

    /// Show context menu, return selected menu item ID
    async fn show_menu(&self) -> Result<u32>;

    /// Set active status
    async fn set_active(&self, active: bool) -> Result<()>;

    /// Get active status
    async fn is_active(&self) -> bool;

    /// Get sent notifications (for testing only)
    fn get_notifications(&self) -> Vec<(String, String)>;

    /// Check if registered (for testing only)
    fn is_registered(&self) -> bool;

    /// Preset menu selections (for testing only)
    fn set_menu_selections(&self, selections: Vec<u32>);
}

/// Tray icon manager - works with any TrayApi implementation
///
/// This is a generic manager that handles high-level tray operations
/// like showing menus, notifications, and managing active state.
pub struct TrayManager<T: TrayApi> {
    pub api: T,
}

impl<T: TrayApi> TrayManager<T> {
    /// Create a new tray manager with the given API implementation
    pub fn new(api: T) -> Self {
        Self { api }
    }

    /// Initialize tray icon with the given window handle
    pub async fn init(&self, hwnd: isize) -> Result<()> {
        self.api.register(hwnd).await
    }

    /// Cleanup tray icon
    pub async fn cleanup(&self) -> Result<()> {
        self.api.unregister().await
    }

    /// Show notification
    pub async fn notify(&self, title: &str, message: &str) -> Result<()> {
        self.api.show_notification(title, message).await
    }

    /// Toggle active status and return new state
    pub async fn toggle_active(&self) -> Result<bool> {
        let current = self.api.is_active().await;
        let new_state = !current;
        self.api.set_active(new_state).await?;
        Ok(new_state)
    }

    /// Get current active status
    pub async fn is_active(&self) -> bool {
        self.api.is_active().await
    }

    /// Show context menu and return selected action
    pub async fn show_context_menu(&self) -> Result<MenuAction> {
        let selection = self.api.show_menu().await?;
        Ok(menu_id_to_action(selection))
    }
}

/// Menu ID constants for standard tray menu items
pub mod menu_ids {
    /// Toggle active state
    pub const TOGGLE_ACTIVE: u32 = 100;
    /// Reload configuration
    pub const RELOAD: u32 = 101;
    /// Open config folder
    pub const OPEN_CONFIG: u32 = 102;
    /// Exit application
    pub const EXIT: u32 = 103;
}

/// Convert menu ID to MenuAction
pub fn menu_id_to_action(id: u32) -> MenuAction {
    match id {
        menu_ids::TOGGLE_ACTIVE => MenuAction::ToggleActive,
        menu_ids::RELOAD => MenuAction::Reload,
        menu_ids::OPEN_CONFIG => MenuAction::OpenConfig,
        menu_ids::EXIT => MenuAction::Exit,
        _ => MenuAction::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_menu_id_to_action() {
        assert_eq!(
            menu_id_to_action(menu_ids::TOGGLE_ACTIVE),
            MenuAction::ToggleActive
        );
        assert_eq!(menu_id_to_action(menu_ids::RELOAD), MenuAction::Reload);
        assert_eq!(
            menu_id_to_action(menu_ids::OPEN_CONFIG),
            MenuAction::OpenConfig
        );
        assert_eq!(menu_id_to_action(menu_ids::EXIT), MenuAction::Exit);
        assert_eq!(menu_id_to_action(999), MenuAction::None);
    }
}
