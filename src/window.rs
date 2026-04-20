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

/// 应用程序命令
#[derive(Debug, Clone, Copy)]
pub enum AppCommand {
    ToggleActive,
    ReloadConfig,
    OpenConfigFolder,
    Exit,
}

/// 消息窗口结构
pub struct MessageWindow {
    hwnd: HWND,
    tray_icon: Arc<Mutex<TrayIcon>>,
    running: Arc<Mutex<bool>>,
    command_callback: Option<Box<dyn Fn(AppCommand) + Send>>,
}

impl MessageWindow {
    /// 创建新的消息窗口
    pub fn new() -> Result<Self> {
        Self::with_icon_path(None)
    }

    /// 创建带自定义图标路径的消息窗口
    pub fn with_icon_path(icon_path: Option<String>) -> Result<Self> {
        let hwnd = Self::create_window()?;

        let window = Self {
            hwnd,
            tray_icon: Arc::new(Mutex::new(TrayIcon::with_icon_path(icon_path))),
            running: Arc::new(Mutex::new(true)),
            command_callback: None,
        };

        // 将窗口实例存储在窗口数据中
        unsafe {
            let ptr = &window as *const Self as isize;
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, ptr);
        }

        Ok(window)
    }

    /// 创建 Windows 窗口
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

    /// 初始化托盘图标
    pub fn init_tray(&self) -> Result<()> {
        let mut tray = self.tray_icon.lock().unwrap();
        tray.register(self.hwnd)?;
        debug!("Tray icon initialized");
        Ok(())
    }

    /// 设置命令回调
    pub fn set_command_callback<F>(&mut self, callback: F)
    where
        F: Fn(AppCommand) + Send + 'static,
    {
        self.command_callback = Some(Box::new(callback));
    }

    /// 发送命令（从窗口过程调用）
    fn send_command(&self, cmd: AppCommand) {
        if let Some(ref callback) = self.command_callback {
            callback(cmd);
        }
    }

    /// 运行消息循环
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

    /// 停止消息循环
    pub fn stop(&self) {
        let mut running = self.running.lock().unwrap();
        *running = false;

        unsafe {
            PostQuitMessage(0);
        }
    }

    /// 获取窗口句柄
    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }

    /// 获取托盘图标
    pub fn tray_icon(&self) -> Arc<Mutex<TrayIcon>> {
        self.tray_icon.clone()
    }

    /// 从窗口句柄获取实例引用
    unsafe fn get_instance(hwnd: HWND) -> Option<&'static Self> {
        let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
        if ptr == 0 {
            return None;
        }
        Some(&*(ptr as *const Self))
    }

    /// 窗口过程
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
                // 托盘图标消息
                Self::handle_tray_notify(hwnd, lparam)
            }
            WM_COMMAND => {
                // 菜单命令
                let id = wparam.0 as u32;
                Self::handle_menu_command(hwnd, id);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }

    /// 处理托盘图标消息
    unsafe fn handle_tray_notify(hwnd: HWND, lparam: LPARAM) -> LRESULT {
        use windows::Win32::UI::WindowsAndMessaging::{
            WM_LBUTTONDBLCLK, WM_LBUTTONUP, WM_RBUTTONUP,
        };

        let msg = lparam.0 as u32;
        match msg {
            WM_RBUTTONUP => {
                // 右键点击，显示菜单
                debug!("Tray icon right-clicked");

                if let Some(instance) = Self::get_instance(hwnd) {
                    let tray = instance.tray_icon.lock().unwrap();
                    match tray.show_menu() {
                        Ok(cmd_id) => {
                            if cmd_id != 0 {
                                // 发送 WM_COMMAND 消息来处理菜单选择
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
                // 左键双击，切换启用状态
                debug!("Tray icon double-clicked");
                if let Some(instance) = Self::get_instance(hwnd) {
                    instance.send_command(AppCommand::ToggleActive);
                }
            }
            WM_LBUTTONUP => {
                // 左键单击
                debug!("Tray icon clicked");
            }
            _ => {}
        }

        LRESULT(0)
    }

    /// 处理菜单命令
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
        // 清理托盘图标
        if let Ok(mut tray) = self.tray_icon.lock() {
            let _ = tray.unregister();
        }
    }
}

// 确保 MessageWindow 是线程安全的
unsafe impl Send for MessageWindow {}
unsafe impl Sync for MessageWindow {}
