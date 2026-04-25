//! Common tray manager implementation
//!
//! This module provides a platform-agnostic tray API trait and manager
//! that works across Windows and macOS platforms.

use crate::platform::traits::{AppCommand, MenuAction};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::mpsc::{channel, Receiver, Sender};
use tracing::{info, warn};

/// Tray API trait - abstracts platform-specific tray operations
///
/// Unified trait supporting both Windows (hwnd-based registration) and
/// macOS (NSStatusItem-based registration) tray implementations.
///
/// Platform-specific methods (set_tooltip, set_icon, show, hide) have
/// default no-op implementations so only platforms that need them override.
#[async_trait]
pub trait TrayApi: Send + Sync {
    /// Register tray icon.
    ///
    /// On Windows, pass `Some(hwnd)` for the message window handle.
    /// On macOS, pass `None` (no handle needed).
    async fn register(&self, hwnd: Option<isize>) -> Result<()>;

    /// Unregister tray icon
    async fn unregister(&self) -> Result<()>;

    /// Show balloon/notification
    async fn show_notification(&self, title: &str, message: &str) -> Result<()>;

    /// Show context menu, return selected menu item ID.
    /// Default returns 0 (no selection).
    async fn show_menu(&self) -> Result<u32> {
        Ok(0)
    }

    /// Set active/enabled status
    async fn set_active(&self, active: bool) -> Result<()>;

    /// Alias for set_active (macOS naming convention compatibility)
    async fn set_active_status(&self, active: bool) -> Result<()> {
        self.set_active(active).await
    }

    /// Get active status. Default returns true.
    async fn is_active(&self) -> bool {
        true
    }

    /// Set tooltip text. Default no-op (Windows uses NOTIFYICONDATA tip).
    async fn set_tooltip(&self, _tooltip: &str) -> Result<()> {
        Ok(())
    }

    /// Set icon from path. Default no-op.
    async fn set_icon(&self, _icon_path: Option<&str>) -> Result<()> {
        Ok(())
    }

    /// Show tray icon. Default no-op.
    async fn show(&self) -> Result<()> {
        Ok(())
    }

    /// Hide tray icon. Default no-op.
    async fn hide(&self) -> Result<()> {
        Ok(())
    }

    /// Get sent notifications (for testing only)
    fn get_notifications(&self) -> Vec<(String, String)> {
        Vec::new()
    }

    /// Check if registered (for testing only)
    fn is_registered(&self) -> bool {
        false
    }

    /// Preset menu selections (for testing only)
    fn set_menu_selections(&self, _selections: Vec<u32>) {}
}

/// Tray icon manager - works with any TrayApi implementation
///
/// This is a generic manager that handles high-level tray operations
/// like showing menus, notifications, and managing active state.
/// Also manages a command channel for receiving [AppCommand] events
/// from the tray menu (e.g., ToggleActive, ReloadConfig, Exit).
///
/// Previously, Windows and macOS had separate `TrayManager` definitions.
/// This unified version combines the API wrapper from the Windows version
/// with the lifecycle management from the macOS version.
pub struct TrayManager<T: TrayApi> {
    tray: Option<TrayIconWrapper<T>>,
    command_sender: Sender<AppCommand>,
    running: bool,
}

impl<T: TrayApi + Default + 'static> TrayManager<T> {
    /// Create a new TrayManager with default API and a command channel.
    ///
    /// Returns `(TrayManager, Receiver<AppCommand>)` so the caller can
    /// receive commands from the tray menu.
    pub fn new() -> (Self, Receiver<AppCommand>) {
        let (sender, receiver) = channel();
        let mgr = Self {
            tray: None,
            command_sender: sender,
            running: false,
        };
        (mgr, receiver)
    }

    /// Start the tray: register icon, set tooltip.
    pub async fn start(&mut self) -> Result<()> {
        if self.running {
            return Ok(());
        }

        let icon = TrayIconWrapper::new(T::default());
        icon.register().await?;
        icon.set_tooltip("wakem - Window Adjust, Keyboard Enhance, Mouse")
            .await?;

        self.tray = Some(icon);
        self.running = true;

        info!("TrayManager started");
        Ok(())
    }

    /// Stop the tray: unregister icon.
    pub async fn stop(&mut self) -> Result<()> {
        if !self.running {
            return Ok(());
        }

        if let Some(ref tray) = self.tray {
            tray.unregister().await?;
        }

        self.tray = None;
        self.running = false;

        info!("TrayManager stopped");
        Ok(())
    }

    /// Check if the tray is currently running
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Show a notification via the tray
    pub async fn notify(&self, title: &str, message: &str) -> Result<()> {
        if let Some(ref tray) = self.tray {
            tray.show_notification(title, message).await
        } else {
            warn!("Cannot send notification: tray not initialized");
            Err(anyhow::anyhow!("Tray not initialized"))
        }
    }

    /// Set the active status via the tray
    pub async fn set_active_status(&self, active: bool) -> Result<()> {
        if let Some(ref tray) = self.tray {
            tray.set_active_status(active).await
        } else {
            warn!("Cannot set active status: tray not initialized");
            Err(anyhow::anyhow!("Tray not initialized"))
        }
    }

    /// Get a clone of the command sender for receiving tray menu commands
    pub fn get_command_sender(&self) -> Sender<AppCommand> {
        self.command_sender.clone()
    }
}

