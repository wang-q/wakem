//! Windows system tray implementation
//!
//! This module provides a complete system tray implementation including:
//! - Tray icon management (register, unregister, notifications)
//! - Context menu display
//! - Message loop for handling tray events
//! - Async API trait for integration with async code
#![cfg(target_os = "windows")]

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE,
    NIM_MODIFY, NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreateIconFromResourceEx, CreatePopupMenu, CreateWindowExW,
    DefWindowProcW, DestroyMenu, DispatchMessageW, GetCursorPos, GetMessageW,
    LoadCursorW, LookupIconIdFromDirectoryEx, PostQuitMessage, RegisterClassW,
    SetForegroundWindow, TrackPopupMenu, TranslateMessage, CS_HREDRAW, CS_VREDRAW,
    CW_USEDEFAULT, HMENU, IDC_ARROW, LR_DEFAULTCOLOR, MF_SEPARATOR, MF_STRING, MSG,
    TPM_BOTTOMALIGN, TPM_LEFTALIGN, WINDOW_STYLE, WM_COMMAND, WM_CREATE, WM_DESTROY,
    WNDCLASSW, WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
};

/// Embedded icon resource
const ICON_BYTES: &[u8] = include_bytes!("../../../assets/icon.ico");

/// Custom message ID
const WM_USER_TRAYICON: u32 = 6000;
const WM_LBUTTONUP: u32 = 0x0202;
const WM_RBUTTONUP: u32 = 0x0205;

/// Menu item IDs
pub const IDM_TOGGLE_ACTIVE: u32 = 100;
pub const IDM_RELOAD: u32 = 101;
pub const IDM_OPEN_CONFIG: u32 = 102;
pub const IDM_EXIT: u32 = 103;

/// Application commands
#[derive(Debug, Clone, Copy)]
pub enum AppCommand {
    ToggleActive,
    ReloadConfig,
    OpenConfigFolder,
    Exit,
}

/// Menu action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    None,
    ToggleActive,
    Reload,
    OpenConfig,
    Exit,
}

/// Callback type for command handling
type CommandCallback = Box<dyn Fn(AppCommand) + Send + 'static>;

/// Global tray icon storage
static mut TRAY_ICON: Option<TrayIconData> = None;
static mut CMD_CALLBACK: Option<CommandCallback> = None;
static mut TRAY_HWND: Option<HWND> = None;

/// Tray icon data structure
struct TrayIconData {
    data: NOTIFYICONDATAW,
}

impl TrayIconData {
    fn create() -> Self {
        let offset = unsafe {
            LookupIconIdFromDirectoryEx(ICON_BYTES.as_ptr(), true, 0, 0, LR_DEFAULTCOLOR)
        };
        let icon_data = &ICON_BYTES[offset as usize..];
        let hicon = unsafe {
            CreateIconFromResourceEx(icon_data, true, 0x30000, 0, 0, LR_DEFAULTCOLOR)
        }
        .expect("Failed to load icon");

        let mut tooltip: Vec<u16> = unsafe { w!("wakem").as_wide() }.to_vec();
        tooltip.resize(128, 0);
        tooltip.pop();
        tooltip.push(0);
        let tooltip: [u16; 128] = tooltip.try_into().unwrap();

        Self {
            data: NOTIFYICONDATAW {
                uID: WM_USER_TRAYICON,
                uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
                uCallbackMessage: WM_USER_TRAYICON,
                hIcon: hicon,
                szTip: tooltip,
                ..Default::default()
            },
        }
    }

    fn register(&mut self, hwnd: HWND) {
        self.data.hWnd = hwnd;
        unsafe {
            let _ = Shell_NotifyIconW(NIM_ADD, &self.data);
        }
        debug!("Tray icon registered");
    }

    fn show_menu(&self) {
        let hwnd = self.data.hWnd;
        let mut cursor = POINT::default();

        unsafe {
            let _ = SetForegroundWindow(hwnd);
            let _ = GetCursorPos(&mut cursor);
            let hmenu: HMENU = CreatePopupMenu().unwrap();

            // Add menu items
            let _ = AppendMenuW(
                hmenu,
                MF_STRING,
                IDM_TOGGLE_ACTIVE as usize,
                w!("Enable/Disable"),
            );
            let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null());
            let _ =
                AppendMenuW(hmenu, MF_STRING, IDM_RELOAD as usize, w!("Reload Config"));
            let _ = AppendMenuW(
                hmenu,
                MF_STRING,
                IDM_OPEN_CONFIG as usize,
                w!("Open Config Folder"),
            );
            let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null());
            let _ = AppendMenuW(hmenu, MF_STRING, IDM_EXIT as usize, w!("Exit"));

