//! macOS system tray (status bar) implementation
//!
//! Provides NSStatusBar-based tray icon with menu and notification support.
//! This is the macOS equivalent of Windows system tray.
//!
//! Design follows Windows version:
//! - Menu callbacks are handled directly via global callback function
//! - No separate thread for tray handling
//! - Commands are sent through mpsc channel to daemon

#![allow(deprecated)]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};
use cocoa::base::{id, nil, YES};
use cocoa::foundation::{NSAutoreleasePool, NSString};
use objc::declare::ClassDecl;
use objc::runtime::{Class, Object, Sel};
use objc::{class, msg_send, sel, sel_impl};
use tracing::{debug, info, warn};

/// Application commands sent from tray menu
#[derive(Debug, Clone, PartialEq, Eq)]
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

/// Global command callback for menu actions
/// This follows the Windows design pattern where menu callbacks
/// directly invoke a handler function
static GLOBAL_COMMAND_CALLBACK: Mutex<Option<Box<dyn Fn(AppCommand) + Send + 'static>>> =
    Mutex::new(None);

/// Menu item tag constants
const MENU_TAG_TOGGLE: i64 = 100;
const MENU_TAG_RELOAD: i64 = 101;
const MENU_TAG_OPEN_CONFIG: i64 = 102;
const MENU_TAG_EXIT: i64 = 103;

/// Tray icon trait for abstraction
#[async_trait::async_trait]
pub trait TrayApi: Send + Sync {
    async fn register(&self) -> Result<()>;
    async fn unregister(&self) -> Result<()>;
    async fn show_notification(&self, title: &str, message: &str) -> Result<()>;
    async fn show_menu(&self) -> Result<MenuAction>;
    async fn set_tooltip(&self, tooltip: &str) -> Result<()>;
    async fn set_icon(&self, icon_path: Option<&str>) -> Result<()>;
    async fn show(&self) -> Result<()>;
    async fn hide(&self) -> Result<()>;
    async fn set_active_status(&self, active: bool) -> Result<()>;
    fn is_registered(&self) -> bool;
}

/// Real macOS tray API using NSStatusBar
///
/// Note: We store Objective-C object pointers as usize to make them Send + Sync.
/// This is safe because:
/// 1. These pointers are only accessed from the main thread (macOS UI requirement)
/// 2. The mutex ensures exclusive access
pub struct RealTrayApi {
    registered: AtomicBool,
    visible: AtomicBool,
    active: AtomicBool,
    status_item: Mutex<Option<usize>>,
    menu: Mutex<Option<usize>>,
    toggle_item: Mutex<Option<usize>>,
}

impl RealTrayApi {
    pub fn new() -> Self {
        Self {
            registered: AtomicBool::new(false),
            visible: AtomicBool::new(true),
            active: AtomicBool::new(true),
            status_item: Mutex::new(None),
            menu: Mutex::new(None),
            toggle_item: Mutex::new(None),
        }
    }

    /// Helper to convert usize to id
    unsafe fn usize_to_id(ptr: usize) -> id {
        ptr as id
    }

    /// Helper to convert id to usize
    fn id_to_usize(ptr: id) -> usize {
        ptr as usize
    }

    /// Create the delegate class for menu actions
    fn create_delegate_class() {
        unsafe {
            // Check if class already exists
            if Class::get("WakemTrayDelegate").is_some() {
                return;
            }

            let superclass = class!(NSObject);
            let mut decl = ClassDecl::new("WakemTrayDelegate", superclass).unwrap();

            // Add method for menu item clicks
            decl.add_method(
                sel!(menuItemClicked:),
                menu_item_clicked as extern "C" fn(&Object, Sel, id),
            );

            decl.register();
        }
    }

    /// Create menu item
    unsafe fn create_menu_item(&self, title: &str, tag: i64, delegate: id) -> id {
        let title_ns = NSString::alloc(nil).init_str(title);
        let item: id = msg_send![class!(NSMenuItem), alloc];
        let item: id = msg_send![item,
            initWithTitle: title_ns
            action: sel!(menuItemClicked:)
            keyEquivalent: NSString::alloc(nil).init_str("")
        ];
        let _: () = msg_send![item, setTarget: delegate];
        let _: () = msg_send![item, setTag: tag];
        item
    }

    /// Show notification using native NSUserNotificationCenter API
    fn show_notification_native(&self, title: &str, message: &str) -> Result<()> {
        use crate::platform::macos::native_api::notification::show_notification;

        show_notification(title, message)
            .map_err(|e| anyhow!("Failed to show notification: {}", e))
    }
}

