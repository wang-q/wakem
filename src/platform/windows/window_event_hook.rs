//! Windows window event hook implementation
#![cfg(target_os = "windows")]

use crate::platform::traits::PlatformWindowEvent;
use anyhow::Result;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use tracing::debug;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Accessibility::{
    SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowTextW, GetWindowThreadProcessId, EVENT_SYSTEM_FOREGROUND,
    WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS,
};

/// Window event hook manager
pub struct WindowEventHook {
    hook: Option<HWINEVENTHOOK>,
    event_tx: Sender<PlatformWindowEvent>,
    shutdown_flag: Arc<AtomicBool>,
}

unsafe impl Send for WindowEventHook {}

impl WindowEventHook {
    /// Create new window event hook
    pub fn new(event_tx: Sender<PlatformWindowEvent>) -> Self {
        Self {
            hook: None,
            event_tx,
            shutdown_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Start window event monitoring
    pub fn start(&mut self) -> Result<()> {
        unsafe {
            let hook = SetWinEventHook(
                EVENT_SYSTEM_FOREGROUND,
                EVENT_SYSTEM_FOREGROUND,
                None,
                Some(win_event_callback),
                0,
                0,
                WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
            );

            if hook.is_invalid() {
                return Err(anyhow::anyhow!("Failed to set WinEventHook"));
            }

            set_global_sender(self.event_tx.clone());

            self.hook = Some(hook);
            debug!("Window event hook started");
            Ok(())
        }
    }

    /// Start window event monitoring with shutdown flag for graceful exit
    pub fn start_with_shutdown(&mut self, shutdown_flag: Arc<AtomicBool>) -> Result<()> {
        self.shutdown_flag = shutdown_flag;
        self.start()
    }

    /// Stop window event monitoring
    pub fn stop(&mut self) {
        if let Some(hook) = self.hook.take() {
            unsafe {
                let _ = UnhookWinEvent(hook);
            }
            debug!("Window event hook stopped");
        }
    }

    /// Get shutdown flag reference
    pub fn shutdown_flag(&self) -> Arc<AtomicBool> {
        self.shutdown_flag.clone()
    }
}

impl Drop for WindowEventHook {
    fn drop(&mut self) {
        self.stop();
    }
}

use std::sync::OnceLock;

static GLOBAL_SENDER: OnceLock<Sender<PlatformWindowEvent>> = OnceLock::new();

fn set_global_sender(sender: Sender<PlatformWindowEvent>) {
    let _ = GLOBAL_SENDER.set(sender);
}

fn get_global_sender() -> Option<&'static Sender<PlatformWindowEvent>> {
    GLOBAL_SENDER.get()
}

unsafe fn get_process_name_for_hwnd(hwnd: HWND) -> String {
    let mut pid: u32 = 0;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));
    if pid == 0 {
        return String::new();
    }
    super::get_process_name_by_pid(pid).unwrap_or_default()
}

/// WinEvent callback function
unsafe extern "system" fn win_event_callback(
    _hook: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    _id_object: i32,
    _id_child: i32,
    _id_event_thread: u32,
    _dwms_event_time: u32,
) {
    if hwnd.0.is_null() {
        return;
    }

    if event == EVENT_SYSTEM_FOREGROUND {
        let mut title_buffer = [0u16; 256];
        let len = GetWindowTextW(hwnd, &mut title_buffer);
        let title = String::from_utf16_lossy(&title_buffer[..len as usize]);

        let process_name = get_process_name_for_hwnd(hwnd);

        debug!("Window activated: {} ({:?})", title, hwnd);
        if let Some(sender) = get_global_sender() {
            let _ = sender.send(PlatformWindowEvent::WindowActivated {
                process_name,
                window_title: title,
                window_id: hwnd.0 as usize,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "windows")]
    fn test_window_event_creation() {
        let (tx, _rx) = std::sync::mpsc::channel();
        let hook = WindowEventHook::new(tx);
        drop(hook);
    }
}
