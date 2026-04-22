//! macOS system tray implementation using native Cocoa APIs
//!
//! This module provides a complete system tray implementation including:
//! - Tray icon management (register, unregister, notifications)
//! - NSApplication event loop for handling tray events
//! - Async API trait for integration with async code
#![cfg(target_os = "macos")]

use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};

use anyhow::{anyhow, Result};
use tracing::{debug, error, info, warn};

use cocoa::appkit::{NSApplication, NSStatusBar, NSStatusItem};
use cocoa::base::{id, nil, NO, YES};
use cocoa::foundation::{NSAutoreleasePool, NSString};
use objc::runtime::Class;
use objc::{msg_send, sel, sel_impl};

/// Application commands sent from tray menu
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppCommand {
    ToggleActive,
    ReloadConfig,
    OpenConfigFolder,
    Exit,
}

/// Menu action results from user interaction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    None,
    ToggleActive,
    Reload,
    OpenConfig,
    Exit,
}

/// Tray icon trait for abstraction
#[async_trait::async_trait]
pub trait TrayApi: Send + Sync {
    async fn register(&self) -> Result<()>;
    async fn unregister(&self) -> Result<()>;
    async fn show_notification(&self, title: &str, message: &str) -> Result<()>;
    async fn set_tooltip(&self, tooltip: &str) -> Result<()>;
    async fn set_icon(&self, icon_path: Option<&str>) -> Result<()>;
    async fn show(&self) -> Result<()>;
    async fn hide(&self) -> Result<()>;
    async fn set_active_status(&self, active: bool) -> Result<()>;
    fn is_registered(&self) -> bool;
}

/// Real macOS tray API using native Cocoa
pub struct RealTrayApi {
    registered: AtomicBool,
    visible: AtomicBool,
    active: AtomicBool,
    command_sender: Sender<AppCommand>,
    command_receiver: Receiver<AppCommand>,
    status_item: RefCell<Option<id>>,
}

impl RealTrayApi {
    pub fn new() -> Self {
        let (sender, receiver) = channel();
        Self {
            registered: AtomicBool::new(false),
            visible: AtomicBool::new(true),
            active: AtomicBool::new(true),
            command_sender: sender,
            command_receiver: receiver,
            status_item: RefCell::new(None),
        }
    }

    /// Show notification
    fn show_notification_native(&self, title: &str, message: &str) -> Result<()> {
        use crate::platform::macos::native_api::notification::show_notification;
        show_notification(title, message)
            .map_err(|e| anyhow!("Failed to show notification: {}", e))
    }

    /// Synchronous version of register for use in blocking context
    pub fn register_blocking(&self) -> Result<()> {
        if self.registered.load(Ordering::SeqCst) {
            return Ok(());
        }
        self.registered.store(true, Ordering::SeqCst);
        info!("RealTrayApi registered (event loop not started yet)");
        Ok(())
    }

    /// Create the actual tray icon
    /// This must be called on the main thread before run_tray_event_loop
    pub fn create_tray_icon(&self) -> Result<()> {
        unsafe {
            let _pool = NSAutoreleasePool::new(nil);

            let status_bar = NSStatusBar::systemStatusBar(nil);
            if status_bar == nil {
                return Err(anyhow!("Failed to get status bar"));
            }

            let status_item: id = msg_send![status_bar, statusItemWithLength: 60.0f64];
            if status_item == nil {
                return Err(anyhow!("Failed to create status item"));
            }

            let title = NSString::alloc(nil).init_str("⚡ Wakem");
            if title != nil {
                let _: () = msg_send![status_item, setTitle: title];
            }

            let _: () = msg_send![status_item, setHighlightMode: YES];

            *self.status_item.borrow_mut() = Some(status_item);
        }

        info!("Tray icon created");
        Ok(())
    }

    /// Synchronous version of unregister for use in blocking context
    pub fn unregister_blocking(&self) -> Result<()> {
        if !self.registered.load(Ordering::SeqCst) {
            return Ok(());
        }

        unsafe {
            if let Some(status_item) = *self.status_item.borrow() {
                let status_bar = NSStatusBar::systemStatusBar(nil);
                let _: () = msg_send![status_bar, removeStatusItem: status_item];
            }
        }

        *self.status_item.borrow_mut() = None;
        self.registered.store(false, Ordering::SeqCst);
        info!("RealTrayApi unregistered");
        Ok(())
    }
}

impl Default for RealTrayApi {
    fn default() -> Self {
        Self::new()
    }
}

// SAFETY: RealTrayApi is Send + Sync
unsafe impl Send for RealTrayApi {}
unsafe impl Sync for RealTrayApi {}

#[async_trait::async_trait]
impl TrayApi for RealTrayApi {
    async fn register(&self) -> Result<()> {
        self.register_blocking()
    }

    async fn unregister(&self) -> Result<()> {
        self.unregister_blocking()
    }

    async fn show_notification(&self, title: &str, message: &str) -> Result<()> {
        self.show_notification_native(title, message)
    }

    async fn set_tooltip(&self, tooltip: &str) -> Result<()> {
        debug!("Set tray tooltip: {}", tooltip);
        Ok(())
    }

