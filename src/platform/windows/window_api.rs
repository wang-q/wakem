//! Windows window API implementation
#![cfg(target_os = "windows")]

use anyhow::Result;
use windows::Win32::Foundation::{HWND, LPARAM, RECT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowRect, IsIconic, IsWindow, IsZoomed, SetWindowPos,
    ShowWindow, HWND_TOP, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOOWNERZORDER,
    SWP_NOSIZE, SWP_NOZORDER, SW_RESTORE,
};

#[cfg(test)]
use crate::platform::mock::WindowApiCall;
use crate::platform::traits::{
    MonitorInfo, PlatformUtilities, WindowApiBase, WindowFrame, WindowInfo,
};
use crate::platform::windows::WindowsPlatform;
use tracing::debug;

/// Real Windows API implementation
pub struct RealWindowApi;

impl RealWindowApi {
    pub fn new() -> Self {
        Self
    }

    fn get_foreground_window_inner(&self) -> Option<HWND> {
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.0.is_null() {
                None
            } else {
                Some(hwnd)
            }
        }
    }

    fn get_window_rect(&self, hwnd: HWND) -> Option<WindowFrame> {
        unsafe {
            let mut rect = RECT::default();
            GetWindowRect(hwnd, &mut rect).ok()?;
            Some(WindowFrame::new(
                rect.left,
                rect.top,
                rect.right - rect.left,
                rect.bottom - rect.top,
            ))
        }
    }

    fn set_window_pos_inner(
        &self,
        hwnd: HWND,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<()> {
        unsafe {
            SetWindowPos(
                hwnd,
                None,
                x,
                y,
                width,
                height,
                SWP_NOACTIVATE | SWP_NOOWNERZORDER | SWP_FRAMECHANGED,
            )?;
            Ok(())
        }
    }

    fn is_window_valid_inner(&self, hwnd: HWND) -> bool {
        unsafe { IsWindow(Some(hwnd)).as_bool() }
    }

    fn get_window_title(&self, hwnd: HWND) -> Option<String> {
        unsafe {
            let mut title_buffer = [0u16; 256];
            let len = windows::Win32::UI::WindowsAndMessaging::GetWindowTextW(
                hwnd,
                &mut title_buffer,
            );
            if len == 0 {
                None
            } else {
                Some(String::from_utf16_lossy(&title_buffer[..len as usize]))
            }
        }
    }

    fn is_minimized_inner(&self, hwnd: HWND) -> bool {
        unsafe { IsIconic(hwnd).as_bool() }
    }

    fn is_maximized_inner(&self, hwnd: HWND) -> bool {
        unsafe { IsZoomed(hwnd).as_bool() }
    }

    fn minimize_window_inner(&self, hwnd: HWND) -> Result<()> {
        unsafe {
            ShowWindow(hwnd, windows::Win32::UI::WindowsAndMessaging::SW_MINIMIZE)
                .ok()
                .map_err(|e| anyhow::anyhow!("Failed to minimize window: {}", e))?;
            Ok(())
        }
    }

    fn maximize_window_inner(&self, hwnd: HWND) -> Result<()> {
        unsafe {
            ShowWindow(hwnd, windows::Win32::UI::WindowsAndMessaging::SW_MAXIMIZE)
                .ok()
                .map_err(|e| anyhow::anyhow!("Failed to maximize window: {}", e))?;
            Ok(())
        }
    }

    fn restore_window_inner(&self, hwnd: HWND) -> Result<()> {
        unsafe {
            ShowWindow(hwnd, SW_RESTORE)
                .ok()
                .map_err(|e| anyhow::anyhow!("Failed to restore window: {}", e))?;
            Ok(())
        }
    }

    fn close_window_inner(&self, hwnd: HWND) -> Result<()> {
        unsafe {
            use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_CLOSE};
            PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0))
                .map_err(|e| anyhow::anyhow!("Failed to post WM_CLOSE: {}", e))?;
            Ok(())
        }
    }

    fn set_topmost_inner(&self, hwnd: HWND, topmost: bool) -> Result<()> {
        unsafe {
            use windows::Win32::UI::WindowsAndMessaging::{
                SetWindowPos, HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOMOVE, SWP_NOSIZE,
            };
            let pos = if topmost {
                Some(HWND_TOPMOST)
            } else {
                Some(HWND_NOTOPMOST)
            };
            let _ = SetWindowPos(hwnd, pos, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE);
            Ok(())
        }
    }

    fn is_topmost_inner(&self, hwnd: HWND) -> bool {
        unsafe {
            use windows::Win32::UI::WindowsAndMessaging::WS_EX_TOPMOST;
            use windows::Win32::UI::WindowsAndMessaging::{
                GetWindowLongW, IsWindow, GWL_EXSTYLE,
            };

            if !IsWindow(Some(hwnd)).as_bool() {
                return false;
            }

            let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
            (ex_style as u32) & WS_EX_TOPMOST.0 != 0
        }
    }
}