impl<T: TrayApi + Default + 'static> Default for TrayManager<T> {
    fn default() -> Self {
        let (mgr, _) = Self::new();
        mgr
    }
}

/// Legacy API-wrapper TrayManager methods (for backward compatibility).
///
/// These methods operate on the `api` directly without lifecycle management.
/// Prefer using `start()/stop()/notify()` for new code.
impl<T: TrayApi> TrayManager<T> {
    /// Create a TrayManager from an existing API instance.
    ///
    /// This is a legacy constructor that does not set up a command channel.
    /// Prefer `TrayManager::new()` which returns `(Self, Receiver<AppCommand>)`.
    pub fn from_api(api: T) -> Self {
        let (sender, _) = channel();
        Self {
            tray: Some(TrayIconWrapper::new(api)),
            command_sender: sender,
            running: false,
        }
    }

    /// Get a reference to the underlying API (if tray is initialized)
    pub fn api(&self) -> Option<&T> {
        self.tray.as_ref().map(|w| &w.api)
    }

    /// Initialize tray icon with optional window handle
    pub async fn init(&self, hwnd: Option<isize>) -> Result<()> {
        if let Some(ref tray) = self.tray {
            tray.api.register(hwnd).await
        } else {
            Err(anyhow::anyhow!("Tray not initialized"))
        }
    }

    /// Initialize with window handle (convenience for Windows)
    pub async fn init_with_hwnd(&self, hwnd: isize) -> Result<()> {
        self.init(Some(hwnd)).await
    }

    /// Initialize without window handle (convenience for macOS)
    pub async fn init_no_handle(&self) -> Result<()> {
        self.init(None).await
    }

    /// Cleanup tray icon
    pub async fn cleanup(&self) -> Result<()> {
        if let Some(ref tray) = self.tray {
            tray.api.unregister().await
        } else {
            Err(anyhow::anyhow!("Tray not initialized"))
        }
    }

    /// Toggle active status and return new state
    pub async fn toggle_active(&self) -> Result<bool> {
        if let Some(ref tray) = self.tray {
            let current = tray.api.is_active().await;
            let new_state = !current;
            tray.api.set_active(new_state).await?;
            Ok(new_state)
        } else {
            Err(anyhow::anyhow!("Tray not initialized"))
        }
    }

    /// Get current active status
    pub async fn is_active(&self) -> bool {
        if let Some(ref tray) = self.tray {
            tray.api.is_active().await
        } else {
            false
        }
    }

    /// Show context menu and return selected action
    pub async fn show_context_menu(&self) -> Result<MenuAction> {
        if let Some(ref tray) = self.tray {
            let selection = tray.api.show_menu().await?;
            Ok(menu_id_to_action(selection))
        } else {
            Err(anyhow::anyhow!("Tray not initialized"))
        }
    }
}

/// Menu ID constants for standard tray menu items
pub mod menu_ids {
    pub const TOGGLE_ACTIVE: u32 = 100;
    pub const RELOAD: u32 = 101;
    pub const OPEN_CONFIG: u32 = 102;
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

/// Generic tray icon wrapper providing convenient methods over [TrayApi].
///
/// Wraps any [TrayApi] implementation and exposes ergonomic methods
/// for common operations (register without hwnd, show_menu returning
/// [MenuAction], etc.). Previously duplicated in `macos/tray.rs`.
pub struct TrayIconWrapper<T: TrayApi> {
    pub api: T,
}

impl<T: TrayApi> TrayIconWrapper<T> {
    pub fn new(api: T) -> Self {
        Self { api }
    }

    pub async fn register(&self) -> Result<()> {
        self.api.register(None).await
    }

    pub async fn unregister(&self) -> Result<()> {
        self.api.unregister().await
    }

    pub async fn show_notification(&self, title: &str, message: &str) -> Result<()> {
        self.api.show_notification(title, message).await
    }

    pub async fn show_menu(&self) -> Result<MenuAction> {
        let selection = self.api.show_menu().await?;
        Ok(menu_id_to_action(selection))
    }

