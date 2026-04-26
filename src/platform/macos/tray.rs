//! macOS system tray implementation using native Cocoa APIs
//!
//! This module provides a complete system tray implementation including:
//! - Tray icon management (register, unregister, notifications)
//! - Context menu with Enable/Disable, Reload Config, Open Config Folder, Exit
//! - NSApplication event loop for handling tray events
//! - Async API trait for integration with async code

// Allow deprecated cocoa APIs - migration to objc2 is planned for future
#![allow(deprecated)]

use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use anyhow::{anyhow, Result};
use tracing::{debug, info};

use cocoa::appkit::NSStatusBar;
use cocoa::base::{id, nil, NO, YES};
use cocoa::foundation::{NSAutoreleasePool, NSString};
use objc::declare::ClassDecl;
use objc::runtime::{Class, Object, Sel};
use objc::{class, msg_send, sel, sel_impl};

// Re-export shared tray types
pub use crate::platform::traits::AppCommand;
// Import unified TrayApi trait from tray_common
pub use crate::platform::tray_common::TrayApi;
// Re-export menu ID constants from tray_common
use crate::platform::tray_common::menu_ids;

/// Type alias for the global callback to reduce complexity
pub type AppCommandCallback = Box<dyn Fn(AppCommand) + Send + 'static>;

// Global callback storage for menu actions
thread_local! {
    static GLOBAL_CALLBACK: RefCell<Option<AppCommandCallback>> = RefCell::new(None);
}

/// Set the global callback for menu actions
fn set_global_callback<F>(callback: F)
where
    F: Fn(AppCommand) + Send + 'static,
{
    GLOBAL_CALLBACK.with(|cb| {
        *cb.borrow_mut() = Some(AppCommandCallback::from(Box::new(callback)));
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
    status_item: Mutex<Option<id>>,
}

impl RealTrayApi {
    pub fn new() -> Self {
        Self {
            registered: AtomicBool::new(false),
            visible: AtomicBool::new(true),
            active: AtomicBool::new(true),
            status_item: Mutex::new(None),
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
        assert_main_thread();
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
        assert_main_thread();
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

            *self.status_item.lock().unwrap() = Some(status_item);
        }

        info!("Tray icon created with menu");
        Ok(())
    }

    /// Synchronous version of unregister for use in blocking context
    pub fn unregister_blocking(&self) -> Result<()> {
        assert_main_thread();
        if !self.registered.load(Ordering::SeqCst) {
            return Ok(());
        }

        unsafe {
            if let Some(status_item) = *self.status_item.lock().unwrap() {
                let status_bar = NSStatusBar::systemStatusBar(nil);
                let _: () = msg_send![status_bar, removeStatusItem: status_item];
            }
        }

        *self.status_item.lock().unwrap() = None;
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

// SAFETY: RealTrayApi is Send + Sync.
// All shared mutable state is protected by atomic operations (AtomicBool)
// or Mutex (status_item). Cocoa UI operations (msg_send! on status_item)
// must only be performed on the main thread. Methods that access Cocoa UI
// include a main-thread assertion in debug builds to catch misuse.
// In production, the NSApplication event loop ensures correct sequencing.
unsafe impl Send for RealTrayApi {}
unsafe impl Sync for RealTrayApi {}

/// Assert that the current thread is the main thread.
/// Cocoa UI operations must run on the main thread.
/// In debug builds, this logs an error. In release builds, it logs a warning
/// since the behavior is technically undefined but often works in practice.
fn assert_main_thread() {
    use objc::runtime::Class;
    unsafe {
        let cls = match Class::get("NSThread") {
            Some(cls) => cls,
            None => return,
        };
        let is_main: bool = msg_send![cls, isMainThread];
        if !is_main {
            #[cfg(debug_assertions)]
            tracing::error!(
                "Cocoa UI operation called from non-main thread! \
                 This is undefined behavior. All tray UI operations \
                 must be dispatched to the main thread."
            );
            #[cfg(not(debug_assertions))]
            tracing::warn!(
                "Cocoa UI operation called from non-main thread. \
                 This may cause undefined behavior. All tray UI operations \
                 should be dispatched to the main thread."
            );
        }
    }
}

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
        assert_main_thread();
        self.visible.store(true, Ordering::SeqCst);
        unsafe {
            if let Some(status_item) = *self.status_item.lock().unwrap() {
                let _: () = msg_send![status_item, setVisible: YES];
            }
        }
        Ok(())
    }

    async fn hide(&self) -> Result<()> {
        assert_main_thread();
        self.visible.store(false, Ordering::SeqCst);
        unsafe {
            if let Some(status_item) = *self.status_item.lock().unwrap() {
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

    async fn is_active(&self) -> bool {
        self.active.load(Ordering::SeqCst)
    }

    async fn show_menu(&self) -> Result<u32> {
        // macOS menu is displayed synchronously via Cocoa
        // The menu action is handled by the global callback
        // Return 0 to indicate no synchronous selection
        Ok(0)
    }

    fn is_registered(&self) -> bool {
        self.registered.load(Ordering::SeqCst)
    }
}

/// Run the tray event loop
/// This function initializes NSApplication, creates the tray, and runs the event loop
pub fn run_tray_message_loop<F>(callback: F) -> Result<()>
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

/// Stop the tray message loop
/// On macOS, this posts a stop request to the NSApplication
#[allow(dead_code)]
pub fn stop_tray() {
    unsafe {
        use cocoa::base::id;
        use objc::{msg_send, sel, sel_impl};

        let app_class = Class::get("NSApplication").expect("NSApplication not found");
        let app: id = msg_send![app_class, sharedApplication];

        if app != nil {
            let _: () = msg_send![app, terminate: nil];
        }
    }
}

/// TrayIcon for macOS (aligned with Windows API)
/// On macOS, this wraps RealTrayApi for API compatibility
#[allow(dead_code)]
pub struct TrayIcon {
    inner: RealTrayApi,
}

#[allow(dead_code)]
impl TrayIcon {
    /// Create new tray icon
    pub fn new() -> Self {
        Self {
            inner: RealTrayApi::new(),
        }
    }

    /// Register tray icon (no-op on macOS, use run_tray_message_loop instead)
    pub fn register(&mut self, _hwnd: Option<isize>) -> Result<()> {
        self.inner.register_blocking()
    }

    /// Unregister tray icon
    pub fn unregister(&mut self) -> Result<()> {
        self.inner.unregister_blocking()
    }

    /// Show notification
    pub fn show_notification(&mut self, title: &str, message: &str) -> Result<()> {
        self.inner.show_notification_native(title, message)
    }

    /// Show context menu (no-op on macOS)
    pub fn show_menu(&mut self) -> Result<u32> {
        Ok(0)
    }
}

impl Default for TrayIcon {
    fn default() -> Self {
        Self::new()
    }
}

// Re-export shared tray types from tray_common (aligned with Windows)
#[allow(unused_imports)]
pub use crate::platform::tray_common::TrayManager;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::tray_common::{MockTrayApi, TrayIconWrapper, TrayManager};

    /// Type alias for test
    pub type MockTrayManager = TrayManager<MockTrayApi>;

    #[test]
    fn test_app_command_variants() {
        assert_eq!(AppCommand::ToggleActive, AppCommand::ToggleActive);
        assert_ne!(AppCommand::ToggleActive, AppCommand::Exit);
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
