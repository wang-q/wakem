//! Windows window API implementation
#![cfg(target_os = "windows")]

use anyhow::Result;
use windows::Win32::Foundation::{HWND, LPARAM, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{MonitorFromWindow, MONITOR_DEFAULTTONEAREST};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowRect, IsIconic, IsWindow, IsZoomed, SetWindowPos,
    ShowWindow, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOOWNERZORDER, SW_RESTORE,
};

#[cfg(test)]
use crate::platform::mock::WindowApiCall;
use crate::platform::traits::{
    MonitorInfo, MonitorWorkArea, WindowApiBase, WindowFrame, WindowInfo,
};

/// Real Windows API implementation
pub struct RealWindowApi;

impl RealWindowApi {
    pub fn new() -> Self {
        Self
    }

    fn get_foreground_window(&self) -> Option<HWND> {
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

    fn set_window_pos(
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

    fn get_monitor_info(&self, hwnd: HWND) -> Option<MonitorInfo> {
        unsafe {
            let hmonitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
            if hmonitor.is_invalid() {
                return None;
            }

            use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MONITORINFO};
            let mut monitor_info = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };

            if !GetMonitorInfoW(hmonitor, &mut monitor_info).as_bool() {
                return None;
            }

            let rect = &monitor_info.rcMonitor;
            Some(MonitorInfo {
                x: rect.left,
                y: rect.top,
                width: rect.right - rect.left,
                height: rect.bottom - rect.top,
            })
        }
    }

    fn get_monitor_work_area(&self, hwnd: HWND) -> Option<MonitorWorkArea> {
        unsafe {
            let hmonitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
            if hmonitor.is_invalid() {
                return None;
            }

            use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MONITORINFO};
            let mut monitor_info = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };

            if !GetMonitorInfoW(hmonitor, &mut monitor_info).as_bool() {
                return None;
            }

            let work_area = &monitor_info.rcWork;
            Some(MonitorWorkArea {
                x: work_area.left,
                y: work_area.top,
                width: work_area.right - work_area.left,
                height: work_area.bottom - work_area.top,
            })
        }
    }

    fn is_window(&self, hwnd: HWND) -> bool {
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

    fn is_iconic(&self, hwnd: HWND) -> bool {
        unsafe { IsIconic(hwnd).as_bool() }
    }

    fn is_zoomed(&self, hwnd: HWND) -> bool {
        unsafe { IsZoomed(hwnd).as_bool() }
    }

    fn minimize_window(&self, hwnd: HWND) -> Result<()> {
        unsafe {
            ShowWindow(hwnd, windows::Win32::UI::WindowsAndMessaging::SW_MINIMIZE)
                .ok()
                .map_err(|e| anyhow::anyhow!("Failed to minimize window: {}", e))?;
            Ok(())
        }
    }

    fn maximize_window(&self, hwnd: HWND) -> Result<()> {
        unsafe {
            ShowWindow(hwnd, windows::Win32::UI::WindowsAndMessaging::SW_MAXIMIZE)
                .ok()
                .map_err(|e| anyhow::anyhow!("Failed to maximize window: {}", e))?;
            Ok(())
        }
    }

    fn restore_window(&self, hwnd: HWND) -> Result<()> {
        unsafe {
            ShowWindow(hwnd, SW_RESTORE)
                .ok()
                .map_err(|e| anyhow::anyhow!("Failed to restore window: {}", e))?;
            Ok(())
        }
    }

    fn close_window(&self, hwnd: HWND) -> Result<()> {
        unsafe {
            use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_CLOSE};
            PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0))
                .map_err(|e| anyhow::anyhow!("Failed to post WM_CLOSE: {}", e))?;
            Ok(())
        }
    }

    fn set_topmost(&self, hwnd: HWND, topmost: bool) -> Result<()> {
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

    fn is_topmost(&self, hwnd: HWND) -> bool {
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

    fn ensure_window_restored(&self, hwnd: HWND) -> Result<()> {
        if self.is_iconic(hwnd) || self.is_zoomed(hwnd) {
            self.restore_window(hwnd)?;
        }
        Ok(())
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

    fn get_foreground_window(&self) -> Option<Self::WindowId> {
        self.get_foreground_window()
    }

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
            super::get_process_name_by_pid(process_id).unwrap_or_default();
        let executable_path = super::get_executable_path_by_pid(process_id).ok();

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

    fn set_window_pos(
        &self,
        window: Self::WindowId,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<()> {
        self.set_window_pos(window, x, y, width, height)
    }

    fn minimize_window(&self, window: Self::WindowId) -> Result<()> {
        self.minimize_window(window)
    }

    fn maximize_window(&self, window: Self::WindowId) -> Result<()> {
        self.maximize_window(window)
    }

    fn restore_window(&self, window: Self::WindowId) -> Result<()> {
        self.restore_window(window)
    }

    fn close_window(&self, window: Self::WindowId) -> Result<()> {
        self.close_window(window)
    }

    fn set_topmost(&self, window: Self::WindowId, topmost: bool) -> Result<()> {
        self.set_topmost(window, topmost)
    }

    fn is_topmost(&self, window: Self::WindowId) -> bool {
        self.is_topmost(window)
    }

    fn get_monitors(&self) -> Vec<MonitorInfo> {
        unsafe { super::window_manager::enumerate_all_monitors() }
    }

    fn move_to_monitor(
        &self,
        _window: Self::WindowId,
        _monitor_index: usize,
    ) -> Result<()> {
        Ok(())
    }

    fn is_window_valid(&self, window: Self::WindowId) -> bool {
        self.is_window(window)
    }

    fn is_minimized(&self, window: Self::WindowId) -> bool {
        self.is_iconic(window)
    }

    fn is_maximized(&self, window: Self::WindowId) -> bool {
        self.is_zoomed(window)
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

        assert!(!api.is_iconic(hwnd));
        assert!(!api.is_zoomed(hwnd));

        api.minimize_window(hwnd).unwrap();
        assert!(api.is_iconic(hwnd));
        assert!(!api.is_zoomed(hwnd));

        api.restore_window(hwnd).unwrap();
        assert!(!api.is_iconic(hwnd));
        assert!(!api.is_zoomed(hwnd));

        api.maximize_window(hwnd).unwrap();
        assert!(!api.is_iconic(hwnd));
        assert!(api.is_zoomed(hwnd));
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
