//! Windows window preset implementation
//!
//! Provides window preset functionality for saving, loading, and automatically
//! applying window layouts based on configuration rules.
//!
//! The core logic lives in [crate::platform::window_preset_common::WindowPresetManager];
//! this module adds the Windows-specific [WindowPresetApi] implementation
//! built on top of [WindowManager].
#![cfg(target_os = "windows")]

use crate::platform::traits::WindowInfo;
use crate::platform::window_preset_common::{
    WindowPresetApi, WindowPresetManager as CommonWindowPresetManager,
};
use crate::platform::windows::window_api::WindowApi;
use crate::platform::windows::window_manager::WindowManager;
use anyhow::Result;
use windows::Win32::Foundation::HWND;

impl<A: WindowApi> WindowPresetApi for WindowManager<A> {
    type WindowId = HWND;

    fn get_foreground_window(&self) -> Option<Self::WindowId> {
        self.api().get_foreground_window()
    }

    fn get_window_info(&self, window: Self::WindowId) -> Result<WindowInfo> {
        let info = self.get_window_info(window)?;
        let process_name = unsafe {
            let pid = get_window_process_id(window)?;
            get_process_name_by_pid(pid).unwrap_or_default()
        };
        let executable_path = unsafe { get_window_executable_path(window).ok() };
        Ok(WindowInfo {
            id: window.0 as usize,
            title: info.title,
            process_name,
            executable_path,
            x: info.frame.x,
            y: info.frame.y,
            width: info.frame.width,
            height: info.frame.height,
        })
    }

    fn set_window_pos(
        &self,
        window: Self::WindowId,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
    ) -> Result<()> {
        let frame = crate::platform::traits::WindowFrame::new(x, y, w, h);
        self.set_window_frame(window, &frame)
    }

    fn minimize_window(&self, window: Self::WindowId) -> Result<()> {
        self.api().minimize_window(window)
    }

    fn maximize_window(&self, window: Self::WindowId) -> Result<()> {
        self.api().maximize_window(window)
    }
}

unsafe fn get_window_process_id(hwnd: HWND) -> Result<u32> {
    let mut pid: u32 = 0;
    windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(
        hwnd,
        Some(&mut pid),
    );
    if pid == 0 {
        return Err(anyhow::anyhow!("Failed to get process ID"));
    }
    Ok(pid)
}

unsafe fn get_process_name_by_pid(pid: u32) -> Result<String> {
    super::get_process_name_by_pid(pid)
}

unsafe fn get_window_executable_path(hwnd: HWND) -> Result<String> {
    let pid = get_window_process_id(hwnd)?;
    super::get_executable_path_by_pid(pid)
}

/// Windows window preset manager (type alias for the common manager)
pub type WindowPresetManager = CommonWindowPresetManager<
    WindowManager<crate::platform::windows::window_api::RealWindowApi>,
>;

impl Default for WindowPresetManager {
    fn default() -> Self {
        Self::new(WindowManager::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::WindowPreset;

    fn test_hwnd(value: usize) -> HWND {
        HWND(value as *mut core::ffi::c_void)
    }

    #[test]
    fn test_preset_matches() {
        let preset = WindowPreset {
            name: "test".to_string(),
            process_name: Some("chrome.exe".to_string()),
            executable_path: None,
            title_pattern: None,
            x: 0,
            y: 0,
            width: 800,
            height: 600,
        };

        assert!(preset.matches("chrome.exe", None, "Google Chrome"));
        assert!(!preset.matches("firefox.exe", None, "Firefox"));
    }

    #[test]
    fn test_preset_wildcard_match() {
        let preset = WindowPreset {
            name: "test".to_string(),
            process_name: Some("*.exe".to_string()),
            executable_path: None,
            title_pattern: Some("*Chrome*".to_string()),
            x: 0,
            y: 0,
            width: 800,
            height: 600,
        };

        assert!(preset.matches("chrome.exe", None, "Google Chrome"));
        assert!(preset.matches("notepad.exe", None, "Chrome Extension"));
        assert!(!preset.matches("chrome.exe", None, "Firefox"));
    }
}
