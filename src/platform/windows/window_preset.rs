//! Windows window preset implementation
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
            super::WindowsPlatform::get_process_name_by_pid(pid).unwrap_or_default()
        };
        let executable_path = unsafe {
            let pid = get_window_process_id(window)?;
            super::WindowsPlatform::get_executable_path_by_pid(pid).ok()
        };
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

pub type WindowPresetManager = CommonWindowPresetManager<
    WindowManager<crate::platform::windows::window_api::RealWindowApi>,
>;

impl Default for WindowPresetManager {
    fn default() -> Self {
        Self::new(WindowManager::new())
    }
}
