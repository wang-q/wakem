use anyhow::Result;
use std::sync::{Arc, Mutex};
use tracing::{debug, error, info};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, GetWindowLongPtrW,
    PostQuitMessage, RegisterClassW, SetWindowLongPtrW, ShowWindow, TranslateMessage,
    CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, GWLP_USERDATA, MSG, SW_HIDE, WM_COMMAND,
    WM_CREATE, WM_DESTROY, WM_LBUTTONUP, WM_RBUTTONUP, WNDCLASSW, WS_EX_NOACTIVATE,
    WS_OVERLAPPEDWINDOW,
};

use crate::platform::windows::tray::{
    TrayIcon, IDM_EXIT, IDM_OPEN_CONFIG, IDM_RELOAD, IDM_TOGGLE_ACTIVE, WM_TRAY_NOTIFY,
};

// Re-export for backward compatibility

/// Application commands
#[derive(Debug, Clone, Copy)]
pub enum AppCommand {
    ToggleActive,
    ReloadConfig,
    OpenConfigFolder,
    Exit,
}

/// Message window structure
pub struct MessageWindow {
    hwnd: HWND,
    tray_icon: Arc<Mutex<TrayIcon>>,
    running: Arc<Mutex<bool>>,
    #[allow(clippy::type_complexity)]
    command_callback: Arc<Mutex<Option<Box<dyn Fn(AppCommand) + Send + 'static>>>>,
}

impl MessageWindow {
    /// Create message window
    pub fn new() -> Result<Arc<Self>> {
        let hwnd = Self::create_window()?;

        let window = Arc::new(Self {
            hwnd,
            tray_icon: Arc::new(Mutex::new(TrayIcon::new())),
            running: Arc::new(Mutex::new(true)),
            command_callback: Arc::new(Mutex::new(None)),
        });

        // Store raw pointer of Arc in window data (safe: Arc uses heap allocation, won't move)
        unsafe {
            let ptr = Arc::into_raw(Arc::clone(&window)) as isize;
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, ptr);
        }

        Ok(window)
    }

    /// Create Windows window
    fn create_window() -> Result<HWND> {
        unsafe {
            let class_name = windows::core::w!("WakemMessageWindow");
            let hinstance = GetModuleHandleW(None)?;

            let wnd_class = WNDCLASSW {
                lpfnWndProc: Some(Self::window_proc),
                hInstance: hinstance.into(),
                lpszClassName: class_name,
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
                WS_EX_NOACTIVATE,
                PCWSTR(atom as _),
                windows::core::w!("wakem"),
                WS_OVERLAPPEDWINDOW,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                None,
                None,
                Some(hinstance.into()),
                None,
            )?;

            info!("Message window created: {:?}", hwnd);

            // Hide the window - it's only used for message handling
            let _ = ShowWindow(hwnd, SW_HIDE);

            Ok(hwnd)
        }
    }

    /// Initialize tray icon
    pub fn init_tray(&self) -> Result<()> {
        let mut tray = self.tray_icon.lock().unwrap();
        tray.register(self.hwnd)?;
        info!("Tray icon registered");
        Ok(())
    }

    /// Set command callback
    pub fn set_command_callback<F>(&self, callback: F)
    where
        F: Fn(AppCommand) + Send + 'static,
    {
        let mut cb = self.command_callback.lock().unwrap();
        *cb = Some(Box::new(callback));
    }

    /// Send command (called from window procedure)
    fn send_command(&self, cmd: AppCommand) {
        let cb = self.command_callback.lock().unwrap();
        if let Some(ref callback) = *cb {
            callback(cmd);
        }
    }

    /// Run message loop
    pub fn run(&self) -> Result<()> {
        info!("Starting message loop");

        unsafe {
            let mut msg: MSG = std::mem::zeroed();

            while GetMessageW(&mut msg, None, 0, 0).into() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }

        info!("Message loop ended");
        Ok(())
    }

    /// Stop message loop
    pub fn stop(&self) {
        let mut running = self.running.lock().unwrap();
        *running = false;

        unsafe {
            PostQuitMessage(0);
        }
    }

    /// Get window handle
    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }

    /// Get instance reference from window handle (get reference without consuming Arc)
    unsafe fn get_instance(hwnd: HWND) -> Option<&'static Self> {
        let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
        if ptr == 0 {
            return None;
        }
        // Get reference without consuming the Arc
        Some(&*(ptr as *const Self))
    }

    /// Window procedure
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
            WM_TRAY_NOTIFY => {
                // Tray icon message - lparam contains the mouse message directly
                Self::handle_tray_notify(hwnd, lparam)
            }
            WM_COMMAND => {
                // Menu command - parse like window-switcher
                let value = wparam.0 as u32;
                let kind = ((value >> 16) & 0xffff) as u16;
                let id = value & 0xffff;
                if kind == 0 {
                    Self::handle_menu_command(hwnd, id);
                }
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }

    /// Handle tray icon message
    unsafe fn handle_tray_notify(hwnd: HWND, lparam: LPARAM) -> LRESULT {
        let mouse_msg = lparam.0 as u32;
        debug!("Tray notify: mouse_msg={}", mouse_msg);

        match mouse_msg {
            WM_LBUTTONUP | WM_RBUTTONUP => {
                debug!("Tray icon clicked");

                if let Some(instance) = Self::get_instance(hwnd) {
                    let mut tray = instance.tray_icon.lock().unwrap();
                    if let Err(e) = tray.show() {
                        error!("Failed to show menu: {}", e);
                    }
                }
            }
            _ => {
                debug!("Unknown tray message: {}", mouse_msg);
            }
        }

        LRESULT(0)
    }

    /// Handle menu command
    unsafe fn handle_menu_command(hwnd: HWND, id: u32) {
        debug!("Menu command: id={}", id);
        if let Some(instance) = Self::get_instance(hwnd) {
            match id {
                IDM_TOGGLE_ACTIVE => {
                    info!("Toggle active");
                    instance.send_command(AppCommand::ToggleActive);
                }
                IDM_RELOAD => {
                    info!("Reload config");
                    instance.send_command(AppCommand::ReloadConfig);
                }
                IDM_OPEN_CONFIG => {
                    info!("Open config folder");
                    instance.send_command(AppCommand::OpenConfigFolder);
                }
                IDM_EXIT => {
                    info!("Exit");
                    instance.send_command(AppCommand::Exit);
                }
                _ => {
                    debug!("Unknown menu command: {}", id);
                }
            }
        }
    }
}

impl Drop for MessageWindow {
    fn drop(&mut self) {
        debug!("MessageWindow dropping");
    }
}

// Ensure MessageWindow is thread-safe
unsafe impl Send for MessageWindow {}
unsafe impl Sync for MessageWindow {}
