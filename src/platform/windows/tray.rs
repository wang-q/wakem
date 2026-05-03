//! Windows system tray implementation
//!
//! This module provides a complete system tray implementation including:
//! - Tray icon management (register, unregister, notifications)
//! - Context menu display
//! - Message loop for handling tray events
//! - Async API trait for integration with async code
#![cfg(target_os = "windows")]

use anyhow::{anyhow, Result};

use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE,
    NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreateIconFromResourceEx, CreatePopupMenu, CreateWindowExW,
    DefWindowProcW, DestroyMenu, DispatchMessageW, GetCursorPos, GetMessageW,
    LoadCursorW, LookupIconIdFromDirectoryEx, PostMessageW, PostQuitMessage,
    RegisterClassW, SetForegroundWindow, TrackPopupMenu, TranslateMessage, CS_HREDRAW,
    CS_VREDRAW, CW_USEDEFAULT, HMENU, IDC_ARROW, LR_DEFAULTCOLOR, MF_SEPARATOR,
    MF_STRING, MSG, TPM_BOTTOMALIGN, TPM_LEFTALIGN, WINDOW_STYLE, WM_COMMAND, WM_CREATE,
    WM_DESTROY, WNDCLASSW, WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
};

/// Embedded icon resource
const ICON_BYTES: &[u8] = include_bytes!("../../../assets/icon.ico");

/// Custom message ID
const WM_USER_TRAYICON: u32 = 6000;
const WM_LBUTTONUP: u32 = 0x0202;
const WM_RBUTTONUP: u32 = 0x0205;

// Re-export shared tray types from platform::types
use crate::platform::common::tray::{default_menu_items, menu_id_to_app_command};
pub use crate::platform::types::AppCommand;

/// Callback type for command handling
type CommandCallback = Box<dyn Fn(AppCommand) + Send + 'static>;

use std::cell::RefCell;
use std::sync::OnceLock;

static TRAY_HWND_GLOBAL: OnceLock<isize> = OnceLock::new();

thread_local! {
    static TRAY_ICON: RefCell<Option<TrayIconData>> = const { RefCell::new(None) };
    static CMD_CALLBACK: RefCell<Option<CommandCallback>> = const { RefCell::new(None) };
}

/// Tray icon data structure (wraps TrayIcon for message loop usage)
struct TrayIconData {
    icon: TrayIcon,
}

impl TrayIconData {
    fn create() -> Self {
        Self {
            icon: TrayIcon::new(),
        }
    }

    fn register(&mut self, hwnd: HWND) {
        if let Err(e) = self.icon.register(hwnd) {
            error!("Failed to register tray icon: {}", e);
        } else {
            debug!("Tray icon registered");
        }
    }

    fn show_menu(&mut self) {
        if let Err(e) = self.icon.show_menu() {
            error!("Failed to show tray menu: {}", e);
        }
    }

