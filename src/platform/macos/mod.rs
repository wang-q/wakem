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

// Re-export common types
pub use crate::platform::launcher_common::Launcher;
pub use crate::platform::traits::InputDeviceConfig;

// Re-export input device
pub use input_device::MacosInputDeviceExt;
/// InputDevice type alias for macOS
pub type InputDevice = MacosInputDeviceExt;

// Re-export output device
pub use output_device::MacosOutputDevice;

// Re-export tray types
pub use tray::{run_tray_event_loop, AppCommand};

// Re-export window API types
pub use window_api::RealMacosWindowApi;

// Re-export window manager types
pub use window_manager::RealMacosWindowManager as WindowManager;

// Re-export window preset types
pub use window_preset::MacosWindowPresetManager as WindowPresetManager;

#[cfg(test)]
pub use window_api::MockMacosWindowApi;

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
