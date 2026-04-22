//! macOS system tray (status bar) implementation
//!
//! Provides NSStatusBar-based tray icon with menu and notification support.
//! This is the macOS equivalent of Windows system tray.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Application commands sent from tray menu
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppCommand {
    Enable,
    Disable,
    ReloadConfig,
    OpenConfig,
    Exit,
}

/// Menu action results from user interaction
#[derive(Debug, Clone)]
pub enum MenuAction {
    Command(AppCommand),
    None,
}

/// Tray icon trait for abstraction
#[async_trait::async_trait]
pub trait TrayApi: Send + Sync {
    async fn register(&self) -> Result<(), String>;
    async fn unregister(&self) -> Result<(), String>;
    async fn show_notification(&self, title: &str, message: &str) -> Result<(), String>;
    async fn show_menu(&self) -> Result<MenuAction, String>;
    async fn set_tooltip(&self, tooltip: &str) -> Result<(), String>;
    async fn show(&self) -> Result<(), String>;
    async fn hide(&self) -> Result<(), String>;
}

/// Real macOS tray API using NSStatusBar
pub struct RealTrayApi {
    registered: AtomicBool,
    visible: AtomicBool,
}

impl RealTrayApi {
    pub fn new() -> Self {
        Self {
            registered: AtomicBool::new(false),
            visible: AtomicBool::new(true),
        }
    }

    /// Show notification using osascript
    fn show_notification_osascript(
        &self,
        title: &str,
        message: &str,
    ) -> Result<(), String> {
        use std::process::Command;
        let script = format!(
            r#"display notification "{}" with title "{}" sound name "default""#,
            message.replace('"', "\\\""),
            title.replace('"', "\\\"")
        );

        let result = Command::new("osascript").arg("-e").arg(script).output();

        match result {
            Ok(output) if output.status.success() => {
                debug!("Notification shown: {} - {}", title, message);
                Ok(())
            }
            Err(e) => Err(format!("Failed to show notification: {}", e)),
            _ => Err("osascript failed".to_string()),
        }
    }
}

impl Default for RealTrayApi {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl TrayApi for RealTrayApi {
    async fn register(&self) -> Result<(), String> {
        // On macOS, NSStatusBar registration would require Cocoa/objc integration
        // For now, mark as registered and use osascript for notifications
        self.registered.store(true, Ordering::SeqCst);
        debug!("RealTrayApi registered on macOS");
        Ok(())
    }

    async fn unregister(&self) -> Result<(), String> {
        self.registered.store(false, Ordering::SeqCst);
        debug!("RealTrayApi unregistered");
        Ok(())
    }

    async fn show_notification(&self, title: &str, message: &str) -> Result<(), String> {
        self.show_notification_osascript(title, message)
    }

    async fn show_menu(&self) -> Result<MenuAction, String> {
        // In a full implementation, this would use NSMenu with NSStatusItem
        // For now, return None as menu requires event loop integration
        Ok(MenuAction::None)
    }

    async fn set_tooltip(&self, tooltip: &str) -> Result<(), String> {
        debug!("Set tray tooltip: {}", tooltip);
        Ok(())
    }

    async fn show(&self) -> Result<(), String> {
        self.visible.store(true, Ordering::SeqCst);
        debug!("Tray shown");
        Ok(())
    }

    async fn hide(&self) -> Result<(), String> {
        self.visible.store(false, Ordering::SeqCst);
        debug!("Tray hidden");
        Ok(())
    }
}

/// Mock tray API for testing
#[cfg(test)]
pub struct MockTrayApi {
    registered: std::sync::Mutex<bool>,
    visible: std::sync::Mutex<bool>,
    notifications: std::sync::Mutex<std::collections::VecDeque<(String, String)>>,
    tooltip: std::sync::Mutex<String>,
    menu_actions: std::sync::Mutex<std::collections::VecDeque<MenuAction>>,
}

#[cfg(test)]
impl MockTrayApi {
    pub fn new() -> Self {
        Self {
            registered: std::sync::Mutex::new(false),
            visible: std::sync::Mutex::new(true),
            notifications: std::sync::Mutex::new(std::collections::VecDeque::new()),
            tooltip: std::sync::Mutex::new(String::new()),
            menu_actions: std::sync::Mutex::new(std::collections::VecDeque::new()),
        }
    }

    pub fn is_registered(&self) -> bool {
        *self.registered.lock().unwrap()
    }

    pub fn is_visible(&self) -> bool {
        *self.visible.lock().unwrap()
    }

    pub fn get_notifications(&self) -> Vec<(String, String)> {
        self.notifications
            .lock()
            .unwrap()
            .clone()
            .into_iter()
            .collect()
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
    async fn register(&self) -> Result<(), String> {
        *self.registered.lock().unwrap() = true;
        Ok(())
    }

    async fn unregister(&self) -> Result<(), String> {
        *self.registered.lock().unwrap() = false;
        Ok(())
    }

    async fn show_notification(&self, title: &str, message: &str) -> Result<(), String> {
        self.notifications
            .lock()
            .unwrap()
            .push_back((title.to_string(), message.to_string()));
        Ok(())
    }

    async fn show_menu(&self) -> Result<MenuAction, String> {
        match self.menu_actions.lock().unwrap().pop_front() {
            Some(action) => Ok(action),
            None => Ok(MenuAction::None),
        }
    }

    async fn set_tooltip(&self, tooltip: &str) -> Result<(), String> {
        *self.tooltip.lock().unwrap() = tooltip.to_string();
        Ok(())
    }

    async fn show(&self) -> Result<(), String> {
        *self.visible.lock().unwrap() = true;
        Ok(())
    }

    async fn hide(&self) -> Result<(), String> {
        *self.visible.lock().unwrap() = false;
        Ok(())
    }
}

/// Tray icon wrapper
pub struct TrayIcon<T: TrayApi> {
    api: T,
    registered: bool,
}

impl<T: TrayApi> TrayIcon<T> {
    pub fn new(api: T) -> Self {
        Self {
            api,
            registered: false,
        }
    }