impl Default for RealTrayApi {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl TrayApi for RealTrayApi {
    async fn register(&self) -> Result<()> {
        if self.registered.load(Ordering::SeqCst) {
            return Ok(());
        }

        unsafe {
            let _pool = NSAutoreleasePool::new(nil);

            // Create delegate class
            Self::create_delegate_class();

            // Get status bar
            let status_bar: id = msg_send![class!(NSStatusBar), systemStatusBar];
            if status_bar == nil {
                return Err(anyhow!("Failed to get system status bar"));
            }

            // Create status item with variable length
            let status_item: id = msg_send![status_bar, statusItemWithLength: -1.0f64]; // NSVariableStatusItemLength
            if status_item == nil {
                return Err(anyhow!("Failed to create status item"));
            }

            // Set default icon (using system template icon)
            let button: id = msg_send![status_item, button];
            if button != nil {
                // Use NSImage with system symbol or template
                let image: id = msg_send![class!(NSImage), imageNamed: NSString::alloc(nil).init_str("NSActionTemplate")];
                if image != nil {
                    let _: () = msg_send![image, setTemplate: YES];
                    let _: () = msg_send![button, setImage: image];
                }
                let _: () = msg_send![button, setToolTip: NSString::alloc(nil).init_str("wakem")];
            }

            // Create delegate instance
            let delegate_class = Class::get("WakemTrayDelegate").unwrap();
            let delegate: id = msg_send![delegate_class, alloc];
            let delegate: id = msg_send![delegate, init];

            // Create menu
            let menu: id = msg_send![class!(NSMenu), alloc];
            let menu: id =
                msg_send![menu, initWithTitle: NSString::alloc(nil).init_str("wakem")];

            // Add "Enable/Disable" menu item
            let toggle_item =
                self.create_menu_item("Disable", MENU_TAG_TOGGLE, delegate);
            let _: () = msg_send![menu, addItem: toggle_item];

            // Add separator
            let separator: id = msg_send![class!(NSMenuItem), separatorItem];
            let _: () = msg_send![menu, addItem: separator];

            // Add "Reload Config" menu item
            let reload_item =
                self.create_menu_item("Reload Config", MENU_TAG_RELOAD, delegate);
            let _: () = msg_send![menu, addItem: reload_item];

            // Add "Open Config Folder" menu item
            let open_item = self.create_menu_item(
                "Open Config Folder",
                MENU_TAG_OPEN_CONFIG,
                delegate,
            );
            let _: () = msg_send![menu, addItem: open_item];

            // Add separator
            let separator2: id = msg_send![class!(NSMenuItem), separatorItem];
            let _: () = msg_send![menu, addItem: separator2];

            // Add "Exit" menu item
            let exit_item = self.create_menu_item("Exit", MENU_TAG_EXIT, delegate);
            let _: () = msg_send![menu, addItem: exit_item];

            // Set menu for status item
            let _: () = msg_send![status_item, setMenu: menu];

            // Store references (convert id to usize for Send + Sync)
            *self.status_item.lock().unwrap() = Some(Self::id_to_usize(status_item));
            *self.menu.lock().unwrap() = Some(Self::id_to_usize(menu));
            *self.toggle_item.lock().unwrap() = Some(Self::id_to_usize(toggle_item));
        }

        self.registered.store(true, Ordering::SeqCst);
        debug!("RealTrayApi registered on macOS");
        Ok(())
    }

    async fn unregister(&self) -> Result<()> {
        if !self.registered.load(Ordering::SeqCst) {
            return Ok(());
        }

        unsafe {
            if let Some(status_item_ptr) = *self.status_item.lock().unwrap() {
                let status_item = Self::usize_to_id(status_item_ptr);
                let status_bar: id = msg_send![class!(NSStatusBar), systemStatusBar];
                let _: () = msg_send![status_bar, removeStatusItem: status_item];
            }
        }

        *self.status_item.lock().unwrap() = None;
        *self.menu.lock().unwrap() = None;
        *self.toggle_item.lock().unwrap() = None;

        self.registered.store(false, Ordering::SeqCst);
        debug!("RealTrayApi unregistered");
        Ok(())
    }

    async fn show_notification(&self, title: &str, message: &str) -> Result<()> {
        self.show_notification_native(title, message)
    }

    async fn show_menu(&self) -> Result<MenuAction> {
        // On macOS, menu is shown automatically when user clicks the status item
        // This method is mainly for testing or programmatic menu display
        Ok(MenuAction::None)
    }

