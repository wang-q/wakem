use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

/// 托盘图标操作 trait - 用于抽象 Windows API 调用，便于测试
#[async_trait]
#[allow(dead_code)]
pub trait TrayApi: Send + Sync {
    /// 注册托盘图标
    async fn register(&self, hwnd: isize) -> Result<()>;

    /// 注销托盘图标
    async fn unregister(&self) -> Result<()>;

    /// 显示气泡通知
    async fn show_notification(&self, title: &str, message: &str) -> Result<()>;

    /// 显示右键菜单，返回选中的菜单项 ID
    async fn show_menu(&self) -> Result<u32>;

    /// 设置激活状态
    async fn set_active(&self, active: bool) -> Result<()>;

    /// 获取激活状态
    async fn is_active(&self) -> bool;

    /// 获取已发送的通知（仅用于测试）
    fn get_notifications(&self) -> Vec<(String, String)>;

    /// 检查是否已注册（仅用于测试）
    fn is_registered(&self) -> bool;

    /// 预设菜单选择（仅用于测试）
    fn set_menu_selections(&self, selections: Vec<u32>);
}

/// 真实的托盘图标 API 实现
#[allow(dead_code)]
pub struct RealTrayApi {
    inner: Arc<Mutex<TrayIconInner>>,
}

struct TrayIconInner {
    tray_icon: super::tray::TrayIcon,
    hwnd: isize,
    active: bool,
}

impl RealTrayApi {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(TrayIconInner {
                tray_icon: super::tray::TrayIcon::new(),
                hwnd: 0,
                active: true,
            })),
        }
    }

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
        // 注意：实际的 TrayIcon::register 需要 HWND 类型
        // 这里为了兼容性，我们使用类型转换
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
        // 真实实现不存储通知
        Vec::new()
    }

    fn is_registered(&self) -> bool {
        // 真实实现无法轻易检测，返回 true
        true
    }

    fn set_menu_selections(&self, _selections: Vec<u32>) {
        // 真实实现不支持预设
    }
}

/// Mock 托盘图标 API 实现 - 用于测试
pub struct MockTrayApi {
    state: Arc<Mutex<MockTrayState>>,
}

#[derive(Default)]
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
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(MockTrayState {
                registered: false,
                hwnd: 0,
                active: true, // 默认激活
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
            Ok(0) // 无选择
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
        // 使用 try_lock 获取通知
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

/// 托盘图标管理器
#[allow(dead_code)]
pub struct TrayManager<T: TrayApi> {
    pub api: T,
}

#[allow(dead_code)]
impl<T: TrayApi> TrayManager<T> {
    pub fn new(api: T) -> Self {
        Self { api }
    }

    /// 初始化托盘图标
    pub async fn init(&self, hwnd: isize) -> Result<()> {
        self.api.register(hwnd).await
    }

    /// 清理托盘图标
    pub async fn cleanup(&self) -> Result<()> {
        self.api.unregister().await
    }

    /// 显示通知
    pub async fn notify(&self, title: &str, message: &str) -> Result<()> {
        self.api.show_notification(title, message).await
    }

    /// 显示菜单并处理选择
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

    /// 切换激活状态
    pub async fn toggle_active(&self) -> Result<bool> {
        let current = self.api.is_active().await;
        let new_state = !current;
        self.api.set_active(new_state).await?;
        Ok(new_state)
    }

    /// 获取激活状态
    pub async fn is_active(&self) -> bool {
        self.api.is_active().await
    }
}

/// 菜单操作
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum MenuAction {
    None,
    ToggleActive,
    Reload,
    OpenConfig,
    Exit,
}

/// 向后兼容的类型别名
#[allow(dead_code)]
pub type TrayIcon = super::tray::TrayIcon;
