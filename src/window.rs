use anyhow::Result;
use std::sync::{Arc, Mutex};
use tracing::{debug, error, info};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreateIconFromResourceEx, CreatePopupMenu, CreateWindowExW,
    DefWindowProcW, DestroyMenu, DispatchMessageW, GetCursorPos, GetMessageW,
    LoadCursorW, LookupIconIdFromDirectoryEx, PostQuitMessage, RegisterClassW,
    SetForegroundWindow, TrackPopupMenu, TranslateMessage, CS_HREDRAW, CS_VREDRAW,
    CW_USEDEFAULT, HMENU, IDC_ARROW, LR_DEFAULTCOLOR, MF_STRING, MSG, TPM_BOTTOMALIGN,
    TPM_LEFTALIGN, WINDOW_STYLE, WM_COMMAND, WM_CREATE, WM_DESTROY, WNDCLASSW,
    WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
};

/// Embedded icon resource
const ICON_BYTES: &[u8] = include_bytes!("../assets/icon.ico");

/// Custom message ID - same as window-switcher
const WM_USER_TRAYICON: u32 = 6000;
const WM_LBUTTONUP: u32 = 0x0202;
const WM_RBUTTONUP: u32 = 0x0205;

/// Menu item IDs
const IDM_TOGGLE_ACTIVE: u32 = 100;
const IDM_RELOAD: u32 = 101;
const IDM_OPEN_CONFIG: u32 = 102;
const IDM_EXIT: u32 = 103;

/// Application commands
#[derive(Debug, Clone, Copy)]
pub enum AppCommand {
    ToggleActive,
    ReloadConfig,
    OpenConfigFolder,
    Exit,
}

/// Callback type for command handling
type CommandCallback = Box<dyn Fn(AppCommand) + Send + 'static>;

/// Global tray icon storage
static mut TRAY_ICON: Option<TrayIcon> = None;
static mut CMD_CALLBACK: Option<CommandCallback> = None;
static mut TRAY_HWND: Option<HWND> = None;

/// Tray icon structure
struct TrayIcon {
    data: NOTIFYICONDATAW,
}

impl TrayIcon {
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
        let mut cursor = windows::Win32::Foundation::POINT::default();

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
            let _ =
                AppendMenuW(hmenu, MF_STRING, IDM_RELOAD as usize, w!("Reload Config"));
            let _ = AppendMenuW(
                hmenu,
                MF_STRING,
                IDM_OPEN_CONFIG as usize,
                w!("Open Config Folder"),
            );
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
            // Only log on actual clicks, not mouse move (0x0200)
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
            return Err(anyhow::anyhow!(
                "Failed to register window class: {:?}",
                err
            ));
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
        TRAY_ICON = Some(TrayIcon::create());
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
        // Send WM_DESTROY to the window to force the message loop to exit
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
