//! Windows window context implementation
#![cfg(target_os = "windows")]

use crate::platform::traits::WindowContext;
use tracing::debug;
use windows::Win32::UI::WindowsAndMessaging::{
    GetClassNameW, GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId,
};

/// Get current foreground window context
pub fn get_current() -> Option<WindowContext> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return None;
        }

        let mut window_class = String::new();
        let mut window_title = String::new();
        let mut process_id: u32 = 0;

        // Get window class name
        let mut class_name = [0u16; 256];
        let len = GetClassNameW(hwnd, &mut class_name);
        if len > 0 {
            window_class = String::from_utf16_lossy(&class_name[..len as usize]);
        }

        // Get window title
        let mut title = [0u16; 512];
        let len = GetWindowTextW(hwnd, &mut title);
        if len > 0 {
            window_title = String::from_utf16_lossy(&title[..len as usize]);
        }

        // Get process ID
        GetWindowThreadProcessId(hwnd, Some(&mut process_id));

        // Get process name
        let process_name = super::get_process_name_by_pid(process_id)
            .unwrap_or_else(|_| format!("pid:{}", process_id));

        // Get executable file path
        let executable_path = super::get_executable_path_by_pid(process_id).ok();

        debug!(
            "Current window: class={}, title={}, process={}, path={}",
            window_class,
            window_title,
            process_name,
            executable_path.as_deref().unwrap_or("unknown")
        );

        Some(WindowContext {
            process_name,
            window_class,
            window_title,
            executable_path,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "windows")]
    fn test_get_current() {
        let context = get_current();
        assert!(context.is_some());

        let ctx = context.unwrap();
        assert!(!ctx.window_class.is_empty() || !ctx.window_title.is_empty());
    }

    #[test]
    fn test_matches() {
        let ctx = WindowContext {
            process_name: "firefox.exe".to_string(),
            window_class: "MozillaWindowClass".to_string(),
            window_title: "Firefox".to_string(),
            executable_path: Some("C:\\Program Files\\Firefox\\firefox.exe".to_string()),
        };

        // Test process name matching
        assert!(ctx.matches(Some("firefox.exe"), None, None, None));
        assert!(ctx.matches(Some("*.exe"), None, None, None));
        assert!(!ctx.matches(Some("chrome.exe"), None, None, None));

        // Test window class name matching
        assert!(ctx.matches(None, Some("Mozilla*"), None, None));
        assert!(!ctx.matches(None, Some("Chrome*"), None, None));

        // Test window title matching
        assert!(ctx.matches(None, None, Some("Fire*"), None));

        // Test path matching
        assert!(ctx.matches(None, None, None, Some("*Firefox*")));
    }
}
