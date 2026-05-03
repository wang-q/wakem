//! Windows notification service using tray balloon notifications
#![cfg(target_os = "windows")]

use crate::platform::traits::NotificationService;
use anyhow::Result;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Shell::{
    NIF_INFO, NIM_MODIFY, NOTIFYICONDATAW, NOTIFY_ICON_INFOTIP_FLAGS,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, RegisterClassW, CS_HREDRAW, CS_VREDRAW,
    CW_USEDEFAULT, WINDOW_STYLE, WNDCLASSW, WS_EX_LAYERED, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST,
};

pub fn show_shell_notification(hwnd: HWND, title: &str, message: &str) -> Result<()> {
    let mut nid = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: 1,
        uFlags: NIF_INFO,
        ..Default::default()
    };

    let title_wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    let message_wide: Vec<u16> =
        message.encode_utf16().chain(std::iter::once(0)).collect();

    let title_len = title_wide.len().min(64);
    let message_len = message_wide.len().min(256);

    nid.szInfoTitle[..title_len].copy_from_slice(&title_wide[..title_len]);
    nid.szInfo[..message_len].copy_from_slice(&message_wide[..message_len]);

    nid.dwInfoFlags = NOTIFY_ICON_INFOTIP_FLAGS(0);

    unsafe {
        let result = windows::Win32::UI::Shell::Shell_NotifyIconW(NIM_MODIFY, &nid);
        if !result.as_bool() {
            return Err(anyhow::anyhow!("Failed to show notification"));
        }
    }

    tracing::info!("Notification shown: {} - {}", title, message);
    Ok(())
}

fn create_message_window() -> Option<isize> {
    unsafe extern "system" fn default_window_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        DefWindowProcW(hwnd, msg, wparam, lparam)
    }

    unsafe {
        let hinstance = GetModuleHandleW(None).ok()?;

        let class_name: Vec<u16> = "WakemNotifyClass\0".encode_utf16().collect();
        let wc = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(default_window_proc),
            hInstance: hinstance.into(),
            lpszClassName: windows::core::PCWSTR(class_name.as_ptr()),
            ..Default::default()
        };

        let _atom = RegisterClassW(&wc);

        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
            windows::core::PCWSTR(class_name.as_ptr()),
            windows::core::w!("WakemNotify"),
            WINDOW_STYLE(0),
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            None,
            None,
            Some(hinstance.into()),
            None,
        );

        hwnd.ok().map(|h| h.0 as isize)
    }
}

pub struct WindowsNotificationService {
    hwnd: Option<isize>,
}

impl WindowsNotificationService {
    pub fn new() -> Self {
        let hwnd = create_message_window();
        if hwnd.is_some() {
            tracing::info!("Created message window for notifications");
        } else {
            tracing::warn!(
                "Failed to create message window, notifications will be disabled"
            );
        }
        Self { hwnd }
    }

    pub fn show(&self, title: &str, message: &str) -> Result<()> {
        let Some(hwnd_value) = self.hwnd else {
            tracing::debug!("Message window not available, skipping notification");
            return Ok(());
        };

        let hwnd = HWND(hwnd_value as *mut std::ffi::c_void);
        show_shell_notification(hwnd, title, message)
    }
}

impl Default for WindowsNotificationService {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for WindowsNotificationService {
    fn drop(&mut self) {
        if let Some(hwnd_value) = self.hwnd.take() {
            unsafe {
                let hwnd = HWND(hwnd_value as *mut std::ffi::c_void);
                let _ = windows::Win32::UI::WindowsAndMessaging::DestroyWindow(hwnd);
            }
        }
    }
}

impl NotificationService for WindowsNotificationService {
    fn show(&self, title: &str, message: &str) -> Result<()> {
        self.show(title, message)
    }
}
