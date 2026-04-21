//! macOS window context implementation
//!
//! Provides information about the currently focused window including
//! process name, window title, and executable path.

use crate::config::wildcard_match;
use crate::platform::traits::WindowContext as WindowContextTrait;
use std::process::Command;
use tracing::debug;

/// macOS window context information
#[derive(Debug, Clone, Default)]
pub struct WindowContext {
    pub process_name: String,
    pub window_class: String,
    pub window_title: String,
    pub executable_path: Option<String>,
}

impl WindowContext {
    /// Create an empty context
    pub fn empty() -> Self {
        Self::default()
    }

    /// Get current window context as platform-agnostic type
    pub fn get_current() -> Option<WindowContextTrait> {
        let script = r#"
            tell application "System Events"
                set frontApp to first application process whose frontmost is true
                set appName to name of frontApp
                set appPath to POSIX path of (path to frontApp)
                try
                    set winTitle to name of first window of frontApp
                on error
                    set winTitle to ""
                end try
                return {appName, appPath, winTitle}
            end tell
        "#;

        let output = Command::new("osascript")
            .arg("-e")
            .arg(script)
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let result = String::from_utf8_lossy(&output.stdout);
        let parts: Vec<&str> = result.trim().split(", ").collect();

        if parts.len() >= 2 {
            let process_name = parts[0].to_string();
            let executable_path = Some(parts[1].to_string());
            let window_title = parts.get(2).map(|s| s.to_string()).unwrap_or_default();

            Some(WindowContextTrait {
                process_name,
                window_class: String::new(),
                window_title,
                executable_path,
            })
        } else {
            None
        }
    }

    /// Get process name by PID using `ps` command
    fn get_process_name_by_pid(pid: u32) -> Option<String> {
        let output = Command::new("ps")
            .args(["-p", &pid.to_string(), "-o", "comm="])
            .output()
            .ok()?;

        if output.status.success() {
            let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if name.is_empty() {
                None
            } else {
                Some(name)
            }
        } else {
            None
        }
    }

    /// Get executable path by PID using `ps` command
    fn get_executable_path_by_pid(pid: u32) -> Option<String> {
        let output = Command::new("ps")
            .args(["-p", &pid.to_string(), "-o", "exe="])
            .output()
            .ok()?;

        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if path.is_empty() {
                None
            } else {
                Some(path)
            }
        } else {
            None
        }
    }

    /// Get PID of frontmost application
    fn get_frontmost_pid() -> Option<u32> {
        let script = r#"tell application "System Events" to unix id of first application process whose frontmost is true"#;

        let output = Command::new("osascript")
            .arg("-e")
            .arg(script)
            .output()
            .ok()?;

        if output.status.success() {
            let pid_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            pid_str.parse::<u32>().ok()
        } else {
            None
        }
    }

    /// Convert to platform-agnostic context
    pub fn to_platform_context(&self) -> WindowContextTrait {
        WindowContextTrait {
            process_name: self.process_name.clone(),
            window_class: self.window_class.clone(),
            window_title: self.window_title.clone(),
            executable_path: self.executable_path.clone(),
        }
    }

    /// Check if matches given context conditions with wildcard support
    pub fn matches(
        &self,
        process_name: Option<&str>,
        window_class: Option<&str>,
        window_title: Option<&str>,
        executable_path: Option<&str>,
    ) -> bool {
        if let Some(pattern) = process_name {
            if !wildcard_match(&self.process_name, pattern) {
                return false;
            }
        }

        if let Some(pattern) = window_class {
            if !wildcard_match(&self.window_class, pattern) {
                return false;
            }
        }

        if let Some(pattern) = window_title {
            if !wildcard_match(&self.window_title, pattern) {
                return false;
            }
        }

        if let Some(pattern) = executable_path {
            match &self.executable_path {
                Some(path) if !wildcard_match(path, pattern) => return false,
                None => return false,
                _ => {}
            }
        }

        true
    }
}

