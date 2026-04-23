//! Windows window event hook implementation
#![cfg(target_os = "windows")]

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
    GetWindowTextW, EVENT_SYSTEM_FOREGROUND, WINEVENT_OUTOFCONTEXT,
    WINEVENT_SKIPOWNPROCESS,
};

/// Window event types
#[derive(Debug, Clone)]
pub enum WindowEvent {
    /// Window activated (became foreground)
    WindowActivated(isize), // Store as isize instead of HWND for Send/Sync
}

// SAFETY: We store HWND as isize, which is Send + Sync
unsafe impl Send for WindowEvent {}
unsafe impl Sync for WindowEvent {}

/// Window event hook manager
pub struct WindowEventHook {
    hook: Option<HWINEVENTHOOK>,
    event_tx: Sender<WindowEvent>,
    /// Shutdown flag for graceful exit
    shutdown_flag: Arc<AtomicBool>,
}

impl WindowEventHook {
    /// Create new window event hook
    pub fn new(event_tx: Sender<WindowEvent>) -> Self {
        Self {
            hook: None,
            event_tx,
            shutdown_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Start window event monitoring
    pub fn start(&mut self) -> Result<()> {
        unsafe {
            // Set window event hook
            // Monitor: foreground window changes (window activation)
            // Note: eventMin must be <= eventMax, so we use the same value for both
            let hook = SetWinEventHook(
                EVENT_SYSTEM_FOREGROUND,
                EVENT_SYSTEM_FOREGROUND,
                None, // Current process
                Some(win_event_callback),
                0, // All processes
                0, // All threads
                WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
            );

            if hook.is_invalid() {
                return Err(anyhow::anyhow!("Failed to set WinEventHook"));
            }

            // Store sender in global/thread-local storage for callback use
            // Using a simple global variable approach here
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

// Global sender (for callback function)
use std::sync::OnceLock;

static GLOBAL_SENDER: OnceLock<Sender<WindowEvent>> = OnceLock::new();

fn set_global_sender(sender: Sender<WindowEvent>) {
    let _ = GLOBAL_SENDER.set(sender);
}

fn get_global_sender() -> Option<&'static Sender<WindowEvent>> {
    GLOBAL_SENDER.get()
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

    // Get window title for debugging
    let mut title_buffer = [0u16; 256];
    let len = GetWindowTextW(hwnd, &mut title_buffer);
    let title = String::from_utf16_lossy(&title_buffer[..len as usize]);

    if event == EVENT_SYSTEM_FOREGROUND {
        debug!("Window activated: {} ({:?})", title, hwnd);
        if let Some(sender) = get_global_sender() {
            let _ = sender.send(WindowEvent::WindowActivated(hwnd.0 as isize));
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
        // Note: Actually starting the hook requires a message loop environment
        drop(hook);
    }
}
