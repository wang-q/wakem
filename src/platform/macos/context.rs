//! macOS window context implementation using native APIs
//!
//! Provides information about the currently focused window including
//! process name, window title, and executable path.
//!
//! Performance: < 5ms for get_current() (vs 180ms with AppleScript)
#![cfg(target_os = "macos")]

use crate::platform::macos::native_api::{cg_window, ns_workspace};
use crate::platform::traits::WindowContext as WindowContextTrait;
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

    /// Get current window context using native APIs
    ///
    /// Uses NSWorkspace + CGWindowList + proc_pidpath for maximum performance.
    ///
    /// # Performance
    ///
    /// - NSWorkspace.get_frontmost_app_name(): < 0.5ms
    /// - CGWindowList.get_frontmost_window_info(): < 2ms
    /// - proc_pidpath(): < 1ms
    /// - **Total: < 4ms** (vs ~180ms with AppleScript)
    pub fn get_current() -> Option<WindowContextTrait> {
        // Step 1: Get frontmost app info via NSWorkspace (< 0.5ms)
        let pid = ns_workspace::get_frontmost_app_pid()?;
        let process_name = ns_workspace::get_frontmost_app_name()?;

        // Step 2: Get window info via CGWindowList (< 2ms)
        let window_info = cg_window::get_frontmost_window_info().ok()?;
        let window_title = window_info.map(|info| info.name).unwrap_or_default();

        // Step 3: Get executable path via proc_pidpath (< 1ms)
        let executable_path = ns_workspace::get_app_path(pid);

        debug!(
            "Got window context natively: {} ({}) - '{}'",
            process_name,
            window_title,
            executable_path.as_deref().unwrap_or("unknown")
        );

        Some(WindowContextTrait {
            process_name,
            window_class: String::new(), // Not easily available from native APIs
            window_title,
            executable_path,
        })
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

    /// Check if matches given context conditions with wildcard support.
    ///
    /// Delegates to [WindowContextTrait::matches] for consistent behavior.
    pub fn matches(
        &self,
        process_name: Option<&str>,
        window_class: Option<&str>,
        window_title: Option<&str>,
        executable_path: Option<&str>,
    ) -> bool {
        self.to_platform_context().matches(
            process_name,
            window_class,
            window_title,
            executable_path,
        )
    }
}

/// Get current modifier state using CGEventSource
///
/// Delegates to [crate::platform::macos::get_modifier_state] to avoid duplication.
pub fn get_modifier_state() -> crate::types::ModifierState {
    super::get_modifier_state()
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
    fn test_get_current_native() {
        // This should work without AppleScript now
        match WindowContext::get_current() {
            Some(ctx) => {
                assert!(
                    !ctx.process_name.is_empty(),
                    "Process name should not be empty"
                );
                debug!(
                    "Got current context natively: {} ({})",
                    ctx.process_name, ctx.window_title
                );
            }
            None => {
                eprintln!("Note: No frontmost window or no accessibility permission");
            }
        }
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
    fn test_get_frontmost_pid_native() {
        let pid = ns_workspace::get_frontmost_app_pid();
        // On a running system, there should be a frontmost app
        // This may be None in headless CI environments or if FFI parsing fails
        match pid {
            Some(pid_val) if pid_val > 0 => {
                debug!("Frontmost app PID: {}", pid_val);
            }
            Some(pid_val) => {
                eprintln!("Note: Got invalid PID {} (FFI issue or headless)", pid_val);
            }
            None => {
                eprintln!("No frontmost application found (headless?)");
            }
        }
    }

    #[test]
    fn test_get_process_name_native() {
        let name = ns_workspace::get_frontmost_app_name();
        // Should have a name on a normal system
        if let Some(app_name) = name {
            assert!(!app_name.is_empty(), "App name should not be empty");
            debug!("Frontmost app name: {}", app_name);
        } else {
            eprintln!("No frontmost application name found");
        }
    }

    #[test]
    fn test_get_app_path_for_current_process() {
        use std::process;

        let current_pid = process::id() as u32;
        let path = ns_workspace::get_app_path(current_pid);

        // The path might exist or not depending on how we're run
        if let Some(p) = path {
            debug!("Current process path: {}", p);
        }

        // Just verify it doesn't panic
        let _ = path;
    }

    #[test]
    fn test_modifier_state() {
        let modifiers = get_modifier_state();
        // Just verify it doesn't panic and returns a valid state
        let _ = modifiers;
    }
}
