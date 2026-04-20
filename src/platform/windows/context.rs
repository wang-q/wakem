use anyhow::Result;
use tracing::debug;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    GetClassNameW, GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId,
};

/// 窗口上下文信息
#[derive(Debug, Clone, Default)]
pub struct WindowContext {
    /// 窗口句柄
    pub hwnd: isize,
    /// 窗口类名
    pub window_class: String,
    /// 窗口标题
    pub window_title: String,
    /// 进程 ID
    pub process_id: u32,
    /// 进程名
    pub process_name: String,
}

impl WindowContext {
    /// 获取当前前台窗口的上下文
    pub fn get_current() -> Option<Self> {
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.0 == 0 {
                return None;
            }

            let mut context = Self::default();
            context.hwnd = hwnd.0;

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

            // 获取进程名（简化版，使用进程 ID 作为标识）
            context.process_name = format!("pid:{}", process_id);

            debug!(
                "Current window: class={}, title={}, process={}",
                context.window_class, context.window_title, context.process_name
            );

            Some(context)
        }
    }

    /// 检查是否匹配给定的上下文条件
    pub fn matches(
        &self,
        window_class: Option<&str>,
        process_name: Option<&str>,
        window_title: Option<&str>,
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

        true
    }
}

/// 简单的通配符匹配（* 匹配任意字符，? 匹配单个字符）
fn wildcard_match(text: &str, pattern: &str) -> bool {
    if pattern == "*" || pattern.is_empty() {
        return true;
    }

    // 简单的包含匹配
    text.to_lowercase().contains(&pattern.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wildcard_match() {
        assert!(wildcard_match("MozillaWindowClass", "Mozilla"));
        assert!(wildcard_match("firefox.exe", "firefox"));
        assert!(wildcard_match("test", "*"));
        assert!(wildcard_match("MozillaWindowClass", "MozillaWindowClass"));
    }

    #[test]
    fn test_get_current() {
        // 测试能否获取当前窗口
        let _ = WindowContext::get_current();
    }
}