impl Default for RealWindowApi {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowApiBase for RealWindowApi {
    type WindowId = HWND;

    fn window_id_to_usize(id: Self::WindowId) -> usize {
        id.0 as usize
    }

    fn usize_to_window_id(id: usize) -> Self::WindowId {
        HWND(id as *mut std::ffi::c_void)
    }

    fn get_foreground_window_inner(&self) -> Option<Self::WindowId> { self.get_foreground_window_inner() }
    fn set_window_pos_inner(&self, w: Self::WindowId, x: i32, y: i32, wd: i32, h: i32) -> Result<()> { self.set_window_pos_inner(w, x, y, wd, h) }
    fn minimize_window_inner(&self, w: Self::WindowId) -> Result<()> { self.minimize_window_inner(w) }
    fn maximize_window_inner(&self, w: Self::WindowId) -> Result<()> { self.maximize_window_inner(w) }
    fn restore_window_inner(&self, w: Self::WindowId) -> Result<()> { self.restore_window_inner(w) }
    fn close_window_inner(&self, w: Self::WindowId) -> Result<()> { self.close_window_inner(w) }
    fn set_topmost_inner(&self, w: Self::WindowId, t: bool) -> Result<()> { self.set_topmost_inner(w, t) }
    fn is_topmost_inner(&self, w: Self::WindowId) -> bool { self.is_topmost_inner(w) }
    fn is_window_valid_inner(&self, w: Self::WindowId) -> bool { self.is_window_valid_inner(w) }
    fn is_minimized_inner(&self, w: Self::WindowId) -> bool { self.is_minimized_inner(w) }
    fn is_maximized_inner(&self, w: Self::WindowId) -> bool { self.is_maximized_inner(w) }

    fn get_window_info(&self, window: Self::WindowId) -> Result<WindowInfo> {
        let title = self.get_window_title(window).unwrap_or_default();
        let frame = self
            .get_window_rect(window)
            .ok_or_else(|| anyhow::anyhow!("Failed to get window rect"))?;
        let process_id = unsafe {
            let mut pid: u32 = 0;
            windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(
                window,
                Some(&mut pid),
            );
            pid
        };
        let process_name =
            WindowsPlatform::get_process_name_by_pid(process_id).unwrap_or_default();
        let executable_path =
            WindowsPlatform::get_executable_path_by_pid(process_id).ok();

        Ok(WindowInfo {
            id: window.0 as usize,
            title,
            process_name,
            executable_path,
            x: frame.x,
            y: frame.y,
            width: frame.width,
            height: frame.height,
        })
    }

    fn get_monitors(&self) -> Vec<MonitorInfo> {
        unsafe { super::window_manager::enumerate_all_monitors() }
    }

