//! macOS platform implementation
//!
//! This module provides macOS-specific implementations of the platform traits
//! using Core Graphics, Cocoa, and Accessibility APIs.
#![cfg(target_os = "macos")]

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

pub use crate::platform::launcher_common::Launcher;
pub use input::CGEventTapDevice as MacosEventTap;
pub use input_device::MacosInputDevice;
pub use output_device::MacosOutputDevice;
pub use tray::{
    run_tray_event_loop, run_tray_message_loop, stop_tray, RealTrayApi,
    TrayIconWrapper as TrayIcon, TrayManager,
};
pub use window_api::{MacosWindowApi, RealMacosWindowApi};
pub use window_event_hook::MacosWindowEventHook;
pub use window_manager::{MacosWindowManager, RealMacosWindowManager};
pub use window_preset::MacosWindowPresetManager;

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