    async fn set_tooltip(&self, tooltip: &str) -> Result<()> {
        unsafe {
            if let Some(status_item_ptr) = *self.status_item.lock().unwrap() {
                let status_item = Self::usize_to_id(status_item_ptr);
                let button: id = msg_send![status_item, button];
                if button != nil {
                    let _: () = msg_send![button, setToolTip: NSString::alloc(nil).init_str(tooltip)];
                }
            }
        }
        debug!("Set tray tooltip: {}", tooltip);
        Ok(())
    }

    async fn set_icon(&self, icon_path: Option<&str>) -> Result<()> {
        unsafe {
            if let Some(status_item_ptr) = *self.status_item.lock().unwrap() {
                let status_item = Self::usize_to_id(status_item_ptr);
                let button: id = msg_send![status_item, button];
                if button != nil {
                    if let Some(path) = icon_path {
                        let image: id = msg_send![class!(NSImage), alloc];
                        let image: id = msg_send![image, initWithContentsOfFile: NSString::alloc(nil).init_str(path)];
                        if image != nil {
                            let _: () = msg_send![image, setTemplate: YES];
                            let _: () = msg_send![button, setImage: image];
                        } else {
                            warn!("Failed to load icon from path: {}", path);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    async fn show(&self) -> Result<()> {
        self.visible.store(true, Ordering::SeqCst);
        unsafe {
            if let Some(status_item_ptr) = *self.status_item.lock().unwrap() {
                let status_item = Self::usize_to_id(status_item_ptr);
                let _: () = msg_send![status_item, setVisible: YES];
            }
        }
        debug!("Tray shown");
        Ok(())
    }

    async fn hide(&self) -> Result<()> {
        self.visible.store(false, Ordering::SeqCst);
        unsafe {
            if let Some(status_item_ptr) = *self.status_item.lock().unwrap() {
                let status_item = Self::usize_to_id(status_item_ptr);
                let _: () = msg_send![status_item, setVisible: false];
            }
        }
        debug!("Tray hidden");
        Ok(())
    }

    async fn set_active_status(&self, active: bool) -> Result<()> {
        self.active.store(active, Ordering::SeqCst);

        unsafe {
            if let Some(toggle_item_ptr) = *self.toggle_item.lock().unwrap() {
                let toggle_item = Self::usize_to_id(toggle_item_ptr);
                let title = if active { "Disable" } else { "Enable" };
                let _: () = msg_send![toggle_item, setTitle: NSString::alloc(nil).init_str(title)];
            }
        }

        debug!("Set active status: {}", active);
        Ok(())
    }

    fn is_registered(&self) -> bool {
        self.registered.load(Ordering::SeqCst)
    }
}

/// Menu item click handler
/// This is called directly by AppKit when user clicks a menu item
/// Following Windows design: invoke callback directly without separate thread
extern "C" fn menu_item_clicked(_self: &Object, _cmd: Sel, sender: id) {
    unsafe {
        let tag: i64 = msg_send![sender, tag];

        let command = match tag {
            MENU_TAG_TOGGLE => Some(AppCommand::ToggleActive),
            MENU_TAG_RELOAD => Some(AppCommand::ReloadConfig),
            MENU_TAG_OPEN_CONFIG => Some(AppCommand::OpenConfigFolder),
            MENU_TAG_EXIT => Some(AppCommand::Exit),
            _ => None,
        };

        if let Some(cmd) = command {
            if let Ok(guard) = GLOBAL_COMMAND_CALLBACK.lock() {
                if let Some(ref callback) = *guard {
                    callback(cmd);
                }
            }
        }
    }
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

    async fn show_menu(&self) -> Result<MenuAction> {
        match self.menu_actions.lock().unwrap().pop_front() {
            Some(action) => Ok(action),
            None => Ok(MenuAction::None),
        }
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

/// Tray icon wrapper
pub struct TrayIcon<T: TrayApi> {
    api: T,
}

impl<T: TrayApi> TrayIcon<T> {
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
        self.api.show_menu().await
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
    tray: Option<TrayIcon<T>>,
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

        let icon = TrayIcon::new(T::default());
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

/// Run the tray message loop (blocking, for main thread)
/// Following Windows design: this runs in the main thread and handles
/// tray messages directly through callbacks
pub async fn run_tray_message_loop<F>(
    _command_receiver: Receiver<AppCommand>,
    shutdown_flag: Arc<AtomicBool>,
    handler: F,
) -> Result<()>
where
    F: Fn(AppCommand) + Send + 'static,
{
    info!("Starting tray message loop (macOS)");

    // Set the global command callback
    // This callback will be invoked directly by menu item click handlers
    {
        let mut guard = GLOBAL_COMMAND_CALLBACK
            .lock()
            .map_err(|_| anyhow!("Failed to lock global command callback"))?;
        *guard = Some(Box::new(handler));
    }

    // On macOS, NSStatusBar menu callbacks run on the main thread
    // We just need to wait for shutdown signal
    while !shutdown_flag.load(Ordering::SeqCst) {
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    // Clear the callback on shutdown
    {
        let mut guard = GLOBAL_COMMAND_CALLBACK
            .lock()
            .map_err(|_| anyhow!("Failed to lock global command callback"))?;
        *guard = None;
    }

    info!("Tray message loop ended");
    Ok(())
}

/// Stop the tray loop
pub fn stop_tray(shutdown_flag: Arc<AtomicBool>) {
    shutdown_flag.store(true, Ordering::SeqCst);
    debug!("Stop signal sent to tray loop");
}

/// Set global command callback (called by daemon during startup)
/// This follows Windows design pattern
pub fn set_global_command_callback<F>(callback: F)
where
    F: Fn(AppCommand) + Send + 'static,
{
    if let Ok(mut guard) = GLOBAL_COMMAND_CALLBACK.lock() {
        *guard = Some(Box::new(callback));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_real_tray_api_creation() {
        let api = RealTrayApi::new();
        assert!(!api.is_registered());
    }

    #[tokio::test]
    async fn test_real_tray_api_lifecycle() {
        let api = RealTrayApi::new();
        assert!(!api.is_registered());

        // Note: Actual registration requires a running NSApplication
        // which is not available in tests
    }

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
    async fn test_mock_multiple_notifications() {
        let api = MockTrayApi::new();
        api.show_notification("A", "1").await.unwrap();
        api.show_notification("B", "2").await.unwrap();
        api.show_notification("C", "3").await.unwrap();

        let notifications = api.get_notifications();
        assert_eq!(notifications.len(), 3);
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
    async fn test_mock_menu_action() {
        let api = MockTrayApi::new();
        api.push_menu_action(MenuAction::Reload);

        let action = api.show_menu().await.unwrap();
        assert_eq!(action, MenuAction::Reload);

        // No more actions - should return None
        let empty = api.show_menu().await.unwrap();
        assert_eq!(empty, MenuAction::None);
    }

    #[tokio::test]
    async fn test_tray_icon_lifecycle() {
        let api = MockTrayApi::new();
        let icon = TrayIcon::new(api);

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

    #[tokio::test]
    async fn test_tray_manager_double_start() {
        let (mut mgr, _) = MockTrayManager::new();
        mgr.start().await.unwrap();
        mgr.start().await.unwrap(); // Should be no-op
        assert!(mgr.is_running());
        mgr.stop().await.unwrap();
    }

    #[test]
    fn test_stop_tray() {
        let flag = Arc::new(AtomicBool::new(false));
        stop_tray(flag.clone());
        assert!(flag.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_run_tray_message_loop() {
        let (_, receiver) = channel::<AppCommand>();
        let flag = Arc::new(AtomicBool::new(false));
        let flag_clone = flag.clone();

        let received_commands = Arc::new(std::sync::Mutex::new(Vec::new()));
        let received_commands_clone = received_commands.clone();

        // Set flag after a short delay to exit the loop
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
            flag_clone.store(true, Ordering::SeqCst);
        });

        run_tray_message_loop(receiver, flag, move |cmd| {
            received_commands_clone.lock().unwrap().push(cmd);
        })
        .await
        .unwrap();

        // Test that the loop exited properly
        assert!(received_commands.lock().unwrap().is_empty());
    }

    #[test]
    fn test_set_global_command_callback() {
        let received = Arc::new(std::sync::Mutex::new(None));
        let received_clone = received.clone();

        set_global_command_callback(move |cmd| {
            *received_clone.lock().unwrap() = Some(cmd);
        });

        // Simulate menu click by invoking callback directly
        if let Ok(guard) = GLOBAL_COMMAND_CALLBACK.lock() {
            if let Some(ref callback) = *guard {
                callback(AppCommand::ReloadConfig);
            }
        }

        assert_eq!(
            received.lock().unwrap().clone(),
            Some(AppCommand::ReloadConfig)
        );
    }
}
