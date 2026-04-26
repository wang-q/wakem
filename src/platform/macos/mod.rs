//! macOS platform implementation
//!
//! This module provides macOS-specific implementations of the platform traits
//! using Core Graphics, Cocoa, and Accessibility APIs.

pub mod context;
pub mod input;
pub mod input_device;
pub mod native_api;
pub mod output_device;
pub mod tray;
pub mod window_api;
pub mod window_event_hook;
pub mod window_manager;
pub mod window_preset;

// Re-export common types (aligned with Windows platform)
pub use crate::platform::launcher_common::Launcher;
pub use input_device::RawInputDevice;
pub use output_device::SendInputDevice;
pub use tray::{run_tray_message_loop, stop_tray};
pub use window_api::RealWindowApi;
pub use window_event_hook::MacosWindowEventHook as WindowEventHook;
pub use window_manager::{RealWindowManager, WindowManager};
pub use crate::platform::traits::MonitorDirection;
pub use window_preset::WindowPresetManager;

#[cfg(test)]
pub use window_api::MockWindowApi;

/// Get current modifier state for macOS using CGEventSource
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

/// Get process name by PID using libproc
pub fn get_process_name_by_pid(pid: u32) -> anyhow::Result<String> {
    use libc::proc_pidpath;
    use std::ffi::CStr;

    let mut path_buf = [0u8; 4096];
    let path_len = unsafe { proc_pidpath(pid as i32, path_buf.as_mut_ptr() as *mut _, 4096) };

    if path_len <= 0 {
        return Err(anyhow::anyhow!("Failed to get process path for pid {}", pid));
    }

    let path = unsafe { CStr::from_ptr(path_buf.as_ptr() as *const _) }
        .to_string_lossy()
        .to_string();

    // Extract process name from path
    let process_name = path
        .split('/')
        .last()
        .unwrap_or("")
        .to_string();

    if process_name.is_empty() {
        return Err(anyhow::anyhow!("Failed to extract process name from path"));
    }

    Ok(process_name)
}

/// Get executable path by PID using libproc
pub fn get_executable_path_by_pid(pid: u32) -> anyhow::Result<String> {
    use libc::proc_pidpath;
    use std::ffi::CStr;

    let mut path_buf = [0u8; 4096];
    let path_len = unsafe { proc_pidpath(pid as i32, path_buf.as_mut_ptr() as *mut _, 4096) };

    if path_len <= 0 {
        return Err(anyhow::anyhow!("Failed to get executable path for pid {}", pid));
    }

    let path = unsafe { CStr::from_ptr(path_buf.as_ptr() as *const _) }
        .to_string_lossy()
        .to_string();

    Ok(path)
}
