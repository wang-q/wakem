//! macOS platform implementation
//!
//! This module provides macOS-specific implementations of the platform traits
//! using Core Graphics, Cocoa, and Accessibility APIs.

pub mod context;
pub mod input;
pub mod input_device;
pub mod launcher;
pub mod native_api; // Native API wrappers (Core Graphics + Accessibility)
pub mod output_device;
pub mod tray;
pub mod window_api;
pub mod window_event_hook;
pub mod window_manager;
pub mod window_preset;

// Re-export types for backward compatibility and convenience
pub use context::WindowContext;
pub use input::CGEventTapDevice as MacosEventTap;
pub use input_device::{
    InputDevice, InputDeviceConfig, InputDeviceFactory, MacosInputDevice,
};
pub use launcher::Launcher;
pub use output_device::MacosOutputDevice;
// Re-export tray types and functions
pub use tray::{
    run_tray_event_loop, run_tray_message_loop, stop_tray, AppCommand, MenuAction,
    RealTrayApi, TrayApi, TrayIconWrapper as TrayIcon, TrayManager,
};
pub use window_api::{
    MacosWindowApi, MonitorWorkArea, RealMacosWindowApi, WindowOperation, WindowState,
};
pub use window_event_hook::{MacosWindowEvent, MacosWindowEventHook};
pub use window_manager::{
    MacosWindowFrame, MacosWindowManager, MonitorDirection, RealMacosWindowManager,
};
pub use window_preset::MacosWindowPresetManager;

// Mock implementations are only exported during tests
#[cfg(test)]
pub use input_device::MockInputDevice;
#[cfg(test)]
pub use output_device::MockMacosOutputDevice;
#[cfg(test)]
pub use tray::MockTrayApi;
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
