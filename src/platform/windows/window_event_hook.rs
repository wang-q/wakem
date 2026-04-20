use anyhow::Result;
use std::sync::mpsc::Sender;
use tracing::debug;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Accessibility::{
    SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowTextW, EVENT_OBJECT_CREATE, EVENT_SYSTEM_FOREGROUND, WINEVENT_OUTOFCONTEXT,
    WINEVENT_SKIPOWNPROCESS,
};

/// 窗口事件类型
#[derive(Debug, Clone)]
pub enum WindowEvent {
    /// 窗口被创建
    WindowCreated(HWND),
    /// 窗口成为前台窗口
    WindowActivated(HWND),
}

/// 窗口事件钩子管理器
pub struct WindowEventHook {
    hook: Option<HWINEVENTHOOK>,
    event_tx: Sender<WindowEvent>,
}

impl WindowEventHook {
    /// 创建新的窗口事件钩子
    pub fn new(event_tx: Sender<WindowEvent>) -> Self {
        Self {
            hook: None,
            event_tx,
        }
    }

    /// 启动窗口事件监听
    pub fn start(&mut self) -> Result<()> {
        unsafe {
            // 设置窗口事件钩子
            // 监听：窗口创建和前台窗口切换
            let hook = SetWinEventHook(
                EVENT_OBJECT_CREATE,
                EVENT_SYSTEM_FOREGROUND,
                None, // 当前进程
                Some(win_event_callback),
                0, // 所有进程
                0, // 所有线程
                WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
            );

            if hook.is_invalid() {
                return Err(anyhow::anyhow!("Failed to set WinEventHook"));
            }

            // 将 sender 存储在全局/线程本地存储中供回调使用
            // 这里使用一个简单的全局变量方式
            set_global_sender(self.event_tx.clone());

            self.hook = Some(hook);
            debug!("Window event hook started");
            Ok(())
        }
    }

    /// 停止窗口事件监听
    pub fn stop(&mut self) {
        if let Some(hook) = self.hook.take() {
            unsafe {
                UnhookWinEvent(hook).ok();
            }
            debug!("Window event hook stopped");
        }
    }
}

impl Drop for WindowEventHook {
    fn drop(&mut self) {
        self.stop();
    }
}

// 全局 sender（用于回调函数）
use std::sync::OnceLock;

static GLOBAL_SENDER: OnceLock<Sender<WindowEvent>> = OnceLock::new();

fn set_global_sender(sender: Sender<WindowEvent>) {
    let _ = GLOBAL_SENDER.set(sender);
}

fn get_global_sender() -> Option<&'static Sender<WindowEvent>> {
    GLOBAL_SENDER.get()
}

/// WinEvent 回调函数
unsafe extern "system" fn win_event_callback(
    _hook: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    _id_object: i32,
    _id_child: i32,
    _id_event_thread: u32,
    _dwms_event_time: u32,
) {
    if hwnd.0 == 0 {
        return;
    }

    // 获取窗口标题用于调试
    let mut title_buffer = [0u16; 256];
    let len = GetWindowTextW(hwnd, &mut title_buffer);
    let title = String::from_utf16_lossy(&title_buffer[..len as usize]);

    match event {
        EVENT_OBJECT_CREATE => {
            debug!("Window created: {} ({:?})", title, hwnd);
            if let Some(sender) = get_global_sender() {
                let _ = sender.send(WindowEvent::WindowCreated(hwnd));
            }
        }
        EVENT_SYSTEM_FOREGROUND => {
            debug!("Window activated: {} ({:?})", title, hwnd);
            if let Some(sender) = get_global_sender() {
                let _ = sender.send(WindowEvent::WindowActivated(hwnd));
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_event_creation() {
        let (tx, _rx) = std::sync::mpsc::channel();
        let hook = WindowEventHook::new(tx);
        // 注意：实际启动钩子需要在有消息循环的环境中
        drop(hook);
    }
}