    pub async fn set_tooltip(&self, tooltip: &str) -> Result<()> {
        self.api.set_tooltip(tooltip).await
    }

    pub async fn set_icon(&self, icon_path: Option<&str>) -> Result<()> {
        self.api.set_icon(icon_path).await
    }

    pub async fn show(&self) -> Result<()> {
        self.api.show().await
    }

    pub async fn hide(&self) -> Result<()> {
        self.api.hide().await
    }

    pub async fn set_active_status(&self, active: bool) -> Result<()> {
        self.api.set_active_status(active).await
    }

    pub fn is_registered(&self) -> bool {
        self.api.is_registered()
    }
}

/// Unified mock [TrayApi] implementation for testing.
///
/// Replaces platform-specific `MockTrayApi` definitions that were previously
/// duplicated across `macos/tray.rs` and `windows/tray.rs`.
/// Uses `std::sync::Mutex` for synchronous locking compatible with both platforms.
pub struct MockTrayApi {
    state: std::sync::Mutex<MockTrayState>,
}

#[derive(Default)]
struct MockTrayState {
    registered: bool,
    hwnd: isize,
    active: bool,
    visible: bool,
    tooltip: String,
    notifications: Vec<(String, String)>,
    menu_selections: Vec<u32>,
    menu_index: usize,
    menu_actions: std::collections::VecDeque<MenuAction>,
}

impl Default for MockTrayApi {
    fn default() -> Self {
        Self::new()
    }
}

impl MockTrayApi {
    pub fn new() -> Self {
        Self {
            state: std::sync::Mutex::new(MockTrayState {
                active: true,
                ..Default::default()
            }),
        }
    }

    pub fn is_visible(&self) -> bool {
        self.state.lock().unwrap().visible
    }

    pub fn get_tooltip(&self) -> String {
        self.state.lock().unwrap().tooltip.clone()
    }

    pub fn push_menu_action(&self, action: MenuAction) {
        self.state.lock().unwrap().menu_actions.push_back(action);
    }

    pub fn clear(&self) {
        let mut s = self.state.lock().unwrap();
        s.notifications.clear();
        s.menu_actions.clear();
    }
}

#[async_trait::async_trait]
impl TrayApi for MockTrayApi {
    async fn register(&self, hwnd: Option<isize>) -> Result<()> {
        let mut s = self.state.lock().unwrap();
        s.registered = true;
        s.hwnd = hwnd.unwrap_or(0);
        Ok(())
    }

    async fn unregister(&self) -> Result<()> {
        self.state.lock().unwrap().registered = false;
        Ok(())
    }

    async fn show_notification(&self, title: &str, message: &str) -> Result<()> {
        self.state
            .lock()
            .unwrap()
            .notifications
            .push((title.to_string(), message.to_string()));
        Ok(())
    }

    async fn show_menu(&self) -> Result<u32> {
        let mut s = self.state.lock().unwrap();
        if let Some(action) = s.menu_actions.pop_front() {
            return Ok(match action {
                MenuAction::ToggleActive => menu_ids::TOGGLE_ACTIVE,
                MenuAction::Reload => menu_ids::RELOAD,
                MenuAction::OpenConfig => menu_ids::OPEN_CONFIG,
                MenuAction::Exit => menu_ids::EXIT,
                MenuAction::None => 0,
            });
        }
        if s.menu_index < s.menu_selections.len() {
            let selection = s.menu_selections[s.menu_index];
            s.menu_index += 1;
            Ok(selection)
        } else {
            Ok(0)
        }
    }

    async fn set_active(&self, active: bool) -> Result<()> {
        self.state.lock().unwrap().active = active;
        Ok(())
    }

    async fn is_active(&self) -> bool {
        self.state.lock().unwrap().active
    }

    async fn set_tooltip(&self, tooltip: &str) -> Result<()> {
        self.state.lock().unwrap().tooltip = tooltip.to_string();
        Ok(())
    }

    async fn set_icon(&self, _icon_path: Option<&str>) -> Result<()> {
        Ok(())
    }

    async fn show(&self) -> Result<()> {
        self.state.lock().unwrap().visible = true;
        Ok(())
    }

    async fn hide(&self) -> Result<()> {
        self.state.lock().unwrap().visible = false;
        Ok(())
    }

    fn get_notifications(&self) -> Vec<(String, String)> {
        self.state.lock().unwrap().notifications.clone()
    }

    fn is_registered(&self) -> bool {
        self.state.lock().unwrap().registered
    }

    fn set_menu_selections(&self, selections: Vec<u32>) {
        let mut s = self.state.lock().unwrap();
        s.menu_selections = selections;
        s.menu_index = 0;
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
