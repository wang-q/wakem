use anyhow::Result;
use tracing::error;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE,
    NIM_MODIFY, NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreateIconFromResourceEx, CreatePopupMenu, DestroyMenu, GetCursorPos,
    LookupIconIdFromDirectoryEx, SetForegroundWindow, TrackPopupMenu, HMENU,
    LR_DEFAULTCOLOR, MF_SEPARATOR, MF_STRING, TPM_BOTTOMALIGN, TPM_LEFTALIGN,
};

/// Embedded icon resource
const ICON_BYTES: &[u8] = include_bytes!("../../../assets/icon.ico");

/// Menu item IDs
pub const IDM_TOGGLE_ACTIVE: u32 = 100;
pub const IDM_RELOAD: u32 = 101;
pub const IDM_OPEN_CONFIG: u32 = 102;
pub const IDM_EXIT: u32 = 103;

/// Custom messages - use WM_USER range like window-switcher
pub const WM_TRAY_NOTIFY: u32 = 6000;

/// Tray icon
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

    /// Create tray icon (like window-switcher)
    fn create() -> Self {
        let data = Self::create_nid();
        Self { data }
    }

    /// Create NOTIFYICONDATAW (like window-switcher)
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
            uID: WM_TRAY_NOTIFY,
            uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
            uCallbackMessage: WM_TRAY_NOTIFY,
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
                .map_err(|e| anyhow::anyhow!("Failed to add tray icon: {}", e))
        }
    }

    /// Check if tray icon exists
    pub fn exist(&mut self) -> bool {
        unsafe { Shell_NotifyIconW(NIM_MODIFY, &self.data).as_bool() }
    }

    /// Unregister tray icon (alias for Drop)
    pub fn unregister(&mut self) -> Result<()> {
        unsafe {
            Shell_NotifyIconW(NIM_DELETE, &self.data)
                .ok()
                .map_err(|e| anyhow::anyhow!("Failed to delete tray icon: {}", e))
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
                .map_err(|e| anyhow::anyhow!("Failed to show notification: {}", e))?;
        }

        // Restore flags
        self.data.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;

        Ok(())
    }

    /// Show context menu and return selected item ID (for API compatibility)
    pub fn show_menu(&mut self) -> Result<u32> {
        // This is a synchronous version that doesn't return the selection
        // The actual menu selection is handled via WM_COMMAND messages
        self.show()?;
        Ok(0)
    }

    /// Show context menu
    pub fn show(&mut self) -> Result<()> {
        unsafe {
            SetForegroundWindow(self.data.hWnd).ok().map_err(|e| {
                anyhow::anyhow!("Failed to set foreground window: {}", e)
            })?;

            let mut cursor = POINT::default();
            GetCursorPos(&mut cursor)
                .map_err(|e| anyhow::anyhow!("Failed to get cursor pos: {}", e))?;

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
            .map_err(|e| anyhow::anyhow!("Failed to show popup menu: {}", e))?;

            // Destroy menu
            if DestroyMenu(hmenu).is_err() {
                error!("Failed to destroy menu");
            }

            Ok(())
        }
    }

    /// Create context menu
    fn create_menu(&mut self) -> Result<HMENU> {
        unsafe {
            let hmenu = CreatePopupMenu()
                .map_err(|e| anyhow::anyhow!("Failed to create menu: {}", e))?;

            // Enable/Disable
            if AppendMenuW(
                hmenu,
                MF_STRING,
                IDM_TOGGLE_ACTIVE as usize,
                w!("Enable/Disable"),
            )
            .is_err()
            {
                return Err(anyhow::anyhow!("Failed to append menu item"));
            }

            // Separator
            if AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null()).is_err() {
                return Err(anyhow::anyhow!("Failed to append separator"));
            }

            // Reload config
            if AppendMenuW(hmenu, MF_STRING, IDM_RELOAD as usize, w!("Reload Config"))
                .is_err()
            {
                return Err(anyhow::anyhow!("Failed to append menu item"));
            }

            // Open config folder
            if AppendMenuW(
                hmenu,
                MF_STRING,
                IDM_OPEN_CONFIG as usize,
                w!("Open Config Folder"),
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
            if AppendMenuW(hmenu, MF_STRING, IDM_EXIT as usize, w!("Exit")).is_err() {
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
        unsafe {
            let _ = Shell_NotifyIconW(NIM_DELETE, &self.data);
        }
    }
}