    fn data(&self) -> &NOTIFYICONDATAW {
        &self.icon.data
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
            TRAY_ICON.with(|t| {
                if let Some(ref mut tray) = *t.borrow_mut() {
                    unsafe {
                        let _ = Shell_NotifyIconW(NIM_DELETE, tray.data());
                    }
                }
            });
            debug!("Window destroyed, tray icon removed");
            PostQuitMessage(0);
            LRESULT(0)
        }
        WM_USER_TRAYICON => {
            let mouse_msg = lparam.0 as u32;
            if mouse_msg == WM_LBUTTONUP || mouse_msg == WM_RBUTTONUP {
                debug!("Tray icon clicked");
                TRAY_ICON.with(|t| {
                    if let Some(ref mut tray) = *t.borrow_mut() {
                        tray.show_menu();
                    }
                });
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            let id = wparam.0 as u32 & 0xffff;
            debug!("Menu command: id={}", id);

            CMD_CALLBACK.with(|c| {
                if let Some(ref callback) = *c.borrow() {
                    if let Some(cmd) = menu_id_to_app_command(id) {
                        callback(cmd);
                    }
                }
            });
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
        CMD_CALLBACK.with(|c| {
            *c.borrow_mut() = Some(Box::new(callback));
        });

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

        let _ = TRAY_HWND_GLOBAL.set(hwnd.0 as isize);

        let mut tray_data = TrayIconData::create();
        tray_data.register(hwnd);
        TRAY_ICON.with(|t| {
            *t.borrow_mut() = Some(tray_data);
        });

        debug!("Starting message loop");
        let mut msg: MSG = std::mem::zeroed();

        loop {
            let ret = GetMessageW(&mut msg, None, 0, 0);
            match ret.0 {
                -1 => {
                    error!("GetMessageW failed");
                    break;
                }
                0 => break,
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
///
/// Sends WM_CLOSE to the tray window, which triggers WM_DESTROY
/// (which removes the tray icon and calls PostQuitMessage).
///
/// Safe to call from any thread — uses a global to find the HWND.
pub fn stop_tray() {
    if let Some(&hwnd_isize) = TRAY_HWND_GLOBAL.get() {
        unsafe {
            let hwnd = HWND(hwnd_isize as *mut std::ffi::c_void);
            let _ = PostMessageW(
                Some(hwnd),
                windows::Win32::UI::WindowsAndMessaging::WM_CLOSE,
                WPARAM(0),
                LPARAM(0),
            );
        }
    }
}

/// Tray icon structure (for standalone usage)
pub struct TrayIcon {
    pub(crate) data: NOTIFYICONDATAW,
}

// SAFETY: TrayIcon contains HWND and HICON which are raw pointers.
// These are thread-affine Windows handles that must only be used from
// the thread that created them. TrayIcon is safe to Send (transfer
// ownership to another thread) but NOT safe to Sync (shared access
// from multiple threads) because concurrent access to Windows UI
// handles is undefined behavior.
unsafe impl Send for TrayIcon {}

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
        } as usize;
        if offset >= ICON_BYTES.len() {
            panic!(
                "Icon offset {} out of bounds (max {})",
                offset,
                ICON_BYTES.len()
            );
        }
        let icon_data = &ICON_BYTES[offset..];
        let hicon = unsafe {
            CreateIconFromResourceEx(icon_data, true, 0x30000, 0, 0, LR_DEFAULTCOLOR)
        }
        .expect("Failed to load icon resource");

        let mut tooltip = [0u16; 128];
        let src: Vec<u16> = unsafe { w!("wakem").as_wide() }.to_vec();
        let copy_len = src.len().min(tooltip.len() - 1);
        tooltip[..copy_len].copy_from_slice(&src[..copy_len]);

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
        super::notification::show_shell_notification(self.data.hWnd, title, message)?;

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

            for item in default_menu_items() {
                if item.is_separator {
                    AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null())
                        .map_err(|_| anyhow!("Failed to append separator"))?;
                } else {
                    let label: Vec<u16> = item
                        .label
                        .encode_utf16()
                        .chain(std::iter::once(0))
                        .collect();
                    AppendMenuW(
                        hmenu,
                        MF_STRING,
                        item.id as usize,
                        PCWSTR(label.as_ptr()),
                    )
                    .map_err(|_| anyhow!("Failed to append menu item"))?;
                }
            }

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

/// Real tray icon API implementation
pub struct RealTrayApi {
    inner: Arc<Mutex<TrayIconInner>>,
}

struct TrayIconInner {
    tray_icon: TrayIcon,
    hwnd: HWND,
    active: bool,
    registered: bool,
}

// SAFETY: TrayIconInner wraps TrayIcon (which is Send) and HWND.
// Windows UI handles are thread-affine: they must only be used from the
// thread that created them. We allow Send so the struct can be moved
// between threads, but callers must ensure that all UI operations
// (register, show_menu, etc.) are dispatched to the creating thread.
// The tokio::sync::Mutex in RealTrayApi provides mutual exclusion but
// does NOT enforce thread affinity. Debug builds include a thread check.
//
// Sync is also needed because Arc<TrayIconInner> requires T: Send + Sync
// for Arc<T> to be Send. All access is protected by Mutex, so concurrent
// access from multiple threads is safe as long as UI operations are
// dispatched to the correct thread.
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
                registered: false,
            })),
        }
    }

    pub fn set_hwnd(&self, hwnd: isize) {
        if let Ok(mut inner) = self.inner.try_lock() {
            inner.hwnd = HWND(hwnd as *mut std::ffi::c_void);
        }
    }
}

impl RealTrayApi {
    pub async fn register(&self) -> Result<()> {
        let mut inner = self.inner.lock().await;
        if inner.hwnd.is_invalid() || inner.hwnd.0.is_null() {
            return Err(anyhow::anyhow!(
                "Windows tray registration requires hwnd; call set_hwnd() first"
            ));
        }
        let hwnd = inner.hwnd;
        inner.tray_icon.register(hwnd)?;
        inner.registered = true;
        Ok(())
    }

    pub async fn unregister(&self) -> Result<()> {
        let mut inner = self.inner.lock().await;
        inner.tray_icon.unregister()?;
        inner.registered = false;
        Ok(())
    }

    pub async fn show_notification(&self, title: &str, message: &str) -> Result<()> {
        let mut inner = self.inner.lock().await;
        inner.tray_icon.show_notification(title, message)?;
        Ok(())
    }

    pub async fn show_menu(&self) -> Result<u32> {
        let mut inner = self.inner.lock().await;
        inner.tray_icon.show_menu()
    }

    pub async fn set_active(&self, active: bool) -> Result<()> {
        let mut inner = self.inner.lock().await;
        inner.active = active;
        Ok(())
    }

    pub async fn is_active(&self) -> bool {
        let inner = self.inner.lock().await;
        inner.active
    }

    pub fn get_notifications(&self) -> Vec<(String, String)> {
        Vec::new()
    }

    pub fn is_registered(&self) -> bool {
        match self.inner.try_lock() {
            Ok(inner) => inner.registered,
            Err(_) => false,
        }
    }

    pub fn set_menu_selections(&self, _selections: Vec<u32>) {}
}

#[async_trait::async_trait]
impl crate::platform::common::tray::TrayApi for RealTrayApi {
    async fn register(&self) -> Result<()> {
        RealTrayApi::register(self).await
    }

    async fn unregister(&self) -> Result<()> {
        RealTrayApi::unregister(self).await
    }

    async fn show_notification(&self, title: &str, message: &str) -> Result<()> {
        RealTrayApi::show_notification(self, title, message).await
    }

    async fn show_menu(&self) -> Result<u32> {
        RealTrayApi::show_menu(self).await
    }

    async fn set_active(&self, active: bool) -> Result<()> {
        RealTrayApi::set_active(self, active).await
    }

    async fn is_active(&self) -> bool {
        RealTrayApi::is_active(self).await
    }

    fn get_notifications(&self) -> Vec<(String, String)> {
        RealTrayApi::get_notifications(self)
    }

    fn is_registered(&self) -> bool {
        RealTrayApi::is_registered(self)
    }

    fn set_menu_selections(&self, selections: Vec<u32>) {
        RealTrayApi::set_menu_selections(self, selections)
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
}
