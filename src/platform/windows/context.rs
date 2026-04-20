use tracing::debug;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::ProcessStatus::GetModuleBaseNameW;
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClassNameW, GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId,
};

use crate::config::wildcard_match;

/// 窗口上下文信息
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct WindowContext {
    /// 窗口句柄
    pub hwnd: isize,
    /// 窗口类名
    pub window_class: String,
    /// Window title
    pub window_title: String,
    /// 进程 ID
    pub process_id: u32,
    /// 进程名
    pub process_name: String,
    /// 可执行文件路径
    pub executable_path: String,
}

#[allow(dead_code)]
impl WindowContext {
    /// 获取当前前台窗口的上下文
    pub fn get_current() -> Option<Self> {
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.0 == 0 {
                return None;
            }

            let mut context = Self {
                hwnd: hwnd.0,
                ..Default::default()
            };

            // 获取窗口类名
            let mut class_name = [0u16; 256];
            let len = GetClassNameW(hwnd, &mut class_name);
            if len > 0 {
                context.window_class =
                    String::from_utf16_lossy(&class_name[..len as usize]);
            }

            // 获取窗口标题
            let mut title = [0u16; 512];
            let len = GetWindowTextW(hwnd, &mut title);
            if len > 0 {
                context.window_title = String::from_utf16_lossy(&title[..len as usize]);
            }

            // 获取进程 ID
            let mut process_id: u32 = 0;
            GetWindowThreadProcessId(hwnd, Some(&mut process_id));
            context.process_id = process_id;

            // 获取进程名
            context.process_name = Self::get_process_name_by_pid(process_id)
                .unwrap_or_else(|| format!("pid:{}", process_id));

            // 获取可执行文件路径
            context.executable_path =
                Self::get_executable_path_by_pid(process_id).unwrap_or_default();

            debug!(
                "Current window: class={}, title={}, process={}, path={}",
                context.window_class,
                context.window_title,
                context.process_name,
                context.executable_path
            );

            Some(context)
        }
    }

    /// 通过进程 ID 获取进程名
    unsafe fn get_process_name_by_pid(pid: u32) -> Option<String> {
        let handle =
            OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid).ok()?;

        let mut buffer = [0u16; 260];
        let len = GetModuleBaseNameW(handle, None, &mut buffer);

        CloseHandle(handle).ok();

        if len == 0 {
            return None;
        }

        Some(String::from_utf16_lossy(&buffer[..len as usize]))
    }

    /// 通过进程 ID 获取可执行文件路径
    unsafe fn get_executable_path_by_pid(pid: u32) -> Option<String> {
        use windows::Win32::System::ProcessStatus::GetModuleFileNameExW;

        let handle =
            OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid).ok()?;

        let mut buffer = [0u16; 260];
        let len = GetModuleFileNameExW(handle, None, &mut buffer);

        CloseHandle(handle).ok();

        if len == 0 {
            return None;
        }

        Some(String::from_utf16_lossy(&buffer[..len as usize]))
    }

    /// 检查是否匹配给定的上下文条件
    #[allow(dead_code)]
    pub fn matches(
        &self,
        window_class: Option<&str>,
        process_name: Option<&str>,
        window_title: Option<&str>,
        executable_path: Option<&str>,
    ) -> bool {
        if let Some(pattern) = window_class {
            if !wildcard_match(&self.window_class, pattern) {
                return false;
            }
        }

        if let Some(pattern) = process_name {
            if !wildcard_match(&self.process_name, pattern) {
                return false;
            }
        }

        if let Some(pattern) = window_title {
            if !wildcard_match(&self.window_title, pattern) {
                return false;
            }
        }

        if let Some(pattern) = executable_path {
            if !wildcard_match(&self.executable_path, pattern) {
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_current() {
        // Test能否获取当前窗口
        let context = WindowContext::get_current();
        assert!(context.is_some());

        let ctx = context.unwrap();
        // 验证基本字段已填充
        assert!(!ctx.window_class.is_empty() || !ctx.window_title.is_empty());
    }

    #[test]
    fn test_matches() {
        let context = WindowContext {
            hwnd: 0,
            window_class: "MozillaWindowClass".to_string(),
            window_title: "Firefox".to_string(),
            process_id: 1234,
            process_name: "firefox.exe".to_string(),
            executable_path: "C:\\Program Files\\Firefox\\firefox.exe".to_string(),
        };

        // Test进程名匹配
        assert!(context.matches(None, Some("firefox.exe"), None, None));
        assert!(context.matches(None, Some("*.exe"), None, None));
        assert!(!context.matches(None, Some("chrome.exe"), None, None));

        // Test窗口类名匹配
        assert!(context.matches(Some("Mozilla*"), None, None, None));
        assert!(!context.matches(Some("Chrome*"), None, None, None));

        // Test窗口标题匹配
        assert!(context.matches(None, None, Some("Fire*"), None));

        // Test路径匹配
        assert!(context.matches(None, None, None, Some("*Firefox*")));
    }
}
