use anyhow::Result;
use std::sync::{Arc, Mutex};
use tracing::{debug, error, info};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, GetWindowLongPtrW,
    PostMessageW, PostQuitMessage, RegisterClassW, SetWindowLongPtrW, TranslateMessage,
    CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, GWLP_USERDATA, MSG, WM_COMMAND, WM_CREATE,
    WM_DESTROY, WNDCLASSW, WS_EX_NOACTIVATE, WS_OVERLAPPEDWINDOW,
};

use crate::platform::windows::tray::{
    TrayIcon, IDM_EXIT, IDM_OPEN_CONFIG, IDM_RELOAD, IDM_TOGGLE_ACTIVE,
    WM_APP_TRAY_NOTIFY,
};

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
    command_callback: Arc<Mutex<Option<Box<dyn Fn(AppCommand) + Send + 'static>>>>,
}

impl MessageWindow {
    /// Create message window with custom icon path
    pub fn with_icon_path(icon_path: Option<String>) -> Result<Arc<Self>> {
        let hwnd = Self::create_window()?;

        let window = Arc::new(Self {
            hwnd,
            tray_icon: Arc::new(Mutex::new(TrayIcon::with_icon_path(icon_path))),
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

            RegisterClassW(&wnd_class);

            let hwnd = CreateWindowExW(
                WS_EX_NOACTIVATE,
                class_name,
                windows::core::w!("wakem"),
                WS_OVERLAPPEDWINDOW,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                None,
                None,
                hinstance,
                None,
            );

            if hwnd.0 == 0 {
                return Err(anyhow::anyhow!("Failed to create window"));
            }

            debug!("Message window created: {:?}", hwnd);
            Ok(hwnd)
        }
    }

    /// Initialize tray icon
    pub fn init_tray(&self) -> Result<()> {
        let mut tray = self.tray_icon.lock().unwrap();
        tray.register(self.hwnd)?;
        debug!("Tray icon initialized");
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
                TranslateMessage(&msg);
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

    /// Get instance reference from window handle (safely reconstruct Arc)
    unsafe fn get_instance(hwnd: HWND) -> Option<Arc<Self>> {
        let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
        if ptr == 0 {
            return None;
        }
        // Safely reconstruct Arc from raw pointer
        Some(Arc::from_raw(ptr as *const Self))
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
            WM_APP_TRAY_NOTIFY => {
                // Tray icon message
                Self::handle_tray_notify(hwnd, lparam)
            }
            WM_COMMAND => {
                // Menu command
                let id = wparam.0 as u32;
                Self::handle_menu_command(hwnd, id);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }

    /// Handle tray icon message
    unsafe fn handle_tray_notify(hwnd: HWND, lparam: LPARAM) -> LRESULT {
        use windows::Win32::UI::WindowsAndMessaging::{
            WM_LBUTTONDBLCLK, WM_LBUTTONUP, WM_RBUTTONUP,
        };

        let msg = lparam.0 as u32;
        match msg {
            WM_RBUTTONUP => {
                // Right-click to show menu
                debug!("Tray icon right-clicked");

                if let Some(instance) = Self::get_instance(hwnd) {
                    let tray = instance.tray_icon.lock().unwrap();
                    match tray.show_menu() {
                        Ok(cmd_id) => {
                            if cmd_id != 0 {
                                // Send WM_COMMAND message to handle menu selection
                                let _ = PostMessageW(
                                    hwnd,
                                    WM_COMMAND,
                                    WPARAM(cmd_id as usize),
                                    LPARAM(0),
                                );
                            }
                        }
                        Err(e) => {
                            error!("Failed to show menu: {}", e);
                        }
                    }
                }
            }
            WM_LBUTTONDBLCLK => {
                // Double-click to toggle active state
                debug!("Tray icon double-clicked");
                if let Some(instance) = Self::get_instance(hwnd) {
                    instance.send_command(AppCommand::ToggleActive);
                }
            }
            WM_LBUTTONUP => {
                // Left-click
                debug!("Tray icon clicked");
            }
            _ => {}
        }

        LRESULT(0)
    }

    /// Handle menu command
    unsafe fn handle_menu_command(hwnd: HWND, id: u32) {
        if let Some(instance) = Self::get_instance(hwnd) {
            match id {
                IDM_TOGGLE_ACTIVE => {
                    info!("Toggle active menu item clicked");
                    instance.send_command(AppCommand::ToggleActive);
                }
                IDM_RELOAD => {
                    info!("Reload config menu item clicked");
                    instance.send_command(AppCommand::ReloadConfig);
                }
                IDM_OPEN_CONFIG => {
                    info!("Open config menu item clicked");
                    instance.send_command(AppCommand::OpenConfigFolder);
                }
                IDM_EXIT => {
                    info!("Exit menu item clicked");
                    instance.send_command(AppCommand::Exit);
                }
                _ => {}
            }
        }
    }
}

impl Drop for MessageWindow {
    fn drop(&mut self) {
        debug!("MessageWindow dropping");
        // Clean up tray icon
        if let Ok(mut tray) = self.tray_icon.lock() {
            if let Err(e) = tray.unregister() {
                debug!("Failed to unregister tray icon: {}", e);
            }
        }
    }
}

// Ensure MessageWindow is thread-safe
unsafe impl Send for MessageWindow {}
unsafe impl Sync for MessageWindow {}
