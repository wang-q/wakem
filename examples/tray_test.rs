// Tray icon test example - Windows only

#[cfg(target_os = "windows")]
mod windows_tray_test {
    use anyhow::Result;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use windows::core::{w, PCWSTR};
    use windows::Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, LoadCursorW,
            PostQuitMessage, RegisterClassW, TranslateMessage, CS_HREDRAW, CS_VREDRAW,
            CW_USEDEFAULT, IDC_ARROW, MSG, WINDOW_STYLE, WM_COMMAND, WM_CREATE,
            WM_DESTROY, WM_USER, WNDCLASSW, WS_EX_LAYERED, WS_EX_TOOLWINDOW,
            WS_EX_TOPMOST,
        },
    };

    // Use same message ID as window-switcher
    const WM_USER_TRAYICON: u32 = 6000;
    const WM_LBUTTONUP: u32 = 0x0202;
    const WM_RBUTTONUP: u32 = 0x0205;

    static mut TRAY_ICON: Option<TrayIconTest> = None;

    struct TrayIconTest {
        data: windows::Win32::UI::Shell::NOTIFYICONDATAW,
    }

    impl TrayIconTest {
        fn create() -> Self {
            use windows::Win32::UI::Shell::{
                NIF_ICON, NIF_MESSAGE, NIF_TIP, NOTIFYICONDATAW,
            };
            use windows::Win32::UI::WindowsAndMessaging::{
                CreateIconFromResourceEx, LookupIconIdFromDirectoryEx, LR_DEFAULTCOLOR,
            };

            const ICON_BYTES: &[u8] = include_bytes!("../assets/icon.ico");

            let offset = unsafe {
                LookupIconIdFromDirectoryEx(
                    ICON_BYTES.as_ptr(),
                    true,
                    0,
                    0,
                    LR_DEFAULTCOLOR,
                )
            };
            let icon_data = &ICON_BYTES[offset as usize..];
            let hicon = unsafe {
                CreateIconFromResourceEx(icon_data, true, 0x30000, 0, 0, LR_DEFAULTCOLOR)
            }
            .expect("Failed to load icon");

            let mut tooltip: Vec<u16> = unsafe { w!("Test Tray").as_wide() }.to_vec();
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
            use windows::Win32::UI::Shell::{Shell_NotifyIconW, NIM_ADD};
            self.data.hWnd = hwnd;
            unsafe {
                let _ = Shell_NotifyIconW(NIM_ADD, &self.data);
            }
            println!("Tray icon registered!");
        }

        fn show_menu(&self) {
            use windows::Win32::UI::WindowsAndMessaging::{
                AppendMenuW, CreatePopupMenu, GetCursorPos, SetForegroundWindow,
                TrackPopupMenu, HMENU, MF_STRING, TPM_BOTTOMALIGN, TPM_LEFTALIGN,
            };

            let hwnd = self.data.hWnd;
            let mut cursor = windows::Win32::Foundation::POINT::default();

            unsafe {
                let _ = SetForegroundWindow(hwnd);
                let _ = GetCursorPos(&mut cursor);
                let hmenu: HMENU = CreatePopupMenu().unwrap();
                let _ = AppendMenuW(hmenu, MF_STRING, 1, w!("Exit"));
                let _ = TrackPopupMenu(
                    hmenu,
                    TPM_LEFTALIGN | TPM_BOTTOMALIGN,
                    cursor.x,
                    cursor.y,
                    None,
                    hwnd,
                    None,
                );
            }
            println!("Menu shown!");
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
                println!("Window created");
                LRESULT(0)
            }
            WM_DESTROY => {
                println!("Window destroyed");
                PostQuitMessage(0);
                LRESULT(0)
            }
            WM_USER_TRAYICON => {
                let mouse_msg = lparam.0 as u32;
                println!("Tray notify: mouse_msg={}", mouse_msg);

                if mouse_msg == WM_LBUTTONUP || mouse_msg == WM_RBUTTONUP {
                    println!("Tray icon clicked!");
                    if let Some(ref mut tray) = TRAY_ICON {
                        tray.show_menu();
                    }
                }
                LRESULT(0)
            }
            WM_COMMAND => {
                println!("Menu command: wparam={}", wparam.0);
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }

    pub fn run() -> Result<()> {
        println!("Starting tray icon test...");

        unsafe {
            let class_name = w!("TestTrayWindow");
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
            println!("Window class registered: {}", atom);

            let hwnd = CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
                PCWSTR(atom as _),
                w!("Test Tray"),
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

            println!("Window created: {:?}", hwnd);

            // Create and register tray icon
            TRAY_ICON = Some(TrayIconTest::create());
            if let Some(ref mut tray) = TRAY_ICON {
                tray.register(hwnd);
            }

            // Message loop
            let mut msg: MSG = std::mem::zeroed();
            println!("Starting message loop...");

            loop {
                let ret = GetMessageW(&mut msg, None, 0, 0);
                match ret.0 {
                    -1 => {
                        println!("GetMessageW error");
                        break;
                    }
                    0 => break,
                    _ => {
                        let _ = TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                }
            }

            println!("Exiting...");
        }

        Ok(())
    }
}

#[cfg(target_os = "windows")]
fn main() -> anyhow::Result<()> {
    windows_tray_test::run()
}

#[cfg(not(target_os = "windows"))]
fn main() {
    println!("This example is only available on Windows");
}