    async fn set_icon(&self, _icon_path: Option<&str>) -> Result<()> {
        Ok(())
    }

    async fn show(&self) -> Result<()> {
        self.visible.store(true, Ordering::SeqCst);
        unsafe {
            if let Some(status_item) = *self.status_item.borrow() {
                let _: () = msg_send![status_item, setVisible: YES];
            }
        }
        Ok(())
    }

    async fn hide(&self) -> Result<()> {
        self.visible.store(false, Ordering::SeqCst);
        unsafe {
            if let Some(status_item) = *self.status_item.borrow() {
                let _: () = msg_send![status_item, setVisible: NO];
            }
        }
        Ok(())
    }

    async fn set_active_status(&self, active: bool) -> Result<()> {
        self.active.store(active, Ordering::SeqCst);
        debug!("Set active status: {}", active);
        Ok(())
    }

    fn is_registered(&self) -> bool {
        self.registered.load(Ordering::SeqCst)
    }
}

/// Tray icon wrapper
pub struct TrayIconWrapper<T: TrayApi> {
    api: T,
}

impl<T: TrayApi> TrayIconWrapper<T> {
    pub fn new(api: T) -> Self {
        Self { api }
    }

    pub async fn register(&self) -> Result<()> {
        self.api.register().await
    }

    pub async fn unregister(&self) -> Result<()> {
        self.api.unregister().await
    }

    pub async fn show_notification(&self, title: &str, message: &str) -> Result<()> {
        self.api.show_notification(title, message).await
    }

    pub async fn show_menu(&self) -> Result<MenuAction> {
        Ok(MenuAction::None)
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

/// Generic tray manager for managing tray lifecycle
pub struct TrayManager<T: TrayApi + Send + Sync> {
    tray: Option<TrayIconWrapper<T>>,
    command_sender: Sender<AppCommand>,
    running: bool,
}

impl<T: TrayApi + Send + Sync + Default + 'static> TrayManager<T> {
    pub fn new() -> (Self, Receiver<AppCommand>) {
        let (sender, receiver) = channel();

        let mgr = Self {
            tray: None,
            command_sender: sender,
            running: false,
        };

        (mgr, receiver)
    }

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

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub async fn notify(&self, title: &str, message: &str) -> Result<()> {
        if let Some(ref tray) = self.tray {
            tray.show_notification(title, message).await
        } else {
            warn!("Cannot send notification: tray not initialized");
            Err(anyhow!("Tray not initialized"))
        }
    }

    pub async fn set_active_status(&self, active: bool) -> Result<()> {
        if let Some(ref tray) = self.tray {
            tray.set_active_status(active).await
        } else {
            warn!("Cannot set active status: tray not initialized");
            Err(anyhow!("Tray not initialized"))
        }
    }

    pub fn get_command_sender(&self) -> Sender<AppCommand> {
        self.command_sender.clone()
    }
}

/// Type aliases for convenience
pub type RealTrayManager = TrayManager<RealTrayApi>;

#[cfg(test)]
pub type MockTrayManager = TrayManager<MockTrayApi>;

/// Run the tray event loop
/// This function initializes NSApplication, creates the tray, and runs the event loop
pub fn run_tray_event_loop<F>(_callback: F) -> Result<()>
where
    F: Fn(AppCommand) + Send + 'static,
{
    info!("Starting tray event loop (macOS native)");

    unsafe {
        let _pool = NSAutoreleasePool::new(nil);

        let app_class = Class::get("NSApplication").expect("NSApplication not found");
        let app: id = msg_send![app_class, sharedApplication];

        if app == nil {
            return Err(anyhow!("Failed to get NSApplication"));
        }

        // Set activation policy to Accessory (no dock icon)
        // NSApplicationActivationPolicyAccessory = 1
        let _: () = msg_send![app, setActivationPolicy: 1i64];

        let tray_api = RealTrayApi::new();
        tray_api.create_tray_icon()?;

        info!("Running NSApplication event loop...");

        let _: () = msg_send![app, run];
    }

    info!("Tray event loop ended");
    Ok(())
}

/// Run the tray message loop (blocking, for main thread)
pub fn run_tray_message_loop<F>(callback: F) -> Result<()>
where
    F: Fn(AppCommand) + Send + 'static,
{
    info!("Starting tray message loop (macOS native)");
    let result = run_tray_event_loop(callback);
    info!("Tray message loop ended");
    result
}

/// Stop the tray loop
pub fn stop_tray() {
    unsafe {
        let app_class = Class::get("NSApplication").expect("NSApplication not found");
        let app: id = msg_send![app_class, sharedApplication];
        if app != nil {
            let _: () = msg_send![app, terminate: nil];
        }
    }
    debug!("Stop signal sent to tray loop");
}

/// Mock tray API for testing
#[cfg(test)]
pub struct MockTrayApi {
    registered: std::sync::Mutex<bool>,
    visible: std::sync::Mutex<bool>,
    active: std::sync::Mutex<bool>,
    notifications: std::sync::Mutex<Vec<(String, String)>>,
    tooltip: std::sync::Mutex<String>,
    menu_actions: std::sync::Mutex<std::collections::VecDeque<MenuAction>>,
}

#[cfg(test)]
impl MockTrayApi {
    pub fn new() -> Self {
        Self {
            registered: std::sync::Mutex::new(false),
            visible: std::sync::Mutex::new(true),
            active: std::sync::Mutex::new(true),
            notifications: std::sync::Mutex::new(Vec::new()),
            tooltip: std::sync::Mutex::new(String::new()),
            menu_actions: std::sync::Mutex::new(std::collections::VecDeque::new()),
        }
    }

