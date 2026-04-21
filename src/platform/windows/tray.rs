use anyhow::Result;
use tracing::{debug, error};
use windows::core::{w, HSTRING, PCWSTR};
use windows::Win32::Foundation::{HINSTANCE, HWND, POINT};
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE,
    NIM_MODIFY, NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, LoadIconW, LoadImageW,
    SetForegroundWindow, TrackPopupMenu, HMENU, IDI_APPLICATION, IMAGE_ICON,
    LR_DEFAULTSIZE, LR_LOADFROMFILE, MF_SEPARATOR, MF_STRING, TPM_BOTTOMALIGN,
    TPM_LEFTALIGN, TPM_RETURNCMD, WM_APP,
};

/// Menu item IDs
pub const IDM_TOGGLE_ACTIVE: u32 = 100;
pub const IDM_RELOAD: u32 = 101;
pub const IDM_OPEN_CONFIG: u32 = 102;
pub const IDM_EXIT: u32 = 103;

/// Custom messages
pub const WM_APP_TRAY_NOTIFY: u32 = WM_APP + 1;

/// Tray icon
pub struct TrayIcon {
    data: NOTIFYICONDATAW,
    hwnd: HWND,
    active: bool,
    icon_path: Option<String>,
}

impl TrayIcon {
    /// Create new tray icon
    pub fn new() -> Self {
        Self::with_icon_path(None)
    }

    /// Create tray icon with custom icon path
    pub fn with_icon_path(icon_path: Option<String>) -> Self {
        let mut data = NOTIFYICONDATAW {
            uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
            uCallbackMessage: WM_APP_TRAY_NOTIFY,
            ..Default::default()
        };

        // Set tooltip text
        let tooltip = w!("wakem - Window/Keyboard/Mouse Enhancer");
        let tooltip_slice = unsafe { tooltip.as_wide() };
        let len = tooltip_slice.len().min(127);
        data.szTip[..len].copy_from_slice(&tooltip_slice[..len]);
        data.szTip[len] = 0;

        Self {
            data,
            hwnd: HWND(0),
            active: true,
            icon_path,
        }
    }

    /// Load icon from file
    fn load_icon_from_file(
        path: &str,
    ) -> anyhow::Result<windows::Win32::UI::WindowsAndMessaging::HICON> {
        let path_wide = HSTRING::from(path);

        unsafe {
            let hicon = LoadImageW(
                HINSTANCE(0),
                &path_wide,
                IMAGE_ICON,
                0,
                0,
                LR_LOADFROMFILE | LR_DEFAULTSIZE,
            )
            .map_err(|e| anyhow::anyhow!("Failed to load icon from file: {}", e))?;

            Ok(windows::Win32::UI::WindowsAndMessaging::HICON(hicon.0))
        }
    }

    /// Register tray icon
    pub fn register(&mut self, hwnd: HWND) -> Result<()> {
        self.hwnd = hwnd;
        self.data.hWnd = hwnd;
        self.data.uID = 1;

        // Try to load custom icon, fallback to system default
        self.data.hIcon = if let Some(ref path) = self.icon_path {
            match Self::load_icon_from_file(path) {
                Ok(icon) => {
                    debug!("Loaded custom icon from: {}", path);
                    icon
                }
                Err(e) => {
                    debug!(
                        "Failed to load custom icon from '{}': {}, using default",
                        path, e
                    );
                    unsafe { LoadIconW(None, IDI_APPLICATION)? }
                }
            }
        } else {
            unsafe { LoadIconW(None, IDI_APPLICATION)? }
        };

        unsafe {
            Shell_NotifyIconW(NIM_ADD, &self.data)
                .ok()
                .map_err(|e| anyhow::anyhow!("Failed to add tray icon: {}", e))?;
        }

        debug!("Tray icon registered");
        Ok(())
    }

    /// Unregister tray icon
    pub fn unregister(&mut self) -> Result<()> {
        if self.hwnd.0 == 0 {
            return Ok(());
        }

        unsafe {
            Shell_NotifyIconW(NIM_DELETE, &self.data)
                .ok()
                .map_err(|e| anyhow::anyhow!("Failed to delete tray icon: {}", e))?;
        }

        self.hwnd = HWND(0);
        debug!("Tray icon unregistered");
        Ok(())
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
                .map_err(|e| anyhow::anyhow!("Failed to show notification: {}", e))?;
        }

        // Restore flags
        self.data.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;

        Ok(())
    }

    /// Show context menu
    pub fn show_menu(&self) -> Result<u32> {
        unsafe {
            SetForegroundWindow(self.hwnd).ok().map_err(|e| {
                anyhow::anyhow!("Failed to set foreground window: {}", e)
            })?;

            let mut cursor = POINT::default();
            if GetCursorPos(&mut cursor).is_err() {
                return Err(anyhow::anyhow!("Failed to get cursor pos"));
            }

            let hmenu = self.create_menu()?;

            let result = TrackPopupMenu(
                hmenu,
                TPM_LEFTALIGN | TPM_BOTTOMALIGN | TPM_RETURNCMD,
                cursor.x,
                cursor.y,
                0,
                self.hwnd,
                None,
            );

            // Destroy menu
            if DestroyMenu(hmenu).is_err() {
                error!("Failed to destroy menu");
            }

            if result.0 == 0 {
                return Ok(0);
            }

            Ok(result.0 as u32)
        }
    }

    /// Create context menu
    fn create_menu(&self) -> Result<HMENU> {
        unsafe {
            let hmenu = CreatePopupMenu()
                .map_err(|e| anyhow::anyhow!("Failed to create menu: {}", e))?;

            // Enable/Disable
            let active_text = if self.active {
                w!("Disable (&D)")
            } else {
                w!("Enable (&E)")
            };
            if AppendMenuW(hmenu, MF_STRING, IDM_TOGGLE_ACTIVE as usize, active_text)
                .is_err()
            {
                return Err(anyhow::anyhow!("Failed to append menu item"));
            }

            // Separator
            if AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null()).is_err() {
                return Err(anyhow::anyhow!("Failed to append separator"));
            }

            // Reload config
            if AppendMenuW(
                hmenu,
                MF_STRING,
                IDM_RELOAD as usize,
                w!("Reload Config (&R)"),
            )
            .is_err()
            {
                return Err(anyhow::anyhow!("Failed to append menu item"));
            }

            // Open config folder
            if AppendMenuW(
                hmenu,
                MF_STRING,
                IDM_OPEN_CONFIG as usize,
                w!("Open Config Folder (&O)"),
            )
            .is_err()
            {
                return Err(anyhow::anyhow!("Failed to append menu item"));
            }

            // Separator
            if AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null()).is_err() {
                return Err(anyhow::anyhow!("Failed to append separator"));
            }

            // Exit
            if AppendMenuW(hmenu, MF_STRING, IDM_EXIT as usize, w!("Exit (&X)")).is_err()
            {
                return Err(anyhow::anyhow!("Failed to append menu item"));
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
        if let Err(e) = self.unregister() {
            error!("Failed to unregister tray icon: {}", e);
        }
    }
}
