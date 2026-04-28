//! Platform abstraction layer
//!
//! This module provides cross-platform abstractions for:
//! - Input device capture (keyboard/mouse)
//! - Output device simulation (sending input events)
//! - Window management
//! - System tray integration
//!
//! ## Architecture
//!
//! The module is organized in three layers:
//!
//! 1. **Types & traits** (`types.rs`, `traits.rs`, `macros.rs`) — shared
//!    data types, trait interfaces, and code-generation macros.
//!
//! 2. **Common implementations** (`*_common.rs`) — platform-agnostic
//!    implementations that work across all supported platforms.
//!
//! 3. **Platform modules** (`windows/`, `macos/`, `linux/`) —
//!    platform-specific code selected via conditional compilation.
//!
//! The module uses conditional compilation to select the appropriate
//! platform-specific implementation via the [`CurrentPlatform`] type alias.

// ---------------------------------------------------------------------------
// Layer 1: Types & traits (shared across all platforms)
// ---------------------------------------------------------------------------
pub mod types;
pub mod traits;
pub mod macros;

// ---------------------------------------------------------------------------
// Layer 2: Common implementations (platform-agnostic logic)
// ---------------------------------------------------------------------------
pub mod output_helpers;
pub mod output_device_common;
pub mod input_device_common;
pub mod launcher_common;
pub mod tray_common;
pub mod window_manager_common;
pub mod window_preset_common;

// ---------------------------------------------------------------------------
// Layer 3: Platform-specific modules (selected via conditional compilation)
// ---------------------------------------------------------------------------
#[cfg(test)]
pub mod mock;

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "linux")]
pub mod linux;

// ---------------------------------------------------------------------------
// Current platform type alias
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
pub type CurrentPlatform = windows::WindowsPlatform;

#[cfg(target_os = "macos")]
pub type CurrentPlatform = macos::MacosPlatform;

#[cfg(target_os = "linux")]
pub type CurrentPlatform = linux::LinuxPlatform;
