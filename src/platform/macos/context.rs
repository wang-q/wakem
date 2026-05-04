//! macOS window context implementation using native APIs
//!
//! Provides information about the currently focused window including
//! process name, window title, and executable path.
//!
//! Performance: < 5ms for get_current() (vs 180ms with AppleScript)
#![cfg(target_os = "macos")]

use crate::platform::macos::native_api::{cg_window, ns_workspace};
use crate::platform::traits::WindowContext;
use tracing::debug;

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
pub fn get_current() -> Option<WindowContext> {
    let pid = ns_workspace::get_frontmost_app_pid()?;
    let process_name = ns_workspace::get_frontmost_app_name()?;

    let window_info = cg_window::get_frontmost_window_info().ok()?;
    let window_title = window_info.map(|info| info.name).unwrap_or_default();

    let executable_path = ns_workspace::get_app_path(pid);

    debug!(
        "Got window context natively: {} ({}) - '{}'",
        process_name,
        window_title,
        executable_path.as_deref().unwrap_or("unknown")
    );

    Some(WindowContext {
        process_name,
        window_class: String::new(),
        window_title,
        executable_path,
    })
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
        match get_current() {
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
                debug!("Note: No frontmost window or no accessibility permission");
            }
        }
    }

    #[test]
    fn test_get_frontmost_pid_native() {
        let pid = ns_workspace::get_frontmost_app_pid();
        match pid {
            Some(pid_val) if pid_val > 0 => {
                debug!("Frontmost app PID: {}", pid_val);
            }
            Some(pid_val) => {
                debug!("Note: Got invalid PID {} (FFI issue or headless)", pid_val);
            }
            None => {
                debug!("No frontmost application found (headless?)");
            }
        }
    }

    #[test]
    fn test_get_process_name_native() {
        let name = ns_workspace::get_frontmost_app_name();
        if let Some(app_name) = name {
            assert!(!app_name.is_empty(), "App name should not be empty");
            debug!("Frontmost app name: {}", app_name);
        } else {
            debug!("No frontmost application name found");
        }
    }

    #[test]
    fn test_get_app_path_for_current_process() {
        use std::process;

        let current_pid = process::id() as u32;
        let path = ns_workspace::get_app_path(current_pid);

        if let Some(p) = path {
            debug!("Current process path: {}", p);
        }

        let _ = path;
    }
}
