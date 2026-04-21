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

/// Window context information
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct WindowContext {
    /// Window handle
    pub hwnd: isize,
    /// Window class name
    pub window_class: String,
    /// Window title
    pub window_title: String,
    /// Process ID
    pub process_id: u32,
    /// Process name
    pub process_name: String,
    /// Executable file path
    pub executable_path: String,
}

#[allow(dead_code)]
impl WindowContext {
    /// Get current foreground window context
    pub fn get_current() -> Option<Self> {
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.0.is_null() {
                return None;
            }

            let mut context = Self {
                hwnd: hwnd.0 as isize,
                ..Default::default()
            };

            // Get window class name
            let mut class_name = [0u16; 256];
            let len = GetClassNameW(hwnd, &mut class_name);
            if len > 0 {
                context.window_class =
                    String::from_utf16_lossy(&class_name[..len as usize]);
            }

            // Get window title
            let mut title = [0u16; 512];
            let len = GetWindowTextW(hwnd, &mut title);
            if len > 0 {
                context.window_title = String::from_utf16_lossy(&title[..len as usize]);
            }

            // Get process ID
            let mut process_id: u32 = 0;
            GetWindowThreadProcessId(hwnd, Some(&mut process_id));
            context.process_id = process_id;

            // Get process name
            context.process_name = Self::get_process_name_by_pid(process_id)
                .unwrap_or_else(|| format!("pid:{}", process_id));

            // Get executable file path
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

    /// Get process name by process ID
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

    /// Get executable path by process ID
    unsafe fn get_executable_path_by_pid(pid: u32) -> Option<String> {
        use windows::Win32::System::ProcessStatus::GetModuleFileNameExW;

        let handle =
            OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid).ok()?;

        let mut buffer = [0u16; 260];
        let len = GetModuleFileNameExW(Some(handle), None, &mut buffer);

        CloseHandle(handle).ok();

        if len == 0 {
            return None;
        }

        Some(String::from_utf16_lossy(&buffer[..len as usize]))
    }

    /// Check if matches given context conditions
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
        // Test if can get current window
        let context = WindowContext::get_current();
        assert!(context.is_some());

        let ctx = context.unwrap();
        // Verify basic fields are populated
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

        // Test process name matching
        assert!(context.matches(None, Some("firefox.exe"), None, None));
        assert!(context.matches(None, Some("*.exe"), None, None));
        assert!(!context.matches(None, Some("chrome.exe"), None, None));

        // Test window class name matching
        assert!(context.matches(Some("Mozilla*"), None, None, None));
        assert!(!context.matches(Some("Chrome*"), None, None, None));

        // Test window title matching
        assert!(context.matches(None, None, Some("Fire*"), None));

        // Test path matching
        assert!(context.matches(None, None, None, Some("*Firefox*")));
    }
}