    fn move_to_monitor(
        &self,
        window: Self::WindowId,
        monitor_index: usize,
    ) -> Result<()> {
        let monitors = self.get_monitors();
        let monitor = monitors.get(monitor_index).ok_or_else(|| {
            anyhow::anyhow!("Monitor index {} out of range", monitor_index)
        })?;

        let info = self.get_window_info(window)?;

        let new_x = monitor.x + (monitor.width - info.width) / 2;
        let new_y = monitor.y + (monitor.height - info.height) / 2;

        unsafe {
            SetWindowPos(
                window,
                Some(HWND_TOP),
                new_x,
                new_y,
                0,
                0,
                SWP_NOACTIVATE | SWP_NOOWNERZORDER | SWP_NOSIZE | SWP_NOZORDER,
            )
            .map_err(|e| anyhow::anyhow!("SetWindowPos failed: {e}"))?;
        }

        Ok(())
    }

}

#[cfg(test)]
pub type MockWindowApi = crate::platform::mock::MockWindowApi<HWND>;

// ============================================================================
// Window Event Hook
// ============================================================================

use crate::platform::traits::PlatformWindowEvent;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Sender;
use std::sync::Arc;
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

// SAFETY: The hook is only used on the thread that created it
// (the window message loop thread). HWINEVENTHOOK is a handle
// that Windows manages per-thread.
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
    pub fn start_with_shutdown_inner(&mut self, shutdown_flag: Arc<AtomicBool>) -> Result<()> {
        self.shutdown_flag = shutdown_flag;
        self.start()
    }

    /// Stop window event monitoring
    pub fn stop_inner(&mut self) {
        if let Some(hook) = self.hook.take() {
            unsafe {
                let _ = UnhookWinEvent(hook);
            }
            debug!("Window event hook stopped");
        }
    }

    /// Get shutdown flag reference
    pub fn shutdown_flag_inner(&self) -> Arc<AtomicBool> {
        self.shutdown_flag.clone()
    }
}

impl Drop for WindowEventHook {
    fn drop(&mut self) {
        self.stop_inner();
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
    WindowsPlatform::get_process_name_by_pid(pid).unwrap_or_default()
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

    fn test_hwnd(value: usize) -> HWND {
        HWND(value as *mut core::ffi::c_void)
    }

    #[test]
    fn test_mock_window_api_basic() {
        let api = MockWindowApi::new();
        let hwnd = test_hwnd(1234);

        let frame = WindowFrame::new(100, 200, 800, 600);
        api.set_window_rect(hwnd, frame);

        let retrieved = api.get_window_rect(hwnd).unwrap();
        assert_eq!(retrieved.x, 100);
        assert_eq!(retrieved.y, 200);
        assert_eq!(retrieved.width, 800);
        assert_eq!(retrieved.height, 600);

        let ops = api.get_operations();
        assert_eq!(ops.len(), 1);
        assert!(matches!(ops[0], WindowApiCall::GetWindowRect { .. }));
    }

    #[test]
    fn test_mock_window_api_set_window_pos() {
        let api = MockWindowApi::new();
        let hwnd = test_hwnd(5678);

        api.set_window_pos(hwnd, 50, 100, 1024, 768).unwrap();

        let frame = api.get_window_rect(hwnd).unwrap();
        assert_eq!(frame.x, 50);
        assert_eq!(frame.y, 100);
        assert_eq!(frame.width, 1024);
        assert_eq!(frame.height, 768);
    }

    #[test]
    fn test_mock_window_api_window_state() {
        let api = MockWindowApi::new();
        let hwnd = test_hwnd(9999);

        assert!(!api.is_minimized_inner(hwnd));
        assert!(!api.is_maximized_inner(hwnd));

        api.minimize_window(hwnd).unwrap();
        assert!(api.is_minimized_inner(hwnd));
        assert!(!api.is_maximized_inner(hwnd));

        api.restore_window(hwnd).unwrap();
        assert!(!api.is_minimized_inner(hwnd));
        assert!(!api.is_maximized_inner(hwnd));

        api.maximize_window(hwnd).unwrap();
        assert!(!api.is_minimized_inner(hwnd));
        assert!(api.is_maximized_inner(hwnd));
    }

    #[test]
    fn test_mock_window_api_foreground_window() {
        let api = MockWindowApi::new();
        let hwnd = test_hwnd(1111);

        assert!(api.get_foreground_window().is_none());

        api.set_foreground_window(hwnd);
        assert_eq!(api.get_foreground_window().unwrap().0 as usize, 1111);
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_window_event_hook_creation() {
        let (tx, _rx) = std::sync::mpsc::channel();
        let hook = WindowEventHook::new(tx);
        drop(hook);
    }
}
