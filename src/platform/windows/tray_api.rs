use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Tray icon operation trait - used to abstract Windows API calls for easier testing
#[async_trait]
#[allow(dead_code)]
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

/// Real tray icon API implementation
#[allow(dead_code)]
pub struct RealTrayApi {
    inner: Arc<Mutex<TrayIconInner>>,
}

struct TrayIconInner {
    #[allow(dead_code)]
    tray_icon: super::tray::TrayIcon,
    #[allow(dead_code)]
    hwnd: isize,
    #[allow(dead_code)]
    active: bool,
}

impl Default for RealTrayApi {
    fn default() -> Self {
        Self::new()
    }
}

impl RealTrayApi {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(TrayIconInner {
                tray_icon: super::tray::TrayIcon::new(),
                hwnd: 0,
                active: true,
            })),
        }
    }

    #[allow(dead_code)]
    pub fn with_icon_path(icon_path: Option<String>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(TrayIconInner {
                tray_icon: super::tray::TrayIcon::with_icon_path(icon_path),
                hwnd: 0,
                active: true,
            })),
        }
    }
}

#[async_trait]
impl TrayApi for RealTrayApi {
    async fn register(&self, hwnd: isize) -> Result<()> {
        let mut inner = self.inner.lock().await;
        inner.hwnd = hwnd;
        // Note: Actual TrayIcon::register requires HWND type
        // Here we use type conversion for compatibility
        let hwnd = windows::Win32::Foundation::HWND(hwnd);
        inner.tray_icon.register(hwnd)?;
        Ok(())
    }

    async fn unregister(&self) -> Result<()> {
        let mut inner = self.inner.lock().await;
        inner.tray_icon.unregister()?;
        Ok(())
    }

    async fn show_notification(&self, title: &str, message: &str) -> Result<()> {
        let mut inner = self.inner.lock().await;
        inner.tray_icon.show_notification(title, message)?;
        Ok(())
    }

    async fn show_menu(&self) -> Result<u32> {
        let inner = self.inner.lock().await;
        inner.tray_icon.show_menu()
    }

    async fn set_active(&self, active: bool) -> Result<()> {
        let mut inner = self.inner.lock().await;
        inner.active = active;
        Ok(())
    }

    async fn is_active(&self) -> bool {
        let inner = self.inner.lock().await;
        inner.active
    }

    fn get_notifications(&self) -> Vec<(String, String)> {
        // Real implementation doesn't store notifications
        Vec::new()
    }

    fn is_registered(&self) -> bool {
        // Real implementation cannot easily detect, return true
        true
    }

    fn set_menu_selections(&self, _selections: Vec<u32>) {
        // Real implementation doesn't support preset
    }
}

/// Mock tray icon API implementation - for testing
#[allow(dead_code)]
pub struct MockTrayApi {
    state: Arc<Mutex<MockTrayState>>,
}

#[derive(Default)]
#[allow(dead_code)]
struct MockTrayState {
    registered: bool,
    hwnd: isize,
    active: bool,
    notifications: Vec<(String, String)>,
    menu_selections: Vec<u32>,
    menu_index: usize,
}

impl Default for MockTrayApi {
    fn default() -> Self {
        Self::new()
    }
}

impl MockTrayApi {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(MockTrayState {
                registered: false,
                hwnd: 0,
                active: true, // Default active
                notifications: Vec::new(),
                menu_selections: Vec::new(),
                menu_index: 0,
            })),
        }
    }
}

#[async_trait]
impl TrayApi for MockTrayApi {
    async fn register(&self, hwnd: isize) -> Result<()> {
        let mut state = self.state.lock().await;
        state.registered = true;
        state.hwnd = hwnd;
        Ok(())
    }

    async fn unregister(&self) -> Result<()> {
        let mut state = self.state.lock().await;
        state.registered = false;
        Ok(())
    }

    async fn show_notification(&self, title: &str, message: &str) -> Result<()> {
        let mut state = self.state.lock().await;
        state
            .notifications
            .push((title.to_string(), message.to_string()));
        Ok(())
    }

    async fn show_menu(&self) -> Result<u32> {
        let mut state = self.state.lock().await;
        if state.menu_index < state.menu_selections.len() {
            let selection = state.menu_selections[state.menu_index];
            state.menu_index += 1;
            Ok(selection)
        } else {
            Ok(0) // No selection
        }
    }

    async fn set_active(&self, active: bool) -> Result<()> {
        let mut state = self.state.lock().await;
        state.active = active;
        Ok(())
    }

    async fn is_active(&self) -> bool {
        let state = self.state.lock().await;
        state.active
    }

    fn get_notifications(&self) -> Vec<(String, String)> {
        // Use try_lock to get notifications
        let state = self.state.clone();
        let result = state.try_lock();
        match result {
            Ok(guard) => guard.notifications.clone(),
            Err(_) => Vec::new(),
        }
    }

    fn is_registered(&self) -> bool {
        let state = self.state.clone();
        let result = state.try_lock();
        match result {
            Ok(guard) => guard.registered,
            Err(_) => false,
        }
    }

    fn set_menu_selections(&self, selections: Vec<u32>) {
        let state = self.state.clone();
        let result = state.try_lock();
        if let Ok(mut guard) = result {
            guard.menu_selections = selections;
            guard.menu_index = 0;
        }
    }
}

/// Tray icon manager
#[allow(dead_code)]
pub struct TrayManager<T: TrayApi> {
    pub api: T,
}

#[allow(dead_code)]
impl<T: TrayApi> TrayManager<T> {
    pub fn new(api: T) -> Self {
        Self { api }
    }

    /// Initialize tray icon
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

    /// Show menu and handle selection
    pub async fn show_context_menu(&self) -> Result<MenuAction> {
        let selection = self.api.show_menu().await?;
        Ok(match selection {
            super::tray::IDM_TOGGLE_ACTIVE => MenuAction::ToggleActive,
            super::tray::IDM_RELOAD => MenuAction::Reload,
            super::tray::IDM_OPEN_CONFIG => MenuAction::OpenConfig,
            super::tray::IDM_EXIT => MenuAction::Exit,
            _ => MenuAction::None,
        })
    }

    /// Toggle active status
    pub async fn toggle_active(&self) -> Result<bool> {
        let current = self.api.is_active().await;
        let new_state = !current;
        self.api.set_active(new_state).await?;
        Ok(new_state)
    }

    /// Get active status
    pub async fn is_active(&self) -> bool {
        self.api.is_active().await
    }
}

/// Menu action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum MenuAction {
    None,
    ToggleActive,
    Reload,
    OpenConfig,
    Exit,
}

/// Backward compatible type alias
#[allow(dead_code)]
pub type TrayIcon = super::tray::TrayIcon;
