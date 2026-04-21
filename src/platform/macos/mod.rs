//! macOS platform implementation
//!
//! This module provides macOS-specific implementations of the platform traits
//! using Core Graphics, Cocoa, and Accessibility APIs.

pub mod context;
pub mod input_device;
pub mod launcher;
pub mod output_device;
pub mod tray;
pub mod window_api;
pub mod window_manager;

pub use context::WindowContext;
pub use input_device::MacosInputDevice;
pub use launcher::Launcher;
pub use output_device::MacosOutputDevice;
pub use tray::MacosTrayIcon;
pub use window_api::MacosWindowApi;
pub use window_manager::MacosWindowManager;

/// Get current modifier state for macOS
pub fn get_modifier_state() -> crate::types::ModifierState {
    // TODO: Implement using NSEvent.modifierFlags
    // or CGEventSource.flagsState
    crate::types::ModifierState::default()
}
