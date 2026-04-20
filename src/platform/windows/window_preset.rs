use anyhow::Result;
use tracing::debug;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

use crate::config::WindowPreset;
use crate::platform::windows::window_manager::RealWindowManager;
use crate::platform::windows::WindowFrame;

/// Window preset管理器
#[allow(dead_code)]
pub struct WindowPresetManager {
    presets: Vec<WindowPreset>,
    window_manager: RealWindowManager,
}

#[allow(dead_code)]
impl WindowPresetManager {
    /// 创建新的预设管理器
    pub fn new() -> Self {
        Self {
            presets: Vec::new(),
            window_manager: RealWindowManager::new(),
        }
    }

    /// 从配置加载预设
    pub fn load_presets(&mut self, presets: Vec<WindowPreset>) {
        self.presets = presets;
        debug!("Loaded {} window presets", self.presets.len());
    }

    /// Save current window as preset
    pub fn save_preset(
        &mut self,
        name: &str,
        hwnd: HWND,
        process_name: Option<String>,
        executable_path: Option<String>,
        title_pattern: Option<String>,
    ) -> Result<()> {
        let info = self.window_manager.get_window_info(hwnd)?;

        // 如果未提供进程名，尝试自动获取
        let process_name = process_name.or_else(|| unsafe {
            let pid = Self::get_window_process_id(hwnd).ok()?;
            Self::get_process_name_by_pid(pid).ok()
        });

        // 如果未提供可执行路径，尝试自动获取
        let executable_path = executable_path
            .or_else(|| unsafe { Self::get_window_executable_path(hwnd).ok() });

        // 检查是否已存在同名预设
        if let Some(existing) = self.presets.iter_mut().find(|p| p.name == name) {
            // 更新现有预设
            existing.process_name = process_name;
            existing.executable_path = executable_path;
            existing.title_pattern = title_pattern;
            existing.x = info.frame.x;
            existing.y = info.frame.y;
            existing.width = info.frame.width;
            existing.height = info.frame.height;
            debug!("Updated preset '{}' for window {:?}", name, hwnd);
        } else {
            // 创建新预设
            let preset = WindowPreset {
                name: name.to_string(),
                process_name,
                executable_path,
                title_pattern,
                x: info.frame.x,
                y: info.frame.y,
                width: info.frame.width,
                height: info.frame.height,
            };
            self.presets.push(preset);
            debug!("Created new preset '{}' for window {:?}", name, hwnd);
        }

        Ok(())
    }

    /// 加载指定预设到窗口
    pub fn load_preset(&self, name: &str, hwnd: HWND) -> Result<()> {
        let preset = self
            .presets
            .iter()
            .find(|p| p.name == name)
            .ok_or_else(|| anyhow::anyhow!("Preset '{}' not found", name))?;

        let frame = WindowFrame::new(preset.x, preset.y, preset.width, preset.height);
        self.window_manager.set_window_frame(hwnd, &frame)?;

        debug!(
            "Applied preset '{}' to window {:?}: {:?}",
            name, hwnd, frame
        );
        Ok(())
    }

    /// Apply matching preset to current window
    pub fn apply_preset_for_window(&self, hwnd: HWND) -> Result<bool> {
        let info = self.window_manager.get_window_info(hwnd)?;

        // 获取窗口的进程信息
        let (process_name, executable_path) = unsafe {
            let pid = Self::get_window_process_id(hwnd)?;
            let proc_name = Self::get_process_name_by_pid(pid).unwrap_or_default();
            let exec_path = Self::get_window_executable_path(hwnd).unwrap_or_default();
            (proc_name, Some(exec_path))
        };

        // 查找匹配的预设
        for preset in &self.presets {
            if preset.matches(&process_name, executable_path.as_deref(), &info.title) {
                let frame =
                    WindowFrame::new(preset.x, preset.y, preset.width, preset.height);
                self.window_manager.set_window_frame(hwnd, &frame)?;
                debug!(
                    "Auto-applied preset '{}' to window '{}' ({:?})",
                    preset.name, info.title, hwnd
                );
                return Ok(true);
            }
        }

        debug!(
            "No matching preset found for window '{}' (process: {})",
            info.title, process_name
        );
        Ok(false)
    }

    /// 获取窗口进程ID
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

    /// 通过进程ID获取进程名
    unsafe fn get_process_name_by_pid(pid: u32) -> Result<String> {
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::System::ProcessStatus::GetModuleBaseNameW;
        use windows::Win32::System::Threading::{
            OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ,
        };

        let handle =
            OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid)
                .map_err(|e| anyhow::anyhow!("Failed to open process: {}", e))?;

        let mut buffer = [0u16; 260];
        let len = GetModuleBaseNameW(handle, None, &mut buffer);

        CloseHandle(handle).ok();

        if len == 0 {
            return Err(anyhow::anyhow!("Failed to get process name"));
        }

        Ok(String::from_utf16_lossy(&buffer[..len as usize]))
    }

    /// 获取窗口可执行文件路径
    unsafe fn get_window_executable_path(hwnd: HWND) -> Result<String> {
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::System::ProcessStatus::GetModuleFileNameExW;
        use windows::Win32::System::Threading::{
            OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ,
        };

        let pid = Self::get_window_process_id(hwnd)?;

        let handle =
            OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid)
                .map_err(|e| anyhow::anyhow!("Failed to open process: {}", e))?;

        let mut buffer = [0u16; 260];
        let len = GetModuleFileNameExW(handle, None, &mut buffer);

        CloseHandle(handle).ok();

        if len == 0 {
            return Err(anyhow::anyhow!("Failed to get executable path"));
        }

        Ok(String::from_utf16_lossy(&buffer[..len as usize]))
    }

    /// 获取前台窗口信息（用于保存预设时）
    pub fn get_foreground_window_info(
        &self,
    ) -> Result<(HWND, String, Option<String>, Option<String>)> {
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.0 == 0 {
                return Err(anyhow::anyhow!("No foreground window"));
            }

            let info = self.window_manager.get_window_info(hwnd)?;
            let pid = Self::get_window_process_id(hwnd)?;
            let process_name = Self::get_process_name_by_pid(pid).ok();
            let executable_path = Self::get_window_executable_path(hwnd).ok();

            Ok((hwnd, info.title, process_name, executable_path))
        }
    }
}

impl Default for WindowPresetManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