/// Get current modifier state
pub fn get_modifier_state() -> crate::types::ModifierState {
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    let mut modifiers = crate::types::ModifierState::default();

    if let Ok(source) = CGEventSource::new(CGEventSourceStateID::HIDSystemState) {
        if let Ok(event) = core_graphics::event::CGEvent::new(source) {
            let flags = event.get_flags();

            if flags.contains(core_graphics::event::CGEventFlags::CGEventFlagShift) {
                modifiers.shift = true;
            }
            if flags.contains(core_graphics::event::CGEventFlags::CGEventFlagControl) {
                modifiers.ctrl = true;
            }
            if flags.contains(core_graphics::event::CGEventFlags::CGEventFlagAlternate) {
                modifiers.alt = true;
            }
            if flags.contains(core_graphics::event::CGEventFlags::CGEventFlagCommand) {
                modifiers.meta = true;
            }
        }
    }

    modifiers
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_context_empty() {
        let ctx = WindowContext::empty();
        assert!(ctx.process_name.is_empty());
        assert!(ctx.window_title.is_empty());
        assert!(ctx.window_class.is_empty());
        assert!(ctx.executable_path.is_none());
    }

    #[test]
    fn test_get_current() {
        let _ctx = WindowContext::get_current();
    }

    #[test]
    fn test_to_platform_context() {
        let ctx = WindowContext {
            process_name: "Safari".to_string(),
            window_class: String::new(),
            window_title: "Apple".to_string(),
            executable_path: Some("/Applications/Safari.app".to_string()),
        };

        let platform_ctx = ctx.to_platform_context();
        assert_eq!(platform_ctx.process_name, "Safari");
        assert_eq!(platform_ctx.window_title, "Apple");
        assert_eq!(
            platform_ctx.executable_path,
            Some("/Applications/Safari.app".into())
        );
    }

    #[test]
    fn test_matches_exact() {
        let ctx = WindowContext {
            process_name: "Safari".to_string(),
            window_class: String::new(),
            window_title: "Apple".to_string(),
            executable_path: Some("/Applications/Safari.app".to_string()),
        };

        // Exact match
        assert!(ctx.matches(Some("Safari"), None, None, None));
        assert!(ctx.matches(None, None, Some("Apple"), None));
        assert!(ctx.matches(None, None, None, Some("*Safari*")));
    }

    #[test]
    fn test_matches_wildcard() {
        let ctx = WindowContext {
            process_name: "Google Chrome".to_string(),
            window_class: String::new(),
            window_title: "Google Chrome - Wikipedia".to_string(),
            executable_path: Some("/Applications/Google Chrome.app".to_string()),
        };

        // Wildcard match
        assert!(ctx.matches(Some("Google*"), None, None, None));
        assert!(ctx.matches(Some("*Chrome"), None, None, None));
        assert!(ctx.matches(None, None, Some("*Wikipedia*"), None));
        assert!(ctx.matches(None, None, None, Some("*Google*")));

        // Non-match
        assert!(!ctx.matches(Some("Firefox"), None, None, None));
        assert!(!ctx.matches(None, None, Some("Safari"), None));
    }

    #[test]
    fn test_matches_no_conditions() {
        let ctx = WindowContext {
            process_name: "Test".to_string(),
            ..Default::default()
        };

        // No conditions should always match
        assert!(ctx.matches(None, None, None, None));
    }

    #[test]
    fn test_matches_executable_path_none() {
        let ctx = WindowContext {
            process_name: "Test".to_string(),
            executable_path: None,
            ..Default::default()
        };

        // If executable_path is None but pattern is provided, should not match
        assert!(!ctx.matches(None, None, None, Some("/some/path")));
    }

    #[test]
    fn test_get_frontmost_pid() {
        let pid = WindowContext::get_frontmost_pid();
        // On a running system, there should be a frontmost app
        // This may be None in headless CI environments
        let _ = pid;
    }

    #[test]
    fn test_get_process_name_by_pid() {
        // Test with current process's parent or a known PID
        // Use PID 1 (launchd) which should always exist on macOS
        let name = WindowContext::get_process_name_by_pid(1);
        // launchd is usually named "launchd" or similar
        let _ = name;
    }

    #[test]
    fn test_modifier_state() {
        let modifiers = get_modifier_state();
        // Just verify it doesn't panic and returns a valid state
        let _ = modifiers;
    }
}
