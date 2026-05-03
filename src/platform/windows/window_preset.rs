//! Windows window preset implementation
#![cfg(target_os = "windows")]

use crate::platform::common::window_preset::{
    WindowPresetApi, WindowPresetManager as CommonWindowPresetManager,
};
use crate::platform::traits::WindowPresetManager as WindowPresetManagerTrait;
use crate::platform::types::{WindowId, WindowInfo};
use crate::platform::windows::window_api::WindowApi;
use crate::platform::windows::window_manager::WindowManager;
use anyhow::Result;
use windows::Win32::Foundation::HWND;

impl WindowPresetApi for WindowManager {
    type WindowId = HWND;

    fn get_foreground_window(&self) -> Option<Self::WindowId> {
        use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.0.is_null() {
                None
            } else {
                Some(hwnd)
            }
        }
    }

    fn get_window_info(&self, window: Self::WindowId) -> Result<WindowInfo> {
        let title = self.api.get_window_title(window).unwrap_or_default();
        let frame = self.api.get_window_rect(window)?;
        let process_name = unsafe {
            match get_window_process_id(window) {
                Ok(pid) => super::get_process_name_by_pid(pid).unwrap_or_default(),
                Err(_) => String::new(),
            }
        };
        let executable_path = unsafe { get_window_executable_path(window).ok() };

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
        w: i32,
        h: i32,
    ) -> Result<()> {
        self.api.ensure_window_restored(window)?;
        self.api.set_window_pos(window, x, y, w, h)
    }

    fn minimize_window(&self, window: Self::WindowId) -> Result<()> {
        self.api.minimize_window(window)
    }

    fn maximize_window(&self, window: Self::WindowId) -> Result<()> {
        self.api.maximize_window(window)
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

unsafe fn get_window_executable_path(hwnd: HWND) -> Result<String> {
    let pid = get_window_process_id(hwnd)?;
    super::get_executable_path_by_pid(pid)
}

pub type WindowPresetManager = CommonWindowPresetManager<WindowManager>;

impl Default for WindowPresetManager {
    fn default() -> Self {
        Self::new(WindowManager::new())
    }
}

impl WindowPresetManagerTrait for WindowPresetManager {
    fn load_presets(&mut self, presets: Vec<crate::config::WindowPreset>) {
        self.load_presets(presets)
    }

    fn save_preset(&mut self, name: String) -> Result<()> {
        self.save_preset(name)
    }

    fn load_preset(&self, name: &str) -> Result<()> {
        self.load_preset(name)
    }

    fn get_foreground_window_info(&self) -> Option<Result<WindowInfo>> {
        self.get_foreground_window_info()
    }

    fn apply_preset_for_window_by_id(&self, window_id: WindowId) -> Result<bool> {
        let hwnd = HWND(window_id as *mut core::ffi::c_void);
        self.apply_preset_for_window_by_id(hwnd)
    }
}
