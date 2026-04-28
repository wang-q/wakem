//! Platform-agnostic implementations shared across all platforms.
//!
//! These modules contain logic that does not depend on any specific
//! operating system API. Platform-specific modules (`windows/`, `macos/`,
//! `linux/`) re-export or wrap the types defined here.

pub mod input_device;
pub mod launcher;
pub mod output_device;
pub mod output_helpers;
pub mod tray;
pub mod window_manager;
pub mod window_preset;
