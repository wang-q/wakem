//! macOS window context implementation

use crate::platform::traits::WindowContext as WindowContextTrait;
use std::process::Command;

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
        // Use AppleScript to get frontmost application info
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
                window_class: String::new(), // macOS doesn't have window classes like Windows
                window_title,
                executable_path,
            })
        } else {
            None
        }
    }

    /// Get frontmost application path using AppleScript
    fn get_frontmost_app_path() -> Option<String> {
        let script = r#"
            tell application "System Events"
                return POSIX path of (path to frontmost application)
            end tell
        "#;

        Command::new("osascript")
            .arg("-e")
            .arg(script)
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
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
}

/// Get current modifier state
pub fn get_modifier_state() -> crate::types::ModifierState {
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    let mut modifiers = crate::types::ModifierState::default();

    // Get current flags from event source
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
    }

    #[test]
    fn test_get_current() {
        // This test may fail if no window is focused
        let _ctx = WindowContext::get_current();
        // Just verify it doesn't panic
    }

    #[test]
    fn test_modifier_state() {
        let modifiers = get_modifier_state();
        // Just verify it doesn't panic
        let _ = modifiers;
    }
}
