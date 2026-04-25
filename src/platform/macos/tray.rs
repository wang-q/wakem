//! macOS system tray implementation using native Cocoa APIs
//!
//! This module provides a complete system tray implementation including:
//! - Tray icon management (register, unregister, notifications)
//! - Context menu with Enable/Disable, Reload Config, Open Config Folder, Exit
//! - NSApplication event loop for handling tray events
//! - Async API trait for integration with async code
#![cfg(target_os = "macos")]

use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};

use anyhow::{anyhow, Result};
use tracing::{debug, error, info, warn};

use cocoa::appkit::{NSApplication, NSMenu, NSMenuItem, NSStatusBar, NSStatusItem};
use cocoa::base::{id, nil, NO, YES};
use cocoa::foundation::{NSAutoreleasePool, NSString};
use objc::declare::ClassDecl;
use objc::runtime::{Class, Object, Protocol, Sel};
use objc::{class, msg_send, sel, sel_impl};

// Re-export shared tray types
pub use crate::platform::traits::{AppCommand, MenuAction};
// Import unified TrayApi trait from tray_common
pub use crate::platform::tray_common::TrayApi;
// Re-export menu ID constants from tray_common
use crate::platform::tray_common::menu_ids;

/// Global callback storage for menu actions
thread_local! {
    static GLOBAL_CALLBACK: RefCell<Option<Box<dyn Fn(AppCommand) + Send + 'static>>> = RefCell::new(None);
}

/// Set the global callback for menu actions
fn set_global_callback<F>(callback: F)
where
    F: Fn(AppCommand) + Send + 'static,
{
    GLOBAL_CALLBACK.with(|cb| {
        *cb.borrow_mut() = Some(Box::new(callback));
    });
}

/// Call the global callback with a command
fn call_global_callback(cmd: AppCommand) {
    GLOBAL_CALLBACK.with(|cb| {
        if let Some(ref callback) = *cb.borrow() {
            callback(cmd);
        }
    });
}

/// Menu item tag constants (from tray_common::menu_ids, cast to i64 for Cocoa)
const MENU_TAG_TOGGLE: i64 = menu_ids::TOGGLE_ACTIVE as i64;
const MENU_TAG_RELOAD: i64 = menu_ids::RELOAD as i64;
const MENU_TAG_OPEN_CONFIG: i64 = menu_ids::OPEN_CONFIG as i64;
const MENU_TAG_EXIT: i64 = menu_ids::EXIT as i64;

/// Create a custom Objective-C class for handling menu actions
fn create_menu_target_class() -> &'static Class {
    unsafe {
        let mut decl = ClassDecl::new("WakemMenuTarget", class!(NSObject))
            .expect("Failed to create WakemMenuTarget class");

        // Add the handleMenuItem: method to the class
        decl.add_method(
            sel!(handleMenuItem:),
            handle_menu_item as extern "C" fn(&Object, Sel, id),
        );

        decl.register()
    }
}

/// Handle menu item selection - called from Objective-C
extern "C" fn handle_menu_item(_this: &Object, _sel: Sel, sender: id) {
    unsafe {
        let tag: i64 = msg_send![sender, tag];
        let cmd = match tag {
            MENU_TAG_TOGGLE => AppCommand::ToggleActive,
            MENU_TAG_RELOAD => AppCommand::ReloadConfig,
            MENU_TAG_OPEN_CONFIG => AppCommand::OpenConfigFolder,
            MENU_TAG_EXIT => AppCommand::Exit,
            _ => return,
        };
        call_global_callback(cmd);
    }
}

/// Create a menu item with title, tag and target/action
unsafe fn create_menu_item(title: &str, tag: i64, target: id) -> id {
    let title_ns = NSString::alloc(nil).init_str(title);
    let item: id = msg_send![class!(NSMenuItem), alloc];
    let item: id = msg_send![item, initWithTitle:title_ns action:sel!(handleMenuItem:) keyEquivalent:NSString::alloc(nil).init_str("")];
    let _: () = msg_send![item, setTag: tag];
    let _: () = msg_send![item, setTarget: target];
    let _: () = msg_send![item, setEnabled: YES];
    item
}

/// Create the context menu
unsafe fn create_context_menu(target: id) -> id {
    let menu: id = msg_send![class!(NSMenu), alloc];
    let menu: id = msg_send![menu, initWithTitle: NSString::alloc(nil).init_str("")];

    // Disable auto-enable so items stay enabled
    let _: () = msg_send![menu, setAutoenablesItems: NO];

    // Enable/Disable
    let toggle_item = create_menu_item("Enable/Disable", MENU_TAG_TOGGLE, target);
    let _: () = msg_send![menu, addItem: toggle_item];

    // Separator
    let separator: id = msg_send![class!(NSMenuItem), separatorItem];
    let _: () = msg_send![menu, addItem: separator];

    // Reload Config
    let reload_item = create_menu_item("Reload Config", MENU_TAG_RELOAD, target);
    let _: () = msg_send![menu, addItem: reload_item];

    // Open Config Folder
    let open_config_item =
        create_menu_item("Open Config Folder", MENU_TAG_OPEN_CONFIG, target);
    let _: () = msg_send![menu, addItem: open_config_item];

    // Separator
    let separator2: id = msg_send![class!(NSMenuItem), separatorItem];
    let _: () = msg_send![menu, addItem: separator2];

    // Exit
    let exit_item = create_menu_item("Exit", MENU_TAG_EXIT, target);
    let _: () = msg_send![menu, addItem: exit_item];

    menu
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

    /// Create the actual tray icon with menu
    /// This must be called on the main thread before run_tray_event_loop
    pub fn create_tray_icon(&self) -> Result<()> {
        unsafe {
            let _pool = NSAutoreleasePool::new(nil);

            // Create custom class for handling menu actions
            let target_class = create_menu_target_class();

            // Create an instance of our custom class as the target
            let target: id = msg_send![target_class, alloc];
            let target: id = msg_send![target, init];

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

            // Create the menu with our custom target
            let menu = create_context_menu(target);

            // Attach menu to status item
            let _: () = msg_send![status_item, setMenu: menu];

            *self.status_item.borrow_mut() = Some(status_item);
        }

        info!("Tray icon created with menu");
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
    async fn register(&self, _hwnd: Option<isize>) -> Result<()> {
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

    async fn set_active(&self, active: bool) -> Result<()> {
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
        self.api.register(None).await
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
pub fn run_tray_event_loop<F>(callback: F) -> Result<()>
where
    F: Fn(AppCommand) + Send + 'static,
{
    info!("Starting tray event loop (macOS native)");

    // Set up the global callback for menu actions
    set_global_callback(callback);

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
    async fn register(&self, _hwnd: Option<isize>) -> Result<()> {
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

    async fn set_active(&self, active: bool) -> Result<()> {
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

        api.register(None).await.unwrap();
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
