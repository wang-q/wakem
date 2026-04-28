//! Windows system tray implementation
//!
//! This module provides a complete system tray implementation including:
//! - Tray icon management (register, unregister, notifications)
//! - Context menu display
//! - Message loop for handling tray events
//! - Async API trait for integration with async code
#![cfg(target_os = "windows")]

use anyhow::{anyhow, Result};
use tracing::{debug, error};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreateIconFromResourceEx, CreatePopupMenu, CreateWindowExW,
    DefWindowProcW, DestroyMenu, DispatchMessageW, GetCursorPos, GetMessageW,
    LoadCursorW, LookupIconIdFromDirectoryEx, PostMessageW, PostQuitMessage,
    RegisterClassW, SetForegroundWindow, TrackPopupMenu, TranslateMessage, CS_HREDRAW,
    CS_VREDRAW, CW_USEDEFAULT, HMENU, IDC_ARROW, LR_DEFAULTCOLOR, MF_SEPARATOR,
    MF_STRING, MSG, TPM_BOTTOMALIGN, TPM_LEFTALIGN, TPM_RIGHTBUTTON, WINDOW_STYLE,
    WM_COMMAND, WM_CREATE, WM_DESTROY, WNDCLASSW, WS_EX_LAYERED, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST,
};

/// Embedded icon resource
const ICON_BYTES: &[u8] = include_bytes!("../../../assets/icon.ico");

/// Custom message ID
const WM_USER_TRAYICON: u32 = 6000;
const WM_LBUTTONUP: u32 = 0x0202;
const WM_RBUTTONUP: u32 = 0x0205;

// Re-export shared tray types from platform::traits
pub use crate::platform::traits::AppCommand;
// Re-export menu ID constants from tray_common
pub use crate::platform::tray_common::menu_ids;

/// Callback type for command handling
type CommandCallback = Box<dyn Fn(AppCommand) + Send + 'static>;

use std::cell::RefCell;

thread_local! {
    static TRAY_ICON: RefCell<Option<TrayIconData>> = const { RefCell::new(None) };
    static CMD_CALLBACK: RefCell<Option<CommandCallback>> = const { RefCell::new(None) };
    static TRAY_HWND: RefCell<Option<HWND>> = const { RefCell::new(None) };
    static MAIN_THREAD_ID: RefCell<Option<u32>> = const { RefCell::new(None) };
}

/// Tray icon data structure
struct TrayIconData {
    data: NOTIFYICONDATAW,
}

impl TrayIconData {
    fn create() -> Self {
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
        .expect("Failed to load icon");

        let mut tooltip = [0u16; 128];
        let src: Vec<u16> = unsafe { w!("wakem").as_wide() }.to_vec();
        let copy_len = src.len().min(tooltip.len() - 1);
        tooltip[..copy_len].copy_from_slice(&src[..copy_len]);

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
                menu_ids::TOGGLE_ACTIVE as usize,
                w!("Enable/Disable"),
            );
            let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null());
            let _ = AppendMenuW(
                hmenu,
                MF_STRING,
                menu_ids::RELOAD as usize,
                w!("Reload Config"),
            );
            let _ = AppendMenuW(
                hmenu,
                MF_STRING,
                menu_ids::OPEN_CONFIG as usize,
                w!("Open Config Folder"),
            );
            let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null());
            let _ = AppendMenuW(hmenu, MF_STRING, menu_ids::EXIT as usize, w!("Exit"));

            // Display menu - do NOT use TPM_RETURNCMD as it prevents WM_COMMAND from being sent
            let _ = TrackPopupMenu(
                hmenu,
                TPM_LEFTALIGN | TPM_BOTTOMALIGN | TPM_RIGHTBUTTON,
                cursor.x,
                cursor.y,
                None,
                hwnd,
                None,
            );

            // Required workaround for tray menu to work correctly on Windows
            // See: https://support.microsoft.com/en-us/kb/135788
            let _ = PostMessageW(Some(hwnd), 0u32, WPARAM(0), LPARAM(0));

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
                TRAY_ICON.with(|t| {
                    if let Some(ref tray) = *t.borrow() {
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
                    match id {
                        menu_ids::TOGGLE_ACTIVE => callback(AppCommand::ToggleActive),
                        menu_ids::RELOAD => callback(AppCommand::ReloadConfig),
                        menu_ids::OPEN_CONFIG => callback(AppCommand::OpenConfigFolder),
                        menu_ids::EXIT => callback(AppCommand::Exit),
                        _ => {}
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

        TRAY_HWND.with(|h| {
            *h.borrow_mut() = Some(hwnd);
        });
        MAIN_THREAD_ID.with(|t| {
            *t.borrow_mut() =
                Some(windows::Win32::System::Threading::GetCurrentThreadId());
        });

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
pub fn stop_tray() {
    unsafe {
        TRAY_HWND.with(|h| {
            if let Some(hwnd) = *h.borrow() {
                let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                    Some(hwnd),
                    windows::Win32::UI::WindowsAndMessaging::WM_CLOSE,
                    WPARAM(0),
                    LPARAM(0),
                );
            }
        });

        MAIN_THREAD_ID.with(|t| {
            if let Some(thread_id) = *t.borrow() {
                let _ = windows::Win32::UI::WindowsAndMessaging::PostThreadMessageW(
                    thread_id,
                    windows::Win32::UI::WindowsAndMessaging::WM_QUIT,
                    WPARAM(0),
                    LPARAM(0),
                );
            }
        });
    }
}


// Re-export shared TrayApi trait from tray_common
pub use crate::platform::tray_common::TrayApi;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::traits::MenuAction;
    use crate::platform::tray_common::MockTrayApi;

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