            let _ = TrackPopupMenu(
                hmenu,
                TPM_LEFTALIGN | TPM_BOTTOMALIGN,
                cursor.x,
                cursor.y,
                None,
                hwnd,
                None,
            );

            let _ = DestroyMenu(hmenu);
        }
        debug!("Menu shown");
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CREATE => {
            debug!("Window created");
            LRESULT(0)
        }
        WM_DESTROY => {
            debug!("Window destroyed");
            PostQuitMessage(0);
            LRESULT(0)
        }
        WM_USER_TRAYICON => {
            let mouse_msg = lparam.0 as u32;
            if mouse_msg == WM_LBUTTONUP || mouse_msg == WM_RBUTTONUP {
                debug!("Tray icon clicked");
                if let Some(ref tray) = TRAY_ICON {
                    tray.show_menu();
                }
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            let id = wparam.0 as u32 & 0xffff;
            debug!("Menu command: id={}", id);

            if let Some(ref callback) = CMD_CALLBACK {
                match id {
                    IDM_TOGGLE_ACTIVE => callback(AppCommand::ToggleActive),
                    IDM_RELOAD => callback(AppCommand::ReloadConfig),
                    IDM_OPEN_CONFIG => callback(AppCommand::OpenConfigFolder),
                    IDM_EXIT => callback(AppCommand::Exit),
                    _ => {}
                }
            }
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

/// Run the tray icon message loop. Blocks until exit.
///
/// This function creates the window and tray icon in the current thread,
/// then runs the message loop. It returns when the user selects "Exit".
pub fn run_tray_message_loop<F>(callback: F) -> Result<()>
where
    F: Fn(AppCommand) + Send + 'static,
{
    unsafe {
        // Store callback
        CMD_CALLBACK = Some(Box::new(callback));

        // Create window
        let class_name = w!("WakemTrayWindow");
        let hinstance = GetModuleHandleW(None)?;
        let hcursor = LoadCursorW(None, IDC_ARROW)?;

        let wnd_class = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            hInstance: hinstance.into(),
            lpszClassName: class_name,
            hCursor: hcursor,
            style: CS_HREDRAW | CS_VREDRAW,
            ..Default::default()
        };

        let atom = RegisterClassW(&wnd_class);
        if atom == 0 {
            let err = windows::Win32::Foundation::GetLastError();
            return Err(anyhow!("Failed to register window class: {:?}", err));
        }
        debug!("Window class registered: {}", atom);

        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
            PCWSTR(atom as _),
            w!("wakem"),
            WINDOW_STYLE(0),
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            None,
            None,
            Some(hinstance.into()),
            None,
        )?;

        debug!("Window created: {:?}", hwnd);

        // Store window handle for later use
        TRAY_HWND = Some(hwnd);

        // Create and register tray icon
        TRAY_ICON = Some(TrayIconData::create());
        if let Some(ref mut tray) = TRAY_ICON {
            tray.register(hwnd);
        }

        // Message loop
        debug!("Starting message loop");
        let mut msg: MSG = std::mem::zeroed();

        loop {
            let ret = GetMessageW(&mut msg, None, 0, 0);
            match ret.0 {
                -1 => {
                    error!("GetMessageW failed");
                    break;
                }
                0 => break, // WM_QUIT
                _ => {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
        }

        debug!("Message loop ended");
        Ok(())
    }
}

/// Post a quit message to stop the message loop
pub fn stop_tray() {
    unsafe {
        if let Some(hwnd) = TRAY_HWND {
            let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                Some(hwnd),
                WM_DESTROY,
                WPARAM(0),
                LPARAM(0),
            );
        }
    }
}

/// Tray icon structure (for standalone usage)
pub struct TrayIcon {
    data: NOTIFYICONDATAW,
}

// SAFETY: TrayIcon contains HWND and HICON which are raw pointers.
// They are only used from the main thread and are safe to send/sync
// as long as they are not accessed concurrently.
unsafe impl Send for TrayIcon {}
unsafe impl Sync for TrayIcon {}

impl TrayIcon {
    /// Create new tray icon
    pub fn new() -> Self {
        Self::create()
    }

    /// Create tray icon
    fn create() -> Self {
        let data = Self::create_nid();
        Self { data }
    }

    /// Create NOTIFYICONDATAW
    fn create_nid() -> NOTIFYICONDATAW {
        let offset = unsafe {
            LookupIconIdFromDirectoryEx(ICON_BYTES.as_ptr(), true, 0, 0, LR_DEFAULTCOLOR)
        };
        let icon_data = &ICON_BYTES[offset as usize..];
        let hicon = unsafe {
            CreateIconFromResourceEx(icon_data, true, 0x30000, 0, 0, LR_DEFAULTCOLOR)
        }
        .expect("Failed to load icon resource");

        let mut tooltip: Vec<u16> = unsafe { w!("wakem").as_wide() }.to_vec();
        tooltip.resize(128, 0);
        tooltip.pop();
        tooltip.push(0);
        let tooltip: [u16; 128] = tooltip.try_into().unwrap();

        NOTIFYICONDATAW {
            uID: WM_USER_TRAYICON,
            uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
            uCallbackMessage: WM_USER_TRAYICON,
            hIcon: hicon,
            szTip: tooltip,
            ..Default::default()
        }
    }

    /// Register tray icon
    pub fn register(&mut self, hwnd: HWND) -> Result<()> {
        self.data.hWnd = hwnd;
        unsafe {
            Shell_NotifyIconW(NIM_ADD, &self.data)
                .ok()
                .map_err(|e| anyhow!("Failed to add tray icon: {}", e))
        }
    }

    /// Unregister tray icon
    pub fn unregister(&mut self) -> Result<()> {
        unsafe {
            Shell_NotifyIconW(NIM_DELETE, &self.data)
                .ok()
                .map_err(|e| anyhow!("Failed to delete tray icon: {}", e))
        }
    }

    /// Show balloon notification
    pub fn show_notification(&mut self, title: &str, message: &str) -> Result<()> {
        self.data.uFlags = NIF_INFO;

        // Set title
        let title_wide: Vec<u16> = title.encode_utf16().collect();
        let title_len = title_wide.len().min(63);
        self.data.szInfoTitle[..title_len].copy_from_slice(&title_wide[..title_len]);
        self.data.szInfoTitle[title_len] = 0;

        // Set message
        let msg_wide: Vec<u16> = message.encode_utf16().collect();
        let msg_len = msg_wide.len().min(255);
        self.data.szInfo[..msg_len].copy_from_slice(&msg_wide[..msg_len]);
        self.data.szInfo[msg_len] = 0;

        unsafe {
            Shell_NotifyIconW(NIM_MODIFY, &self.data)
                .ok()
                .map_err(|e| anyhow!("Failed to show notification: {}", e))?;
        }

        // Restore flags
        self.data.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;

        Ok(())
    }

    /// Show context menu
    pub fn show_menu(&mut self) -> Result<u32> {
        unsafe {
            SetForegroundWindow(self.data.hWnd)
                .ok()
                .map_err(|e| anyhow!("Failed to set foreground window: {}", e))?;

            let mut cursor = POINT::default();
            GetCursorPos(&mut cursor)
                .map_err(|e| anyhow!("Failed to get cursor pos: {}", e))?;

            let hmenu = self.create_menu()?;

            TrackPopupMenu(
                hmenu,
                TPM_LEFTALIGN | TPM_BOTTOMALIGN,
                cursor.x,
                cursor.y,
                None,
                self.data.hWnd,
                None,
            )
            .ok()
            .map_err(|e| anyhow!("Failed to show popup menu: {}", e))?;

            if DestroyMenu(hmenu).is_err() {
                error!("Failed to destroy menu");
            }

            Ok(0)
        }
    }

    /// Create context menu
    fn create_menu(&mut self) -> Result<HMENU> {
        unsafe {
            let hmenu = CreatePopupMenu()
                .map_err(|e| anyhow!("Failed to create menu: {}", e))?;

            // Enable/Disable
            AppendMenuW(
                hmenu,
                MF_STRING,
                IDM_TOGGLE_ACTIVE as usize,
                w!("Enable/Disable"),
            )
            .map_err(|_| anyhow!("Failed to append menu item"))?;

            // Separator
            AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null())
                .map_err(|_| anyhow!("Failed to append separator"))?;

            // Reload config
            AppendMenuW(hmenu, MF_STRING, IDM_RELOAD as usize, w!("Reload Config"))
                .map_err(|_| anyhow!("Failed to append menu item"))?;

            // Open config folder
            AppendMenuW(
                hmenu,
                MF_STRING,
                IDM_OPEN_CONFIG as usize,
                w!("Open Config Folder"),
            )
            .map_err(|_| anyhow!("Failed to append menu item"))?;

            // Separator
            AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null())
                .map_err(|_| anyhow!("Failed to append separator"))?;

            // Exit
            AppendMenuW(hmenu, MF_STRING, IDM_EXIT as usize, w!("Exit"))
                .map_err(|_| anyhow!("Failed to append menu item"))?;

            Ok(hmenu)
        }
    }
}

