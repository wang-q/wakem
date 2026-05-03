//! Windows notification service using tray balloon notifications
#![cfg(target_os = "windows")]

use crate::platform::traits::NotificationService;
use crate::platform::types::NotificationInitContext;
use anyhow::Result;
use std::sync::Arc;

pub struct WindowsNotificationService {
    hwnd: Arc<std::sync::RwLock<Option<isize>>>,
}

impl WindowsNotificationService {
    pub fn new() -> Self {
        Self {
            hwnd: Arc::new(std::sync::RwLock::new(None)),
        }
    }

    pub fn set_hwnd(&self, hwnd_value: isize) {
        if let Ok(mut guard) = self.hwnd.write() {
            *guard = Some(hwnd_value);
        }
    }

    pub fn show(&self, title: &str, message: &str) -> Result<()> {
        let hwnd_value = self.hwnd.read().ok().and_then(|guard| *guard);

        let Some(hwnd_value) = hwnd_value else {
            tracing::debug!("Message window not registered, skipping notification");
            return Ok(());
        };

        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::Shell::{
            NIF_INFO, NIM_MODIFY, NOTIFYICONDATAW, NOTIFY_ICON_INFOTIP_FLAGS,
        };

        let hwnd = HWND(hwnd_value as *mut std::ffi::c_void);

        let mut nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: 1,
            uFlags: NIF_INFO,
            ..Default::default()
        };

        let title_wide: Vec<u16> =
            title.encode_utf16().chain(std::iter::once(0)).collect();
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

    pub fn initialize(&self, ctx: &NotificationInitContext) {
        if let Some(handle) = ctx.native_handle {
            self.set_hwnd(handle as isize);
        }
    }
}

impl Default for WindowsNotificationService {
    fn default() -> Self {
        Self::new()
    }
}

impl NotificationService for WindowsNotificationService {
    fn show(&self, title: &str, message: &str) -> Result<()> {
        self.show(title, message)
    }

    fn initialize(&self, ctx: &NotificationInitContext) {
        self.initialize(ctx)
    }
}