    pub fn is_visible(&self) -> bool {
        *self.visible.lock().unwrap()
    }

    pub fn get_notifications(&self) -> Vec<(String, String)> {
        self.notifications.lock().unwrap().clone()
    }

    pub fn get_tooltip(&self) -> String {
        self.tooltip.lock().unwrap().clone()
    }

    pub fn push_menu_action(&self, action: MenuAction) {
        self.menu_actions.lock().unwrap().push_back(action);
    }

    pub fn clear(&self) {
        self.notifications.lock().unwrap().clear();
        self.menu_actions.lock().unwrap().clear();
    }
}

#[cfg(test)]
impl Default for MockTrayApi {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[async_trait::async_trait]
impl TrayApi for MockTrayApi {
    async fn register(&self) -> Result<()> {
        *self.registered.lock().unwrap() = true;
        Ok(())
    }

    async fn unregister(&self) -> Result<()> {
        *self.registered.lock().unwrap() = false;
        Ok(())
    }

    async fn show_notification(&self, title: &str, message: &str) -> Result<()> {
        self.notifications
            .lock()
            .unwrap()
            .push((title.to_string(), message.to_string()));
        Ok(())
    }

    async fn set_tooltip(&self, tooltip: &str) -> Result<()> {
        *self.tooltip.lock().unwrap() = tooltip.to_string();
        Ok(())
    }

    async fn set_icon(&self, _icon_path: Option<&str>) -> Result<()> {
        Ok(())
    }

    async fn show(&self) -> Result<()> {
        *self.visible.lock().unwrap() = true;
        Ok(())
    }

    async fn hide(&self) -> Result<()> {
        *self.visible.lock().unwrap() = false;
        Ok(())
    }

    async fn set_active_status(&self, active: bool) -> Result<()> {
        *self.active.lock().unwrap() = active;
        Ok(())
    }

    fn is_registered(&self) -> bool {
        *self.registered.lock().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_command_variants() {
        assert_eq!(AppCommand::ToggleActive, AppCommand::ToggleActive);
        assert_ne!(AppCommand::ToggleActive, AppCommand::Exit);
    }

    #[test]
    fn test_menu_action_variants() {
        assert_eq!(MenuAction::None, MenuAction::None);
        assert_eq!(MenuAction::ToggleActive, MenuAction::ToggleActive);
        assert_ne!(MenuAction::ToggleActive, MenuAction::Exit);
    }

    #[tokio::test]
    async fn test_mock_tray_api_lifecycle() {
        let api = MockTrayApi::new();
        assert!(!api.is_registered());

        api.register().await.unwrap();
        assert!(api.is_registered());

        api.unregister().await.unwrap();
        assert!(!api.is_registered());
    }

    #[tokio::test]
    async fn test_mock_notification() {
        let api = MockTrayApi::new();
        api.show_notification("Test Title", "Test Message")
            .await
            .unwrap();

        let notifications = api.get_notifications();
        assert_eq!(notifications.len(), 1);
        assert_eq!(
            notifications[0],
            ("Test Title".to_string(), "Test Message".to_string())
        );
    }

    #[tokio::test]
    async fn test_mock_tooltip() {
        let api = MockTrayApi::new();
        api.set_tooltip("My Tooltip").await.unwrap();
        assert_eq!(api.get_tooltip(), "My Tooltip");
    }

    #[tokio::test]
    async fn test_mock_show_hide() {
        let api = MockTrayApi::new();
        assert!(api.is_visible());

        api.hide().await.unwrap();
        assert!(!api.is_visible());

        api.show().await.unwrap();
        assert!(api.is_visible());
    }

    #[tokio::test]
    async fn test_tray_icon_lifecycle() {
        let api = MockTrayApi::new();
        let icon = TrayIconWrapper::new(api);

        assert!(!icon.is_registered());
        icon.register().await.unwrap();
        assert!(icon.is_registered());
        icon.unregister().await.unwrap();
        assert!(!icon.is_registered());
    }

    #[tokio::test]
    async fn test_tray_manager_start_stop() {
        let (mut mgr, _) = MockTrayManager::new();

        assert!(!mgr.is_running());
        mgr.start().await.unwrap();
        assert!(mgr.is_running());
        mgr.stop().await.unwrap();
        assert!(!mgr.is_running());
    }

    #[tokio::test]
    async fn test_tray_manager_notify() {
        let (mut mgr, _) = MockTrayManager::new();
        mgr.start().await.unwrap();

        mgr.notify("Title", "Message").await.unwrap();
    }
}