impl Default for TrayIcon {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for TrayIcon {
    fn drop(&mut self) {
        unsafe {
            let _ = Shell_NotifyIconW(NIM_DELETE, &self.data);
        }
    }
}

/// Tray icon operation trait - used to abstract Windows API calls for easier testing
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

/// Real tray icon API implementation
pub struct RealTrayApi {
    inner: Arc<Mutex<TrayIconInner>>,
}

struct TrayIconInner {
    tray_icon: TrayIcon,
    hwnd: HWND,
    active: bool,
}

// SAFETY: HWND is just a pointer, and we only use it from one thread at a time
unsafe impl Send for TrayIconInner {}
unsafe impl Sync for TrayIconInner {}

impl Default for RealTrayApi {
    fn default() -> Self {
        Self::new()
    }
}

impl RealTrayApi {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(TrayIconInner {
                tray_icon: TrayIcon::new(),
                hwnd: HWND(std::ptr::null_mut()),
                active: true,
            })),
        }
    }
}

#[async_trait]
impl TrayApi for RealTrayApi {
    async fn register(&self, hwnd: isize) -> Result<()> {
        let mut inner = self.inner.lock().await;
        let hwnd_ptr = HWND(hwnd as *mut std::ffi::c_void);
        inner.hwnd = hwnd_ptr;
        inner.tray_icon.register(hwnd_ptr)?;
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
        let mut inner = self.inner.lock().await;
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
        Vec::new()
    }