    pub async fn register(&mut self) -> Result<(), String> {
        self.api.register().await?;
        self.registered = true;
        Ok(())
    }

    pub async fn unregister(&mut self) -> Result<(), String> {
        self.api.unregister().await?;
        self.registered = false;
        Ok(())
    }

    pub async fn show_notification(
        &self,
        title: &str,
        message: &str,
    ) -> Result<(), String> {
        self.api.show_notification(title, message).await
    }

    pub async fn show_menu(&self) -> Result<MenuAction, String> {
        self.api.show_menu().await
    }

    pub async fn set_tooltip(&self, tooltip: &str) -> Result<(), String> {
        self.api.set_tooltip(tooltip).await
    }

    pub async fn show(&self) -> Result<(), String> {
        self.api.show().await
    }

    pub async fn hide(&self) -> Result<(), String> {
        self.api.hide().await
    }

    pub fn is_registered(&self) -> bool {
        self.registered
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

    pub async fn start(&mut self) -> Result<(), String> {
        if self.running {
            return Ok(());
        }

        let icon = TrayIcon::new(T::default());
        let mut icon_box = Some(icon);

        if let Some(ref mut tray) = icon_box {
            tray.register().await?;
            tray.set_tooltip("wakem - Window Adjust, Keyboard Enhance, Mouse")
                .await?;
        }

        self.tray = icon_box;
        self.running = true;

        info!("TrayManager started");
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<(), String> {
        if !self.running {
            return Ok(());
        }

        if let Some(ref mut tray) = self.tray {
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

    pub async fn notify(&self, title: &str, message: &str) -> Result<(), String> {
        if let Some(ref tray) = self.tray {
            tray.show_notification(title, message).await
        } else {
            warn!("Cannot send notification: tray not initialized");
            Err("Tray not initialized".to_string())
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

/// Run the tray loop (blocking, for main thread)
pub async fn run_tray_loop(
    command_receiver: Receiver<AppCommand>,
    shutdown_flag: Arc<AtomicBool>,
) -> Result<(), String> {
    info!("Starting tray loop");

    while !shutdown_flag.load(Ordering::SeqCst) {
        match command_receiver.try_recv() {
            Ok(cmd) => {
                debug!("Received tray command: {:?}", cmd);
                match cmd {
                    AppCommand::Exit => {
                        info!("Exit command received from tray");
                        shutdown_flag.store(true, Ordering::SeqCst);
                        break;
                    }
                    AppCommand::Enable => {
                        info!("Enable command received from tray");
                    }
                    AppCommand::Disable => {
                        info!("Disable command received from tray");
                    }
                    AppCommand::ReloadConfig => {
                        info!("Reload config command received from tray");
                    }
                    AppCommand::OpenConfig => {
                        info!("Open config command received from tray");
                    }
                }
            }
            Err(_) => {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }
    }

    info!("Tray loop ended");
    Ok(())
}

/// Stop the tray loop
pub fn stop_tray(shutdown_flag: Arc<AtomicBool>) {
    shutdown_flag.store(true, Ordering::SeqCst);
    debug!("Stop signal sent to tray loop");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_real_tray_api_creation() {
        let api = RealTrayApi::new();
        drop(api);
    }

    #[tokio::test]
    async fn test_real_tray_api_lifecycle() {
        let api = RealTrayApi::new();
        assert!(!api.registered.load(Ordering::SeqCst));

        api.register().await.unwrap();
        assert!(api.registered.load(Ordering::SeqCst));

        api.unregister().await.unwrap();
        assert!(!api.registered.load(Ordering::SeqCst));
    }

    #[test]
    fn test_app_command_variants() {
        assert_eq!(AppCommand::Enable, AppCommand::Enable);
        assert_ne!(AppCommand::Enable, AppCommand::Disable);
    }

    #[test]
    fn test_menu_action_variants() {
        let cmd = MenuAction::Command(AppCommand::Exit);
        match cmd {
            MenuAction::Command(c) => assert_eq!(c, AppCommand::Exit),
            MenuAction::None => panic!("Expected Command"),
        }

        let none = MenuAction::None;
        matches!(none, MenuAction::None);
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
        api.push_menu_action(MenuAction::Command(AppCommand::ReloadConfig));

        let action = api.show_menu().await.unwrap();
        match action {
            MenuAction::Command(cmd) => assert_eq!(cmd, AppCommand::ReloadConfig),
            _ => panic!("Expected Command"),
        }

        // No more actions - should return None
        let empty = api.show_menu().await.unwrap();
        matches!(empty, MenuAction::None);
    }

    #[tokio::test]
    async fn test_tray_icon_lifecycle() {
        let api = MockTrayApi::new();
        let mut icon = TrayIcon::<MockTrayApi>::new(api);

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
}