    fn is_registered(&self) -> bool {
        true
    }

    fn set_menu_selections(&self, _selections: Vec<u32>) {}
}

/// Mock tray icon API implementation - for testing
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
                active: true,
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
            Ok(0)
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
        match self.state.try_lock() {
            Ok(guard) => guard.notifications.clone(),
            Err(_) => Vec::new(),
        }
    }

    fn is_registered(&self) -> bool {
        match self.state.try_lock() {
            Ok(guard) => guard.registered,
            Err(_) => false,
        }
    }

    fn set_menu_selections(&self, selections: Vec<u32>) {
        if let Ok(mut guard) = self.state.try_lock() {
            guard.menu_selections = selections;
            guard.menu_index = 0;
        }
    }
}

/// Tray icon manager
pub struct TrayManager<T: TrayApi> {
    pub api: T,
}

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
            IDM_TOGGLE_ACTIVE => MenuAction::ToggleActive,
            IDM_RELOAD => MenuAction::Reload,
            IDM_OPEN_CONFIG => MenuAction::OpenConfig,
            IDM_EXIT => MenuAction::Exit,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tray_icon_creation() {
        let tray = TrayIcon::new();
        drop(tray);
    }

    #[test]
    fn test_mock_tray_api() {
        let api = MockTrayApi::new();
        assert!(!api.is_registered());
    }

    #[test]
    fn test_menu_action_enum() {
        assert_eq!(MenuAction::None as i32, 0);
        assert_eq!(MenuAction::ToggleActive as i32, 1);
        assert_eq!(MenuAction::Reload as i32, 2);
        assert_eq!(MenuAction::OpenConfig as i32, 3);
        assert_eq!(MenuAction::Exit as i32, 4);
    }
}
